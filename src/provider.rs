use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<ModelMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelOutput {
    Text(String),
    ToolCall {
        name: String,
        arguments: serde_json::Value,
    },
}

#[async_trait]
pub trait LlmProvider: Send + Sync + Clone + 'static {
    async fn complete(&self, request: ModelRequest) -> Result<ModelOutput>;
}

#[derive(Debug, Clone)]
pub struct MockProvider {
    response: String,
}

impl MockProvider {
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _request: ModelRequest) -> Result<ModelOutput> {
        Ok(ModelOutput::Text(self.response.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    name: String,
    base_url: String,
    api_key: String,
    default_model: String,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            default_model: default_model.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new("openai", "https://api.openai.com/v1", api_key, model)
    }

    pub fn dashscope(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(
            "dashscope",
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            api_key,
            model,
        )
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn complete(&self, request: ModelRequest) -> Result<ModelOutput> {
        let model = if request.model.trim().is_empty() {
            self.default_model.clone()
        } else {
            request.model
        };
        let messages: Vec<Value> = request
            .messages
            .into_iter()
            .map(|message| json!({"role": message.role, "content": message.content}))
            .collect();
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": model,
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|err| AgentError::Runtime(format!("{} request failed: {err}", self.name)))?;
        let status = response.status();
        let body = response.text().await.map_err(|err| {
            AgentError::Runtime(format!("{} response body failed: {err}", self.name))
        })?;
        if !status.is_success() {
            return Err(AgentError::Runtime(format!(
                "{} chat completion failed with status {status}: {}",
                self.name,
                redact_sensitive_error(&body)
            )));
        }
        let value: Value = serde_json::from_str(&body)?;
        let message = &value["choices"][0]["message"];
        if let Some(tool_call) = message["tool_calls"]
            .as_array()
            .and_then(|calls| calls.first())
        {
            let name = tool_call["function"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let arguments = tool_call["function"]["arguments"]
                .as_str()
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_else(|| json!({}));
            return Ok(ModelOutput::ToolCall { name, arguments });
        }
        let content = message["content"].as_str().ok_or_else(|| {
            AgentError::Runtime(format!(
                "{} chat completion missing text content: {body}",
                self.name
            ))
        })?;
        Ok(ModelOutput::Text(content.to_string()))
    }
}

fn redact_sensitive_error(body: &str) -> String {
    body.split_whitespace()
        .map(|part| {
            if part.contains("sk-") || part.contains("Bearer") {
                "[redacted]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

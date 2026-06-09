use async_trait::async_trait;
use futures_util::StreamExt;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelStreamEvent {
    ContentDelta {
        delta: String,
    },
    ToolCallDelta {
        name: Option<String>,
        arguments: Option<String>,
    },
}

pub type ModelStreamSink<'a> = dyn FnMut(ModelStreamEvent) -> Result<()> + Send + 'a;

#[async_trait]
pub trait LlmProvider: Send + Sync + Clone + 'static {
    async fn complete(&self, request: ModelRequest) -> Result<ModelOutput>;
    async fn stream(
        &self,
        request: ModelRequest,
        sink: &mut ModelStreamSink<'_>,
    ) -> Result<ModelOutput>;
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

    async fn stream(
        &self,
        _request: ModelRequest,
        sink: &mut ModelStreamSink<'_>,
    ) -> Result<ModelOutput> {
        sink(ModelStreamEvent::ContentDelta {
            delta: self.response.clone(),
        })?;
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

    async fn stream(
        &self,
        request: ModelRequest,
        sink: &mut ModelStreamSink<'_>,
    ) -> Result<ModelOutput> {
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
                "stream": true,
            }))
            .send()
            .await
            .map_err(|err| AgentError::Runtime(format!("{} request failed: {err}", self.name)))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(|err| {
                AgentError::Runtime(format!("{} response body failed: {err}", self.name))
            })?;
            return Err(AgentError::Runtime(format!(
                "{} streaming chat completion failed with status {status}: {}",
                self.name,
                redact_sensitive_error(&body)
            )));
        }

        let mut content = String::new();
        let mut tool_name = String::new();
        let mut tool_arguments = String::new();
        let mut pending = Vec::<u8>::new();
        let mut chunks = response.bytes_stream();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk.map_err(|err| {
                AgentError::Runtime(format!("{} stream chunk failed: {err}", self.name))
            })?;
            pending.extend_from_slice(&chunk);
            while let Some(line_end) = pending.iter().position(|byte| *byte == b'\n') {
                let line = pending.drain(..=line_end).collect::<Vec<_>>();
                let line = String::from_utf8_lossy(&line);
                let line = line.trim();
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                let Some(payload) = line.strip_prefix("data:") else {
                    continue;
                };
                let payload = payload.trim();
                if payload == "[DONE]" {
                    return streamed_output(content, tool_name, tool_arguments);
                }
                consume_stream_payload(
                    payload,
                    &mut content,
                    &mut tool_name,
                    &mut tool_arguments,
                    sink,
                )?;
            }
        }

        if !pending.is_empty() {
            let line = String::from_utf8_lossy(&pending);
            let line = line.trim();
            if let Some(payload) = line.strip_prefix("data:") {
                let payload = payload.trim();
                if payload != "[DONE]" {
                    consume_stream_payload(
                        payload,
                        &mut content,
                        &mut tool_name,
                        &mut tool_arguments,
                        sink,
                    )?;
                }
            }
        }

        streamed_output(content, tool_name, tool_arguments)
    }
}

fn consume_stream_payload(
    payload: &str,
    content: &mut String,
    tool_name: &mut String,
    tool_arguments: &mut String,
    sink: &mut ModelStreamSink<'_>,
) -> Result<()> {
    let value: Value = serde_json::from_str(payload)?;
    for choice in value["choices"].as_array().into_iter().flatten() {
        let delta = &choice["delta"];
        if let Some(text) = delta["content"].as_str() {
            if !text.is_empty() {
                content.push_str(text);
                sink(ModelStreamEvent::ContentDelta {
                    delta: text.to_string(),
                })?;
            }
        }
        for tool_call in delta["tool_calls"].as_array().into_iter().flatten() {
            let function = &tool_call["function"];
            let name = function["name"].as_str();
            let arguments = function["arguments"].as_str();
            if let Some(name) = name {
                tool_name.push_str(name);
            }
            if let Some(arguments) = arguments {
                tool_arguments.push_str(arguments);
            }
            if name.is_some() || arguments.is_some() {
                sink(ModelStreamEvent::ToolCallDelta {
                    name: name.map(ToString::to_string),
                    arguments: arguments.map(ToString::to_string),
                })?;
            }
        }
    }
    Ok(())
}

fn streamed_output(
    content: String,
    tool_name: String,
    tool_arguments: String,
) -> Result<ModelOutput> {
    if !content.is_empty() {
        return Ok(ModelOutput::Text(content));
    }
    if !tool_name.is_empty() {
        return Ok(ModelOutput::ToolCall {
            name: tool_name,
            arguments: serde_json::from_str(&tool_arguments).unwrap_or_else(|_| json!({})),
        });
    }
    Ok(ModelOutput::Text(String::new()))
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

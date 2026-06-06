use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

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

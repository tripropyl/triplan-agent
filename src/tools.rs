use async_trait::async_trait;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::error::{AgentError, Result};
use crate::shadowbox::Shadowbox;

#[async_trait]
pub trait Tool {
    fn name(&self) -> &'static str;
    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value>;
}

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_read requires path".to_string()))?;
        let full_path = shadowbox.resolve_workspace_path(path)?;
        let content = tokio::fs::read_to_string(full_path).await?;
        Ok(json!({ "content": shadowbox.truncate_output(&content) }))
    }
}

pub struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &'static str {
        "search"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("search requires query".to_string()))?;
        let mut matches = Vec::new();
        for entry in WalkDir::new(shadowbox.workspace_root())
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let full_path = entry.path();
            if !entry.file_type().is_file() {
                continue;
            }
            if full_path
                .components()
                .any(|component| component.as_os_str() == ".git")
            {
                continue;
            }
            let Ok(content) = tokio::fs::read_to_string(full_path).await else {
                continue;
            };
            if content.contains(query) {
                let relative_path = full_path
                    .strip_prefix(shadowbox.workspace_root())
                    .unwrap_or(full_path);
                matches.push(json!({
                    "path": relative_path.to_string_lossy(),
                    "snippet": shadowbox.truncate_output(&content),
                }));
            }
        }
        Ok(json!({ "matches": matches }))
    }
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
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

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("bash requires command".to_string()))?;
        let output = timeout(
            Duration::from_secs(30),
            shell_command(command)
                .current_dir(shadowbox.workspace_root())
                .output(),
        )
        .await
        .map_err(|_| AgentError::Runtime("bash command timed out".to_string()))??;
        Ok(json!({
            "exit_code": output.status.code().unwrap_or(-1),
            "stdout": shadowbox.truncate_output(&String::from_utf8_lossy(&output.stdout)),
            "stderr": shadowbox.truncate_output(&String::from_utf8_lossy(&output.stderr)),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct FileState {
    path: PathBuf,
    modified: std::time::SystemTime,
    len: u64,
    fingerprint: u64,
}

impl FileState {
    pub async fn snapshot(path: &Path) -> Result<Self> {
        let metadata = tokio::fs::metadata(path).await?;
        let bytes = tokio::fs::read(path).await?;
        Ok(Self {
            path: path.to_path_buf(),
            modified: metadata.modified()?,
            len: metadata.len(),
            fingerprint: fingerprint(&bytes),
        })
    }

    async fn ensure_matches(&self, path: &Path) -> Result<()> {
        let expected = self.path.canonicalize()?;
        let actual = path.canonicalize()?;
        if expected != actual {
            return Err(AgentError::PolicyDenied(
                "file state does not match edit path".to_string(),
            ));
        }
        Ok(())
    }

    async fn ensure_fresh(&self) -> Result<()> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        let bytes = tokio::fs::read(&self.path).await?;
        if metadata.modified()? != self.modified
            || metadata.len() != self.len
            || fingerprint(&bytes) != self.fingerprint
        {
            return Err(AgentError::PolicyDenied(
                "file changed since read".to_string(),
            ));
        }
        Ok(())
    }
}

pub struct FileEditTool {
    state: FileState,
}

impl FileEditTool {
    pub fn new(state: FileState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &'static str {
        "file_edit"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_edit requires path".to_string()))?;
        let old = input["old"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_edit requires old".to_string()))?;
        let new = input["new"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_edit requires new".to_string()))?;
        let full_path = shadowbox.resolve_workspace_path(path)?;
        self.state.ensure_matches(&full_path).await?;
        self.state.ensure_fresh().await?;

        let content = tokio::fs::read_to_string(&full_path).await?;
        if !content.contains(old) {
            return Err(AgentError::Runtime("old text not found".to_string()));
        }
        tokio::fs::write(&full_path, content.replacen(old, new, 1)).await?;
        Ok(json!({"edited": true}))
    }
}

fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut child = Command::new("cmd");
    child.arg("/C").arg(command);
    child
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut child = Command::new("sh");
    child.arg("-c").arg(command);
    child
}

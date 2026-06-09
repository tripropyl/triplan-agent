use std::path::{Path, PathBuf};

use crate::error::{AgentError, Result};

#[derive(Debug, Clone)]
pub struct Shadowbox {
    workspace_root: PathBuf,
    max_output_bytes: usize,
}

impl Shadowbox {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            max_output_bytes: 65_536,
        }
    }

    pub fn resolve_workspace_path(&self, relative: &str) -> Result<PathBuf> {
        let candidate = self.workspace_root.join(relative);
        self.ensure_inside_workspace(&candidate)?;
        Ok(candidate)
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn ensure_inside_workspace(&self, path: &Path) -> Result<()> {
        let root = self.workspace_root.canonicalize()?;
        let absolute = if path.exists() {
            path.canonicalize()?
        } else {
            path.parent()
                .unwrap_or(&self.workspace_root)
                .canonicalize()?
                .join(path.file_name().unwrap_or_default())
        };
        if !absolute.starts_with(&root) {
            return Err(AgentError::PolicyDenied(format!(
                "path outside workspace: {}",
                path.display()
            )));
        }
        Ok(())
    }

    pub fn truncate_output(&self, text: &str) -> String {
        if text.len() <= self.max_output_bytes {
            return text.to_string();
        }
        let mut end = self.max_output_bytes;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}\n[output truncated]", &text[..end])
    }
}

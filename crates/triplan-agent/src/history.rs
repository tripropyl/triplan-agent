use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::Utc;
use tokio::fs;
use walkdir::WalkDir;

use crate::{config::RuntimePaths, error::Result};

#[derive(Debug, Clone)]
pub struct ConversationHistoryStore {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ConversationHistoryEntry {
    pub path: PathBuf,
    pub modified_at: SystemTime,
}

impl ConversationHistoryStore {
    pub fn new(paths: &RuntimePaths) -> Self {
        Self {
            root: paths.conversation_history_dir(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn conversation_path(&self, workspace_id: &str, conversation_id: &str) -> PathBuf {
        self.root
            .join(safe_path_component(workspace_id))
            .join(format!("{}.md", safe_path_component(conversation_id)))
    }

    pub async fn append_message(
        &self,
        workspace_id: &str,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<PathBuf> {
        let path = self.conversation_path(workspace_id, conversation_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if !path.exists() {
            let header = format!(
                "# Conversation: {conversation_id}\n\n\
                 - Workspace: `{workspace_id}`\n\
                 - Conversation ID: `{conversation_id}`\n\
                 - Created: {}\n\n",
                Utc::now().to_rfc3339()
            );
            fs::write(&path, header).await?;
        }

        let entry = format!(
            "## {}\n\n**{}**\n\n{}\n\n",
            Utc::now().to_rfc3339(),
            display_role(role),
            content.trim()
        );
        let mut existing = fs::read_to_string(&path).await?;
        existing.push_str(&entry);
        fs::write(&path, existing).await?;
        Ok(path)
    }

    pub fn recent_entries(&self, limit: usize) -> Result<Vec<ConversationHistoryEntry>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file()
                || entry.path().extension().and_then(|ext| ext.to_str()) != Some("md")
            {
                continue;
            }
            let metadata = std::fs::metadata(entry.path())?;
            entries.push(ConversationHistoryEntry {
                path: entry.path().to_path_buf(),
                modified_at: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
        entries.sort_by(|a, b| {
            b.modified_at
                .cmp(&a.modified_at)
                .then_with(|| a.path.cmp(&b.path))
        });
        entries.truncate(limit);
        Ok(entries)
    }

    pub async fn recent_context(&self, limit: usize, max_bytes: usize) -> Result<String> {
        let entries = self.recent_entries(limit)?;
        let mut output = String::new();
        output.push_str("# Recent Conversation History\n\n");
        output.push_str(
            "The following Markdown snippets are user-owned conversation history for context injection.\n\n",
        );

        let mut remaining = max_bytes.saturating_sub(output.len());
        for entry in entries {
            if remaining == 0 {
                break;
            }
            let mut content = fs::read_to_string(&entry.path).await?;
            if content.len() > remaining {
                content = tail_to_char_boundary(&content, remaining);
            }
            output.push_str("<!-- source: ");
            output.push_str(&entry.path.display().to_string());
            output.push_str(" -->\n\n");
            output.push_str(&content);
            if !output.ends_with('\n') {
                output.push('\n');
            }
            output.push('\n');
            remaining = max_bytes.saturating_sub(output.len());
        }

        Ok(output)
    }
}

fn safe_path_component(value: &str) -> String {
    let mut safe = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            safe.push(ch);
        } else {
            safe.push('-');
        }
    }
    let safe = safe.trim_matches('-');
    if safe.is_empty() {
        "default".to_string()
    } else {
        safe.chars().take(96).collect()
    }
}

fn display_role(role: &str) -> String {
    match role.trim().to_ascii_lowercase().as_str() {
        "user" => "User".to_string(),
        "assistant" => "Assistant".to_string(),
        "system" => "System".to_string(),
        "tool" => "Tool".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "User".to_string(),
    }
}

fn tail_to_char_boundary(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }
    if max_bytes <= 32 {
        return String::new();
    }

    let marker = "\n\n[...truncated older conversation history...]\n\n";
    let budget = max_bytes.saturating_sub(marker.len());
    let mut start = content.len().saturating_sub(budget);
    while start < content.len() && !content.is_char_boundary(start) {
        start += 1;
    }
    format!("{marker}{}", &content[start..])
}

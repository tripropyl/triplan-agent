use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{AgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace_name: String,
    pub default_agent: String,
    pub default_provider: String,
    pub database_path: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            workspace_name: "agent-workspace".to_string(),
            default_agent: "default".to_string(),
            default_provider: "mock".to_string(),
            database_path: ".agents/agent.db".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfileConfig {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub model: String,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub mcp_servers: Vec<String>,
}

pub fn agents_dir(workspace: &Path) -> PathBuf {
    workspace.join(".agents")
}

pub fn config_path(workspace: &Path) -> PathBuf {
    agents_dir(workspace).join("config.toml")
}

pub async fn init_workspace(workspace: &Path) -> Result<()> {
    let root = agents_dir(workspace);
    fs::create_dir_all(root.join("agents")).await?;
    fs::create_dir_all(root.join("skills")).await?;
    fs::create_dir_all(root.join("prompts/compact")).await?;

    write_toml_if_missing(&root.join("config.toml"), &WorkspaceConfig::default()).await?;
    write_toml_if_missing(
        &root.join("agents/default.toml"),
        &AgentProfileConfig {
            name: "default".to_string(),
            description: "General-purpose local agent.".to_string(),
            system_prompt: "You are a careful local agent working inside this workspace."
                .to_string(),
            model: "mock".to_string(),
            tools: vec![
                "bash".to_string(),
                "file_read".to_string(),
                "file_write".to_string(),
                "file_edit".to_string(),
                "search".to_string(),
                "skill".to_string(),
                "agent_message".to_string(),
                "compact".to_string(),
                "mcp_bridge".to_string(),
            ],
            skills: vec![],
            mcp_servers: vec![],
        },
    )
    .await?;
    write_toml_if_missing(
        &root.join("agents/lead.toml"),
        &AgentProfileConfig {
            name: "lead".to_string(),
            description: "Lead agent that coordinates worker agents.".to_string(),
            system_prompt: "You coordinate work, delegate carefully, and summarize results."
                .to_string(),
            model: "mock".to_string(),
            tools: vec![
                "agent_message".to_string(),
                "compact".to_string(),
                "skill".to_string(),
            ],
            skills: vec![],
            mcp_servers: vec![],
        },
    )
    .await?;
    write_if_missing(
        &root.join("prompts/compact/default.md"),
        DEFAULT_COMPACT_PROMPT,
    )
    .await?;
    write_if_missing(&root.join("mcp.toml"), "[servers]\n").await?;
    write_if_missing(
        &root.join("shadowbox.toml"),
        "workspace_boundary = true\ncommand_timeout_seconds = 30\nmax_output_bytes = 65536\n",
    )
    .await?;
    Ok(())
}

pub async fn load_workspace_config(workspace: &Path) -> Result<WorkspaceConfig> {
    let path = config_path(workspace);
    let content = fs::read_to_string(&path).await.map_err(|_| {
        AgentError::Config(format!("missing workspace config at {}", path.display()))
    })?;
    Ok(toml::from_str(&content)?)
}

async fn write_toml_if_missing<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let content = toml::to_string_pretty(value)?;
    write_if_missing(path, &content).await
}

async fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, content).await?;
    Ok(())
}

const DEFAULT_COMPACT_PROMPT: &str = r#"# Compact Summary Instructions

Summarize the conversation so a future agent can continue the work.

Include:
- primary request and intent
- key technical decisions
- files and code sections
- errors and fixes
- pending tasks
- current work
- next step
"#;

use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{AgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace_name: String,
    pub default_agent: String,
    pub default_provider: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            workspace_name: "triplan-agent".to_string(),
            default_agent: "default".to_string(),
            default_provider: "dashscope".to_string(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRegistryConfig {
    pub providers: BTreeMap<String, ProviderEndpointConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpointConfig {
    pub base_url: String,
    pub api_key_env: String,
}

#[derive(Debug, Clone)]
pub struct RuntimePaths {
    user_root_dir: PathBuf,
    user_agents_dir: PathBuf,
    app_data_dir: PathBuf,
    system_resources_dir: PathBuf,
    checkpoint_database_path: PathBuf,
}

impl RuntimePaths {
    pub fn new(user_root_dir: PathBuf, app_data_dir: PathBuf) -> Self {
        Self {
            user_agents_dir: user_root_dir.join(".agents"),
            system_resources_dir: app_data_dir.join("resources"),
            checkpoint_database_path: app_data_dir.join("checkpoints.sqlite3"),
            user_root_dir,
            app_data_dir,
        }
    }

    pub fn user_root_dir(&self) -> &Path {
        &self.user_root_dir
    }

    pub fn user_agents_dir(&self) -> &Path {
        &self.user_agents_dir
    }

    pub fn user_config_dir(&self) -> &Path {
        &self.user_agents_dir
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    pub fn system_resources_dir(&self) -> &Path {
        &self.system_resources_dir
    }

    pub fn checkpoint_database_path(&self) -> &Path {
        &self.checkpoint_database_path
    }

    pub fn user_config_path(&self) -> PathBuf {
        self.user_agents_dir.join("config.toml")
    }

    pub fn providers_path(&self) -> PathBuf {
        self.user_agents_dir.join("providers.toml")
    }

    pub fn mcp_path(&self) -> PathBuf {
        self.user_agents_dir.join("mcp.toml")
    }

    pub fn shadowbox_path(&self) -> PathBuf {
        self.user_agents_dir.join("shadowbox.toml")
    }

    pub fn agent_profiles_dir(&self) -> PathBuf {
        self.user_agents_dir.join("agents")
    }

    pub fn user_skills_dir(&self) -> PathBuf {
        self.user_agents_dir.join("skills")
    }

    pub fn user_prompts_dir(&self) -> PathBuf {
        self.user_agents_dir.join("prompts")
    }

    pub fn user_compact_prompts_dir(&self) -> PathBuf {
        self.user_prompts_dir().join("compact")
    }

    pub fn conversation_history_dir(&self) -> PathBuf {
        self.user_root_dir.join("conversations")
    }
}

pub fn runtime_paths() -> Result<RuntimePaths> {
    Ok(RuntimePaths::new(user_root_dir()?, app_data_dir()?))
}

pub fn user_root_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("TRIPLAN_AGENT_HOME") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".triplan-agent"))
}

pub fn app_data_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("TRIPLAN_AGENT_APP_DATA") {
        return Ok(PathBuf::from(path));
    }

    if cfg!(windows) {
        env::var_os("APPDATA")
            .or_else(|| env::var_os("LOCALAPPDATA"))
            .map(PathBuf::from)
            .map(|path| path.join("triplan-agent"))
            .ok_or_else(|| AgentError::Config("APPDATA or LOCALAPPDATA is not set".to_string()))
    } else if cfg!(target_os = "macos") {
        Ok(home_dir()?.join("Library/Application Support/triplan-agent"))
    } else {
        Ok(env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or(home_dir()?.join(".local/share"))
            .join("triplan-agent"))
    }
}

pub async fn init_environment() -> Result<()> {
    let paths = runtime_paths()?;
    init_environment_at(&paths).await
}

pub async fn init_environment_at(paths: &RuntimePaths) -> Result<()> {
    init_system_data_at(paths).await?;
    init_user_data_at(paths).await
}

pub async fn init_system_data_at(paths: &RuntimePaths) -> Result<()> {
    fs::create_dir_all(paths.app_data_dir()).await?;
    fs::create_dir_all(paths.system_resources_dir().join("prompts/compact")).await?;
    write_if_missing(
        &paths
            .system_resources_dir()
            .join("prompts/compact/default.md"),
        DEFAULT_COMPACT_PROMPT,
    )
    .await?;
    Ok(())
}

pub async fn init_user_data_at(paths: &RuntimePaths) -> Result<()> {
    fs::create_dir_all(paths.agent_profiles_dir()).await?;
    fs::create_dir_all(paths.user_skills_dir()).await?;
    fs::create_dir_all(paths.user_compact_prompts_dir()).await?;
    fs::create_dir_all(paths.conversation_history_dir()).await?;

    write_toml_if_missing(&paths.user_config_path(), &WorkspaceConfig::default()).await?;
    write_if_missing(&paths.providers_path(), DEFAULT_PROVIDERS).await?;
    write_toml_if_missing(
        &paths.agent_profiles_dir().join("default.toml"),
        &AgentProfileConfig {
            name: "default".to_string(),
            description: "General-purpose local agent.".to_string(),
            system_prompt: "You are a careful local agent working inside this workspace."
                .to_string(),
            model: "deepseek-v4-flash".to_string(),
            tools: vec![
                "bash".to_string(),
                "file_read".to_string(),
                "file_write".to_string(),
                "file_edit".to_string(),
                "search".to_string(),
                "clarify".to_string(),
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
        &paths.agent_profiles_dir().join("lead.toml"),
        &AgentProfileConfig {
            name: "lead".to_string(),
            description: "Lead agent that coordinates worker agents.".to_string(),
            system_prompt: "You coordinate work, delegate carefully, and summarize results."
                .to_string(),
            model: "deepseek-v4-flash".to_string(),
            tools: vec![
                "agent_message".to_string(),
                "compact".to_string(),
                "clarify".to_string(),
                "skill".to_string(),
            ],
            skills: vec![],
            mcp_servers: vec![],
        },
    )
    .await?;
    write_if_missing(
        &paths.user_compact_prompts_dir().join("default.md"),
        USER_COMPACT_PROMPT,
    )
    .await?;
    write_if_missing(&paths.mcp_path(), "[servers]\n").await?;
    write_if_missing(
        &paths.shadowbox_path(),
        "workspace_boundary = true\ncommand_timeout_seconds = 30\nmax_output_bytes = 65536\n",
    )
    .await?;
    Ok(())
}

pub async fn init_workspace(_workspace: &Path) -> Result<()> {
    init_environment().await
}

pub async fn load_user_config() -> Result<WorkspaceConfig> {
    let paths = runtime_paths()?;
    load_user_config_from(&paths).await
}

pub async fn load_user_config_from(paths: &RuntimePaths) -> Result<WorkspaceConfig> {
    let path = paths.user_config_path();
    let content = fs::read_to_string(&path).await.map_err(|_| {
        AgentError::Config(format!(
            "missing user config at {}; run triplan-agent init",
            path.display()
        ))
    })?;
    Ok(toml::from_str(&content)?)
}

pub async fn load_workspace_config(_workspace: &Path) -> Result<WorkspaceConfig> {
    load_user_config().await
}

pub async fn load_agent_profile_from(
    paths: &RuntimePaths,
    agent_name: &str,
) -> Result<AgentProfileConfig> {
    let path = paths
        .agent_profiles_dir()
        .join(format!("{agent_name}.toml"));
    let content = fs::read_to_string(&path).await.map_err(|_| {
        AgentError::Config(format!(
            "missing agent profile at {}; run triplan-agent init",
            path.display()
        ))
    })?;
    Ok(toml::from_str(&content)?)
}

pub async fn load_provider_registry_from(paths: &RuntimePaths) -> Result<ProviderRegistryConfig> {
    let path = paths.providers_path();
    let content = fs::read_to_string(&path).await.map_err(|_| {
        AgentError::Config(format!(
            "missing provider config at {}; run triplan-agent init",
            path.display()
        ))
    })?;
    Ok(toml::from_str(&content)?)
}

pub async fn load_provider_endpoint_from(
    paths: &RuntimePaths,
    provider_name: &str,
) -> Result<ProviderEndpointConfig> {
    let registry = load_provider_registry_from(paths).await?;
    registry
        .providers
        .get(provider_name)
        .cloned()
        .ok_or_else(|| AgentError::Config(format!("unknown provider `{provider_name}`")))
}

pub async fn checkpoint_database_url() -> Result<String> {
    let paths = runtime_paths()?;
    checkpoint_database_url_for(&paths).await
}

pub async fn checkpoint_database_url_for(paths: &RuntimePaths) -> Result<String> {
    if let Some(parent) = paths.checkpoint_database_path().parent() {
        fs::create_dir_all(parent).await?;
    }
    Ok(format!(
        "sqlite://{}?mode=rwc",
        paths.checkpoint_database_path().to_string_lossy()
    ))
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

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(
            || match (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH")) {
                (Some(drive), Some(path)) => {
                    let mut home = PathBuf::from(drive);
                    home.push(path);
                    Some(home)
                }
                _ => None,
            },
        )
        .ok_or_else(|| AgentError::Config("could not resolve the user home directory".to_string()))
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

const USER_COMPACT_PROMPT: &str = r#"# User Compact Summary Overrides

Add local compaction preferences here. The built-in default lives in APP_DATA resources.
"#;

const DEFAULT_PROVIDERS: &str = r#"[providers.dashscope]
base_url = "https://bailian.bangdao-tech.com/compatible-mode/v1"
api_key_env = "DASHSCOPE_API_KEY"

[providers.openai]
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
"#;

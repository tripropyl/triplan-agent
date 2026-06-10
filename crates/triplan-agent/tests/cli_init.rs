use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

struct IsolatedPaths {
    user_root: PathBuf,
    user_agents: PathBuf,
    user_config: PathBuf,
    providers: PathBuf,
    agent_profiles: PathBuf,
    prompts: PathBuf,
    mcp: PathBuf,
    shadowbox: PathBuf,
    app_data: PathBuf,
}

fn isolated_paths(temp: &tempfile::TempDir) -> IsolatedPaths {
    let user_root = temp.path().join("home/.triplan-agent");
    let user_agents = user_root.join(".agents");
    let app_data = temp.path().join("app-data");
    IsolatedPaths {
        user_config: user_root.join("config.toml"),
        providers: user_root.join("providers.toml"),
        agent_profiles: user_root.join("agents"),
        prompts: user_root.join("prompts"),
        mcp: user_root.join("mcp.toml"),
        shadowbox: user_root.join("shadowbox.toml"),
        user_root,
        user_agents,
        app_data,
    }
}

fn isolated_cmd(paths: &IsolatedPaths) -> Command {
    let mut cmd = Command::cargo_bin("triplan-agent").expect("triplan-agent binary exists");
    cmd.env("TRIPLAN_AGENT_HOME", &paths.user_root)
        .env("TRIPLAN_AGENT_APP_DATA", &paths.app_data);
    cmd
}

#[test]
fn cli_prints_version() {
    let mut cmd = Command::cargo_bin("triplan-agent").expect("triplan-agent binary exists");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("triplan-agent "));
}

#[test]
fn short_binary_prints_version() {
    let mut cmd = Command::cargo_bin("triplan").expect("triplan binary exists");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("triplan "));
}

#[test]
fn init_creates_workspace_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);
    let mut cmd = isolated_cmd(&paths);
    cmd.current_dir(temp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "initialized triplan-agent environment",
        ));

    assert!(!temp.path().join(".triplan-agent").exists());
    assert!(paths.user_config.is_file());
    assert!(!paths.user_agents.join("config.toml").exists());
    assert!(paths.providers.is_file());
    assert!(!paths.user_agents.join("providers.toml").exists());
    assert!(paths.agent_profiles.join("lead.toml").is_file());
    assert!(paths.agent_profiles.join("default.toml").is_file());
    assert!(!paths.user_agents.join("agents").exists());
    assert!(paths.prompts.join("compact/default.md").is_file());
    assert!(!paths.user_agents.join("prompts").exists());
    assert!(paths.user_agents.join("skills").is_dir());
    assert!(paths.mcp.is_file());
    assert!(!paths.user_agents.join("mcp.toml").exists());
    assert!(paths.shadowbox.is_file());
    assert!(!paths.user_agents.join("shadowbox.toml").exists());
    assert!(paths.user_root.join("conversations").is_dir());
    assert!(!paths.app_data.join("config.toml").exists());
    assert!(!paths.app_data.join("providers.toml").exists());
    assert!(!paths.app_data.join("mcp.toml").exists());
    assert!(!paths.app_data.join("shadowbox.toml").exists());
    assert!(!paths.app_data.join("agents").exists());
    assert!(!paths.app_data.join("skills").exists());
    assert!(paths.app_data.join("checkpoints.sqlite3").is_file());
    assert!(paths
        .app_data
        .join("resources/prompts/compact/default.md")
        .is_file());
}

#[test]
fn history_add_and_recent_use_user_conversation_markdown() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);

    isolated_cmd(&paths)
        .args([
            "history",
            "add",
            "--workspace",
            "workspace-1",
            "--conversation",
            "conversation-1",
            "Need",
            "recent",
            "context",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("history:"));

    let history_path = paths
        .user_root
        .join("conversations/workspace-1/conversation-1.md");
    assert!(history_path.is_file());

    isolated_cmd(&paths)
        .args(["history", "recent", "--limit", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Recent Conversation History"))
        .stdout(predicate::str::contains("Need recent context"));
}

#[test]
fn run_records_user_prompt_in_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);

    isolated_cmd(&paths)
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    isolated_cmd(&paths)
        .current_dir(temp.path())
        .args([
            "run",
            "--conversation",
            "build-plan",
            "Use",
            "recent",
            "history",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("history:"));

    let content = std::fs::read_to_string(
        paths
            .user_root
            .join("conversations/triplan-agent/build-plan.md"),
    )
    .expect("history content");
    assert!(content.contains("Use recent history"));
}

#[test]
fn print_shortcut_records_user_prompt_in_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);

    isolated_cmd(&paths)
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    isolated_cmd(&paths)
        .current_dir(temp.path())
        .args(["-p", "Use quick print"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent run requested"));

    let content = std::fs::read_to_string(
        paths
            .user_root
            .join("conversations/triplan-agent/default.md"),
    )
    .expect("history content");
    assert!(content.contains("Use quick print"));
}

#[test]
fn workspace_shortcut_lazily_initializes_and_prompts_for_api_key() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");

    let mut cmd = Command::cargo_bin("triplan").expect("triplan binary exists");
    cmd.current_dir(temp.path())
        .env("TRIPLAN_AGENT_HOME", &paths.user_root)
        .env("TRIPLAN_AGENT_APP_DATA", &paths.app_data)
        .arg(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "initialized triplan-agent environment",
        ))
        .stdout(predicate::str::contains("missing API key"))
        .stdout(predicate::str::contains("DASHSCOPE_API_KEY"));

    assert!(paths.user_config.is_file());
    assert!(paths.providers.is_file());
    assert!(paths
        .user_root
        .join("conversations/triplan-agent/default.md")
        .is_file());
}

#[test]
fn provider_api_key_env_secret_value_is_reported_without_leaking_secret() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");

    isolated_cmd(&paths)
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    std::fs::write(
        &paths.providers,
        r#"[providers.dashscope]
base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1"
api_key_env = "sk-test"
"#,
    )
    .expect("providers");

    let mut cmd = Command::cargo_bin("triplan").expect("triplan binary exists");
    cmd.current_dir(temp.path())
        .env("TRIPLAN_AGENT_HOME", &paths.user_root)
        .env("TRIPLAN_AGENT_APP_DATA", &paths.app_data)
        .arg(&workspace)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "`api_key_env` should be an environment variable name",
        ))
        .stdout(predicate::str::contains("DASHSCOPE_API_KEY"))
        .stdout(predicate::str::contains("sk-test").not());
}

#[test]
fn init_defaults_to_dashscope_deepseek_flash() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);
    isolated_cmd(&paths)
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    let config = std::fs::read_to_string(paths.user_config).expect("user config");
    let providers = std::fs::read_to_string(paths.providers).expect("providers");
    let default_agent =
        std::fs::read_to_string(paths.agent_profiles.join("default.toml")).expect("default agent");

    assert!(config.contains("default_provider = \"dashscope\""));
    assert!(providers.contains("[providers.dashscope]"));
    assert!(providers.contains("base_url = \"https://dashscope.aliyuncs.com/compatible-mode/v1\""));
    assert!(default_agent.contains("model = \"deepseek-v4-flash\""));
}

#[test]
fn doctor_reports_basic_checks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("triplan-agent").expect("triplan-agent binary exists");
    cmd.current_dir(temp.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("CLI: ok"))
        .stdout(predicate::str::contains(
            "User config: stored under ~/.triplan-agent by default",
        ))
        .stdout(predicate::str::contains(
            "Agent assets: only standard assets such as skills live under ~/.triplan-agent/.agents",
        ));
}

#[test]
fn status_reports_workspace_after_init() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);
    isolated_cmd(&paths)
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    isolated_cmd(&paths)
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("workspace: triplan-agent"))
        .stdout(predicate::str::contains("user_data:"))
        .stdout(predicate::str::contains("user_config:"))
        .stdout(predicate::str::contains("provider_config:"))
        .stdout(predicate::str::contains("agent_assets:"))
        .stdout(predicate::str::contains("conversation_history:"))
        .stdout(predicate::str::contains("app_data:"))
        .stdout(predicate::str::contains("checkpoint_db:"));
}

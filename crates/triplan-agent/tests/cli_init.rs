use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

struct IsolatedPaths {
    user_root: PathBuf,
    user_agents: PathBuf,
    app_data: PathBuf,
}

fn isolated_paths(temp: &tempfile::TempDir) -> IsolatedPaths {
    let user_root = temp.path().join("home/.triplan-agent");
    let user_agents = user_root.join(".agents");
    let app_data = temp.path().join("app-data");
    IsolatedPaths {
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
    assert!(paths.user_agents.join("config.toml").is_file());
    assert!(paths.user_agents.join("providers.toml").is_file());
    assert!(paths.user_agents.join("agents/lead.toml").is_file());
    assert!(paths.user_agents.join("agents/default.toml").is_file());
    assert!(paths
        .user_agents
        .join("prompts/compact/default.md")
        .is_file());
    assert!(paths.user_agents.join("mcp.toml").is_file());
    assert!(paths.user_agents.join("shadowbox.toml").is_file());
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
fn init_defaults_to_dashscope_deepseek_flash() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = isolated_paths(&temp);
    isolated_cmd(&paths)
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    let config =
        std::fs::read_to_string(paths.user_agents.join("config.toml")).expect("user config");
    let providers =
        std::fs::read_to_string(paths.user_agents.join("providers.toml")).expect("providers");
    let default_agent = std::fs::read_to_string(paths.user_agents.join("agents/default.toml"))
        .expect("default agent");

    assert!(config.contains("default_provider = \"dashscope\""));
    assert!(providers.contains("[providers.dashscope]"));
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
            "User config: stored under ~/.triplan-agent/.agents by default",
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
        .stdout(predicate::str::contains("conversation_history:"))
        .stdout(predicate::str::contains("app_data:"))
        .stdout(predicate::str::contains("checkpoint_db:"));
}

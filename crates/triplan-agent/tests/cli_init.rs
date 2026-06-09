use assert_cmd::Command;
use predicates::prelude::*;

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
    let mut cmd = Command::cargo_bin("triplan-agent").expect("triplan-agent binary exists");
    cmd.current_dir(temp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "initialized triplan-agent workspace",
        ));

    assert!(temp.path().join(".triplan-agent/config.toml").is_file());
    assert!(temp
        .path()
        .join(".triplan-agent/agents/lead.toml")
        .is_file());
    assert!(temp
        .path()
        .join(".triplan-agent/agents/default.toml")
        .is_file());
    assert!(temp
        .path()
        .join(".triplan-agent/prompts/compact/default.md")
        .is_file());
    assert!(temp.path().join(".triplan-agent/mcp.toml").is_file());
    assert!(temp.path().join(".triplan-agent/shadowbox.toml").is_file());
}

#[test]
fn init_defaults_to_dashscope_deepseek_flash() {
    let temp = tempfile::tempdir().expect("tempdir");
    Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join(".triplan-agent/config.toml"))
        .expect("workspace config");
    let default_agent =
        std::fs::read_to_string(temp.path().join(".triplan-agent/agents/default.toml"))
            .expect("default agent");

    assert!(config.contains("default_provider = \"dashscope\""));
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
        .stdout(predicate::str::contains("CLI: ok"));
}

#[test]
fn status_reports_workspace_after_init() {
    let temp = tempfile::tempdir().expect("tempdir");
    Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("workspace: triplan-agent"));
}

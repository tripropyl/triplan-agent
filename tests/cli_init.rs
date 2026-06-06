use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_prints_version() {
    let mut cmd = Command::cargo_bin("agent").expect("agent binary exists");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("agent "));
}

#[test]
fn init_creates_workspace_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("agent").expect("agent binary exists");
    cmd.current_dir(temp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized agent workspace"));

    assert!(temp.path().join(".agents/config.toml").is_file());
    assert!(temp.path().join(".agents/agents/lead.toml").is_file());
    assert!(temp.path().join(".agents/agents/default.toml").is_file());
    assert!(temp
        .path()
        .join(".agents/prompts/compact/default.md")
        .is_file());
    assert!(temp.path().join(".agents/mcp.toml").is_file());
    assert!(temp.path().join(".agents/shadowbox.toml").is_file());
}

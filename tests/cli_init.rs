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

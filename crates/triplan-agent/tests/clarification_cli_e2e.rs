use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_clarification_request_list_answer_round_trip() {
    let temp = tempfile::tempdir().expect("tempdir");

    Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();

    let request = Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .args([
            "clarify",
            "request",
            "--conversation",
            "conversation-1",
            "--run",
            "run-1",
            "--agent",
            "default",
            "--question",
            "Which target should I build first?",
            "--reason",
            "The task needs a target before it can continue.",
            "--option",
            "cli",
            "--option",
            "sdk",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: pending"))
        .stdout(predicate::str::contains("run_status: paused"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(request).expect("utf8 stdout");
    let clarification_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("clarification_id: "))
        .expect("clarification id")
        .to_string();

    Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .args(["clarify", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&clarification_id))
        .stdout(predicate::str::contains(
            "Which target should I build first?",
        ));

    Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .args(["clarify", "answer", &clarification_id, "--answer", "cli"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: answered"))
        .stdout(predicate::str::contains("run_status: resumed"));

    Command::cargo_bin("triplan-agent")
        .expect("triplan-agent binary exists")
        .current_dir(temp.path())
        .args(["clarify", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no pending clarifications"));
}

use assert_cmd::Command;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn isolated_cmd(user_root: &std::path::Path, app_data: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("triplan-agent").expect("triplan-agent binary exists");
    cmd.env("TRIPLAN_AGENT_HOME", user_root)
        .env("TRIPLAN_AGENT_APP_DATA", app_data);
    cmd
}

#[tokio::test]
async fn run_stream_json_outputs_deltas_and_records_assistant_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_root = temp.path().join("home/.triplan-agent");
    let app_data = temp.path().join("app-data");

    isolated_cmd(&user_root, &app_data)
        .arg("init")
        .assert()
        .success();

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("local addr");
    let providers = format!(
        "[providers.local]\nbase_url = \"http://{address}/v1\"\napi_key_env = \"LOCAL_API_KEY\"\n"
    );
    std::fs::write(user_root.join(".agents/providers.toml"), providers).expect("providers");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buffer = [0_u8; 8192];
        let _ = socket.read(&mut buffer).await.expect("read request");
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" there\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    });

    let command_user_root = user_root.clone();
    let command_app_data = app_data.clone();
    let output = tokio::task::spawn_blocking(move || {
        isolated_cmd(&command_user_root, &command_app_data)
            .env("LOCAL_API_KEY", "test-key")
            .args([
                "run",
                "--provider",
                "local",
                "--output-format",
                "stream-json",
                "hello",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    })
    .await
    .expect("command task");
    server.await.expect("server task");

    let stdout = String::from_utf8(output).expect("stdout");
    assert!(stdout.contains("\"type\":\"run_started\""));
    assert!(stdout.contains("\"type\":\"content_delta\""));
    assert!(stdout.contains("\"delta\":\"hi\""));
    assert!(stdout.contains("\"delta\":\" there\""));
    assert!(stdout.contains("\"type\":\"run_completed\""));

    let history = std::fs::read_to_string(user_root.join("conversations/triplan-agent/default.md"))
        .expect("history");
    assert!(history.contains("hello"));
    assert!(history.contains("**Assistant**"));
    assert!(history.contains("hi there"));
}

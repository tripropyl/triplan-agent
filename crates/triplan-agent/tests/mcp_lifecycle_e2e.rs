use serde_json::json;
use triplan_agent::mcp::{McpRequest, StdioMcpClient};

#[tokio::test]
async fn stdio_mcp_client_round_trips_json_rpc_and_stops_child() {
    let temp = tempfile::tempdir().expect("tempdir");
    let server = temp.path().join("mcp_echo.py");
    tokio::fs::write(
        &server,
        r#"
import json
import sys

line = sys.stdin.readline()
request = json.loads(line)
response = {
  "jsonrpc": "2.0",
  "id": request["id"],
  "result": {"method": request["method"], "ok": True},
}
sys.stdout.write(json.dumps(response) + "\n")
sys.stdout.flush()
"#,
    )
    .await
    .expect("write server");

    let mut client = StdioMcpClient::spawn("python3", &[server.to_string_lossy().to_string()])
        .await
        .expect("spawn client");
    let response = client
        .request(McpRequest::new(7, "tools/list", json!({})))
        .await
        .expect("request");
    client.shutdown().await.expect("shutdown");

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 7);
    assert_eq!(response["result"]["method"], "tools/list");
    assert_eq!(response["result"]["ok"], true);
}

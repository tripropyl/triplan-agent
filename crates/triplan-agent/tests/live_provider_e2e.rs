use triplan_agent::provider::{
    LlmProvider, ModelMessage, ModelOutput, ModelRequest, ModelStreamEvent,
    OpenAiCompatibleProvider,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn load_local_env() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join("../../.env"));
    let _ = dotenvy::dotenv();
}

#[tokio::test]
async fn openai_compatible_provider_streams_sse_content() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buffer = [0_u8; 4096];
        let _ = socket.read(&mut buffer).await.expect("read request");
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
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

    let provider = OpenAiCompatibleProvider::new(
        "local",
        format!("http://{address}/v1"),
        "test-key",
        "test-model",
    );
    let mut deltas = Vec::new();
    let mut sink = |event: ModelStreamEvent| {
        if let ModelStreamEvent::ContentDelta { delta } = event {
            deltas.push(delta);
        }
        Ok(())
    };
    let output = provider
        .stream(
            ModelRequest {
                model: "test-model".to_string(),
                messages: vec![ModelMessage {
                    role: "user".to_string(),
                    content: "hello".to_string(),
                }],
            },
            &mut sink,
        )
        .await
        .expect("stream completion");
    server.await.expect("server task");

    assert_eq!(deltas, vec!["hel", "lo"]);
    match output {
        ModelOutput::Text(text) => assert_eq!(text, "hello"),
        other => panic!("expected text output, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY and spends real OpenAI API tokens"]
async fn openai_live_chat_completion_returns_text() {
    load_local_env();
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY");
    let model = std::env::var("OPENAI_LIVE_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let provider = OpenAiCompatibleProvider::new(
        "openai",
        "https://api.openai.com/v1",
        api_key,
        model.clone(),
    );
    let output = provider
        .complete(ModelRequest {
            model,
            messages: vec![ModelMessage {
                role: "user".to_string(),
                content: "Return exactly: triplan-agent-openai-live-ok".to_string(),
            }],
        })
        .await
        .expect("openai completion");

    match output {
        ModelOutput::Text(text) => assert!(text.contains("triplan-agent-openai-live-ok")),
        other => panic!("expected text output, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires DASHSCOPE_API_KEY and spends real DashScope API tokens"]
async fn dashscope_live_chat_completion_returns_text() {
    load_local_env();
    let api_key = std::env::var("DASHSCOPE_API_KEY").expect("DASHSCOPE_API_KEY");
    let base_url = std::env::var("DASHSCOPE_BASE_URL")
        .unwrap_or_else(|_| "https://bailian.bangdao-tech.com/compatible-mode/v1".to_string());
    let model =
        std::env::var("DASHSCOPE_LIVE_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
    let provider = OpenAiCompatibleProvider::new("dashscope", base_url, api_key, model.clone());
    let output = provider
        .complete(ModelRequest {
            model,
            messages: vec![ModelMessage {
                role: "user".to_string(),
                content: "Return exactly: triplan-agent-dashscope-live-ok".to_string(),
            }],
        })
        .await
        .expect("dashscope completion");

    match output {
        ModelOutput::Text(text) => assert!(text.contains("triplan-agent-dashscope-live-ok")),
        other => panic!("expected text output, got {other:?}"),
    }
}

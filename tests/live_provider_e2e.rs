use agent_ease::provider::{
    LlmProvider, ModelMessage, ModelOutput, ModelRequest, OpenAiCompatibleProvider,
};

fn load_local_env() {
    let _ = dotenvy::dotenv();
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
                content: "Return exactly: agent-ease-openai-live-ok".to_string(),
            }],
        })
        .await
        .expect("openai completion");

    match output {
        ModelOutput::Text(text) => assert!(text.contains("agent-ease-openai-live-ok")),
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
                content: "Return exactly: agent-ease-dashscope-live-ok".to_string(),
            }],
        })
        .await
        .expect("dashscope completion");

    match output {
        ModelOutput::Text(text) => assert!(text.contains("agent-ease-dashscope-live-ok")),
        other => panic!("expected text output, got {other:?}"),
    }
}

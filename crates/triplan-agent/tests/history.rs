use triplan_agent::{config::RuntimePaths, history::ConversationHistoryStore};

#[tokio::test]
async fn conversation_history_appends_markdown_under_user_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = RuntimePaths::new(
        temp.path().join("home/.triplan-agent"),
        temp.path().join("app-data"),
    );
    let store = ConversationHistoryStore::new(&paths);

    let path = store
        .append_message(
            "workspace-1",
            "conversation/one",
            "user",
            "Build the CLI first.",
        )
        .await
        .expect("append user message");
    store
        .append_message(
            "workspace-1",
            "conversation/one",
            "assistant",
            "I will start there.",
        )
        .await
        .expect("append assistant message");

    assert!(path.starts_with(paths.conversation_history_dir()));
    assert_eq!(
        path,
        paths
            .conversation_history_dir()
            .join("workspace-1/conversation-one.md")
    );

    let content = tokio::fs::read_to_string(&path).await.expect("history md");
    assert!(content.contains("# Conversation: conversation/one"));
    assert!(content.contains("**User**"));
    assert!(content.contains("Build the CLI first."));
    assert!(content.contains("**Assistant**"));
    assert!(content.contains("I will start there."));
}

#[tokio::test]
async fn recent_context_returns_markdown_for_context_injection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = RuntimePaths::new(
        temp.path().join("home/.triplan-agent"),
        temp.path().join("app-data"),
    );
    let store = ConversationHistoryStore::new(&paths);
    store
        .append_message(
            "workspace-1",
            "conversation-1",
            "user",
            "Remember the auth issue.",
        )
        .await
        .expect("append");

    let context = store.recent_context(3, 4096).await.expect("recent context");

    assert!(context.contains("# Recent Conversation History"));
    assert!(context.contains("<!-- source:"));
    assert!(context.contains("Remember the auth issue."));
}

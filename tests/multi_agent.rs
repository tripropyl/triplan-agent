use agent_ease::context::{AgentBus, ContextPatchStore};
use agent_ease::db::{connect_sqlite, migrate};

#[tokio::test]
async fn agent_message_becomes_context_patch() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let bus = AgentBus::new(pool.clone());
    let patches = ContextPatchStore::new(pool);

    bus.send_message(
        "workspace-1",
        "conversation-1",
        "lead",
        "worker",
        "prefer config first",
    )
    .await
    .expect("send");
    let pending = patches
        .pending_for_agent("workspace-1", "conversation-1", "worker")
        .await
        .expect("pending");

    assert_eq!(pending[0].kind, "agent_message");
    assert!(pending[0].content.contains("prefer config first"));
}

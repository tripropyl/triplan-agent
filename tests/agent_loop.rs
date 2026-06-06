use agent_ease::context::CompactionEngine;
use agent_ease::db::{connect_sqlite, migrate, EventStore};
use agent_ease::provider::MockProvider;
use agent_ease::runtime::AgentLoop;

#[tokio::test]
async fn mock_agent_loop_records_assistant_message() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let store = EventStore::new(pool);
    let provider = MockProvider::new("hello from mock");
    let loop_worker = AgentLoop::new(store.clone(), provider);

    loop_worker
        .run_once(
            "workspace-1",
            "conversation-1",
            "run-1",
            "default",
            "say hello",
        )
        .await
        .expect("run once");

    let events = store.list_events("workspace-1").await.expect("events");
    assert!(events
        .iter()
        .any(|event| event.payload.to_string().contains("hello from mock")));
}

#[tokio::test]
async fn compaction_preserves_current_work_sections() {
    let summary = CompactionEngine::summarize_for_test(&[
        "User asked to build runtime",
        "Agent inspected SQLite design",
        "Pending task: implement CLI",
    ]);
    assert!(summary.contains("Primary Request and Intent"));
    assert!(summary.contains("Pending Tasks"));
    assert!(summary.contains("implement CLI"));
}

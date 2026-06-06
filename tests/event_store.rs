use agent_ease::db::{connect_sqlite, migrate, EventStore};
use agent_ease::model::{EventPayload, EventType};
use serde_json::json;

#[tokio::test]
async fn appends_events_with_monotonic_sequence() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let store = EventStore::new(pool);

    let first = store
        .append_event(
            "workspace-1",
            Some("conversation-1"),
            Some("run-1"),
            Some("default"),
            EventType::RunRequested,
            EventPayload::Json(json!({"prompt":"hello"})),
            None,
            None,
            None,
        )
        .await
        .expect("first event");
    let second = store
        .append_event(
            "workspace-1",
            Some("conversation-1"),
            Some("run-1"),
            Some("default"),
            EventType::RunStarted,
            EventPayload::Json(json!({})),
            Some(first.event_id),
            Some(first.event_id),
            Some(first.correlation_id),
        )
        .await
        .expect("second event");

    assert_eq!(first.sequence + 1, second.sequence);
}

use agent_ease::db::{connect_sqlite, migrate, EventStore};
use agent_ease::db::{CheckpointStore, TaskStore};
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

#[tokio::test]
async fn checkpoint_round_trips_projection() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let checkpoints = CheckpointStore::new(pool);

    checkpoints
        .save_checkpoint("workspace-1", "run-1", 7, json!({"messages":["hello"]}))
        .await
        .expect("save checkpoint");

    let loaded = checkpoints
        .latest_checkpoint("workspace-1", "run-1")
        .await
        .expect("load checkpoint")
        .expect("checkpoint exists");

    assert_eq!(loaded.after_sequence, 7);
    assert_eq!(loaded.projection["messages"][0], "hello");
}

#[tokio::test]
async fn task_lease_is_exclusive() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let tasks = TaskStore::new(pool);
    let task_id = tasks
        .enqueue(
            "agent_loop_step",
            Some("run-1"),
            10,
            json!({"run_id":"run-1"}),
        )
        .await
        .expect("enqueue");

    let first = tasks.lease_next("worker-a", 30).await.expect("first lease");
    let second = tasks
        .lease_next("worker-b", 30)
        .await
        .expect("second lease");

    assert_eq!(first.expect("leased task").task_id, task_id);
    assert!(second.is_none());
}

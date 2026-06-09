use serde_json::json;
use triplan_agent::db::{connect_sqlite, migrate, ClarificationStore, EventStore};
use triplan_agent::model::EventType;
use triplan_agent::tools::{ClarifyTool, ControlTool, ControlToolContext};

#[tokio::test]
async fn clarify_tool_persists_pause_and_resume_events() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let clarifications = ClarificationStore::new(pool.clone());
    let events = EventStore::new(pool.clone());
    let tool = ClarifyTool::new(clarifications.clone());

    let output = tool
        .call(
            ControlToolContext {
                workspace_id: "workspace-1",
                conversation_id: "conversation-1",
                run_id: "run-1",
                agent_id: "default",
            },
            json!({
                "question": "Which target should I build first?",
                "reason": "The task can continue only after choosing a target.",
                "options": ["cli", "sdk"]
            }),
        )
        .await
        .expect("clarify request");

    assert_eq!(output["tool"], "clarify");
    assert_eq!(output["status"], "pending");
    assert_eq!(output["run_status"], "paused");
    let clarification_id = output["clarification_id"]
        .as_str()
        .expect("clarification id");

    let pending = clarifications
        .list_pending("workspace-1", Some("run-1"))
        .await
        .expect("pending clarifications");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].question, "Which target should I build first?");
    assert_eq!(pending[0].options, vec!["cli", "sdk"]);

    let recorded = events.list_events("workspace-1").await.expect("events");
    assert!(recorded
        .iter()
        .any(|event| event.event_type == EventType::ClarificationRequested));
    assert!(recorded
        .iter()
        .any(|event| event.event_type == EventType::RunPaused));

    let answered = clarifications
        .answer(clarification_id, "cli")
        .await
        .expect("answer clarification");
    assert_eq!(answered.status, "answered");
    assert_eq!(answered.answer.as_deref(), Some("cli"));

    let recorded = events.list_events("workspace-1").await.expect("events");
    assert!(recorded
        .iter()
        .any(|event| event.event_type == EventType::ClarificationAnswered));
    assert!(recorded
        .iter()
        .any(|event| event.event_type == EventType::RunResumed));
}

#[tokio::test]
async fn clarify_allows_only_one_pending_question_per_run() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let clarifications = ClarificationStore::new(pool);
    let tool = ClarifyTool::new(clarifications);
    let context = ControlToolContext {
        workspace_id: "workspace-1",
        conversation_id: "conversation-1",
        run_id: "run-1",
        agent_id: "default",
    };

    tool.call(
        context,
        json!({
            "question": "First question?",
            "options": ["a", "b"]
        }),
    )
    .await
    .expect("first clarification");

    let err = tool
        .call(
            context,
            json!({
                "question": "Second question?",
                "options": ["c", "d"]
            }),
        )
        .await
        .expect_err("second pending clarification rejected");

    assert!(err
        .to_string()
        .contains("pending clarification already exists"));
}

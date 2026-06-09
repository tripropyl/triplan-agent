use serde_json::json;
use triplan_agent::config::{
    checkpoint_database_url_for, init_environment_at, load_user_config_from, RuntimePaths,
};
use triplan_agent::context::{AgentBus, CompactionEngine, ContextPatchStore};
use triplan_agent::db::{connect_sqlite, migrate, CheckpointStore, EventStore, TaskStore};
use triplan_agent::mcp::McpRequest;
use triplan_agent::provider::MockProvider;
use triplan_agent::runtime::AgentLoop;
use triplan_agent::shadowbox::Shadowbox;
use triplan_agent::skills::SkillRegistry;
use triplan_agent::tools::{BashTool, FileEditTool, FileReadTool, FileState, SearchTool, Tool};

#[tokio::test]
async fn foundation_runtime_e2e_flow() {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = RuntimePaths::new(
        temp.path().join("home/.triplan-agent"),
        temp.path().join("app-data"),
    );
    init_environment_at(&paths).await.expect("init environment");
    let config = load_user_config_from(&paths).await.expect("load config");
    assert_eq!(config.workspace_name, "triplan-agent");

    let db_url = checkpoint_database_url_for(&paths)
        .await
        .expect("checkpoint database url");
    let pool = connect_sqlite(&db_url).await.expect("pool");
    migrate(&pool).await.expect("migrate");

    let events = EventStore::new(pool.clone());
    let agent_loop = AgentLoop::new(events.clone(), MockProvider::new("e2e assistant response"));
    agent_loop
        .run_once(
            "workspace-1",
            "conversation-1",
            "run-1",
            "default",
            "say hello",
        )
        .await
        .expect("agent loop");
    let recorded = events.list_events("workspace-1").await.expect("events");
    assert!(recorded
        .iter()
        .any(|event| event.payload.to_string().contains("e2e assistant response")));

    let checkpoints = CheckpointStore::new(pool.clone());
    checkpoints
        .save_checkpoint("workspace-1", "run-1", 3, json!({"phase":"e2e"}))
        .await
        .expect("save checkpoint");
    let checkpoint = checkpoints
        .latest_checkpoint("workspace-1", "run-1")
        .await
        .expect("checkpoint query")
        .expect("checkpoint exists");
    assert_eq!(checkpoint.projection["phase"], "e2e");

    let tasks = TaskStore::new(pool.clone());
    let task_id = tasks
        .enqueue(
            "agent_loop_step",
            Some("run-1"),
            10,
            json!({"run_id":"run-1"}),
        )
        .await
        .expect("enqueue");
    let task = tasks
        .lease_next("worker-1", 30)
        .await
        .expect("lease")
        .expect("task leased");
    assert_eq!(task.task_id, task_id);

    let bus = AgentBus::new(pool.clone());
    bus.send_message(
        "workspace-1",
        "conversation-1",
        "lead",
        "default",
        "prefer deterministic tests",
    )
    .await
    .expect("agent message");
    let patches = ContextPatchStore::new(pool)
        .pending_for_agent("workspace-1", "conversation-1", "default")
        .await
        .expect("patches");
    assert!(patches[0].content.contains("prefer deterministic tests"));

    let shadowbox = Shadowbox::new(temp.path());
    let note = temp.path().join("note.txt");
    tokio::fs::write(&note, "alpha beta")
        .await
        .expect("write note");
    let read = FileReadTool
        .call(&shadowbox, json!({"path":"note.txt"}))
        .await
        .expect("file read");
    assert_eq!(read["content"], "alpha beta");
    let search = SearchTool
        .call(&shadowbox, json!({"query":"beta"}))
        .await
        .expect("search");
    assert_eq!(search["matches"].as_array().expect("matches").len(), 1);
    let state = FileState::snapshot(&note).await.expect("snapshot");
    FileEditTool::new(state)
        .call(
            &shadowbox,
            json!({"path":"note.txt","old":"beta","new":"gamma"}),
        )
        .await
        .expect("file edit");
    assert_eq!(
        tokio::fs::read_to_string(&note).await.expect("read edited"),
        "alpha gamma"
    );
    let bash_command = if cfg!(windows) {
        "echo e2e"
    } else {
        "printf e2e"
    };
    let bash = BashTool
        .call(&shadowbox, json!({"command":bash_command}))
        .await
        .expect("bash");
    assert!(bash["stdout"].as_str().expect("stdout").contains("e2e"));

    let skill_dir = paths.user_agents_dir().join("skills/review");
    tokio::fs::create_dir_all(&skill_dir)
        .await
        .expect("skill dir");
    tokio::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review code changes.\n---\n\nFull instructions.",
    )
    .await
    .expect("skill");
    let skills = SkillRegistry::scan_user_data(&paths).await.expect("skills");
    assert_eq!(skills.metadata()[0].name, "review");
    assert_eq!(
        skills.load("review").await.expect("skill load").body.trim(),
        "Full instructions."
    );

    let summary = CompactionEngine::summarize_for_test(&["Pending task: e2e"]);
    assert!(summary.contains("Primary Request and Intent"));
    assert!(summary.contains("Pending Tasks"));

    let mcp =
        serde_json::to_value(McpRequest::new(1, "tools/list", json!({}))).expect("mcp serialize");
    assert_eq!(mcp["jsonrpc"], "2.0");
    assert_eq!(mcp["method"], "tools/list");
}

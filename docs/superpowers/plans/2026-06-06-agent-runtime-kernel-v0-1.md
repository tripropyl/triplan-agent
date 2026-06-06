# Agent Runtime Kernel v0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first executable foundation slice of the Rust-first, CLI-only, SQLite-backed agent runtime kernel in the current workspace.

**Architecture:** Implement one Rust binary crate with focused modules for CLI, config, SQLite event store, scheduler, agent loop, tools, context patches, compaction, and MCP. The first pass uses a mock LLM provider for deterministic tests, then adds OpenAI-compatible and DashScope-compatible request adapters without requiring live API calls in CI.

**Tech Stack:** Rust 2021, `clap`, `tokio`, `serde`, `serde_json`, `toml`, `sqlx` with SQLite, `anyhow`, `thiserror`, `tracing`, `uuid`, `chrono`, `assert_cmd`, `tempfile`, `wiremock` or local mock HTTP where needed.

---

## Scope

This plan implements the first foundation slice of `docs/superpowers/specs/2026-06-06-agent-runtime-kernel-design.md`. The design spec covers several coupled subsystems, so implementation is split into sequential plans. This plan creates the crate, event store, checkpoint/task primitives, mock-provider loop, core tools, skills, Agent Bus, context patch, manual compaction, MCP request foundation, and CLI smoke surface. The next plan will extend this foundation to full v0.1 acceptance with real provider adapters, scheduler workers, automatic/reactive compaction, full MCP process lifecycle, and richer CLI run/debug commands.

This plan intentionally starts from the current workspace, which has no Rust crate yet. Existing `.agents/` and `.codex/` untracked content must not be deleted or mass-staged.

The plan produces working, testable software through vertical slices:

1. crate and CLI initialization
2. SQLite schema and event store
3. workspace config and agent profiles
4. scheduler and checkpoint primitives
5. mock provider and single-agent loop
6. built-in tools and Shadowbox
7. skills and stdio MCP bridge
8. Agent Bus and context patches
9. compaction engine
10. CLI debugging commands and acceptance smoke tests

## File Structure

Create these files:

- `/Users/BaiGod/Documents/agent-ease/Cargo.toml` - crate metadata, dependencies, binary and library targets.
- `/Users/BaiGod/Documents/agent-ease/src/lib.rs` - public module exports.
- `/Users/BaiGod/Documents/agent-ease/src/main.rs` - CLI entrypoint.
- `/Users/BaiGod/Documents/agent-ease/src/error.rs` - crate-wide error type and `Result`.
- `/Users/BaiGod/Documents/agent-ease/src/cli.rs` - `clap` command definitions and command dispatch.
- `/Users/BaiGod/Documents/agent-ease/src/config.rs` - workspace config loading, validation, and default file generation.
- `/Users/BaiGod/Documents/agent-ease/src/model.rs` - core ids and runtime domain structs.
- `/Users/BaiGod/Documents/agent-ease/src/db.rs` - SQLite pool, migrations, event store, tasks, checkpoints, and queries.
- `/Users/BaiGod/Documents/agent-ease/src/provider.rs` - provider trait, mock provider, OpenAI-compatible adapter, DashScope adapter.
- `/Users/BaiGod/Documents/agent-ease/src/shadowbox.rs` - path boundary, env policy, command policy, and output truncation.
- `/Users/BaiGod/Documents/agent-ease/src/tools.rs` - tool trait, registry, built-in tools.
- `/Users/BaiGod/Documents/agent-ease/src/skills.rs` - `.agents/skills` metadata and progressive loading.
- `/Users/BaiGod/Documents/agent-ease/src/mcp.rs` - stdio MCP process wrapper and minimal JSON-RPC tool invocation.
- `/Users/BaiGod/Documents/agent-ease/src/context.rs` - context patches, reactor trait, patch selection, and compaction.
- `/Users/BaiGod/Documents/agent-ease/src/runtime.rs` - scheduler, agent loop, run lifecycle, prompt projection.
- `/Users/BaiGod/Documents/agent-ease/tests/cli_init.rs` - CLI init and config smoke tests.
- `/Users/BaiGod/Documents/agent-ease/tests/event_store.rs` - SQLite event/checkpoint/task tests.
- `/Users/BaiGod/Documents/agent-ease/tests/agent_loop.rs` - mock provider loop tests.
- `/Users/BaiGod/Documents/agent-ease/tests/tools.rs` - Shadowbox and tool tests.
- `/Users/BaiGod/Documents/agent-ease/tests/multi_agent.rs` - Agent Bus and multi-run tests.

Avoid creating a desktop directory, web app, SDK package, or HTTP MCP transport in v0.1.

## Task 0: Rust Toolchain Preflight

**Files:**
- No file changes.

- [ ] **Step 1: Check current toolchain**

Run:

```bash
cargo --version
```

Expected if not configured:

```text
error: rustup could not choose a version of cargo to run
```

- [ ] **Step 2: Configure stable Rust toolchain when needed**

Run only if Step 1 fails with the rustup default error:

```bash
rustup default stable
cargo --version
```

Expected:

```text
cargo 1.
```

- [ ] **Step 3: Confirm repository state before edits**

Run:

```bash
git status --short --branch
```

Expected:

```text
## main
?? .agents/
?? .codex/
```

`docs/` may appear if this plan has not been committed yet. Do not stage `.agents/` or `.codex/`.

## Task 1: Crate Bootstrap And CLI Skeleton

**Files:**
- Create: `/Users/BaiGod/Documents/agent-ease/Cargo.toml`
- Create: `/Users/BaiGod/Documents/agent-ease/src/lib.rs`
- Create: `/Users/BaiGod/Documents/agent-ease/src/main.rs`
- Create: `/Users/BaiGod/Documents/agent-ease/src/error.rs`
- Create: `/Users/BaiGod/Documents/agent-ease/src/cli.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/cli_init.rs`

- [ ] **Step 1: Write failing CLI version test**

Create `/Users/BaiGod/Documents/agent-ease/tests/cli_init.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_prints_version() {
    let mut cmd = Command::cargo_bin("agent").expect("agent binary exists");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("agent "));
}
```

- [ ] **Step 2: Run test to verify it fails before crate exists**

Run:

```bash
cargo test --test cli_init cli_prints_version -- --nocapture
```

Expected: FAIL because `Cargo.toml` or `agent` binary does not exist.

- [ ] **Step 3: Add crate manifest**

Create `/Users/BaiGod/Documents/agent-ease/Cargo.toml`:

```toml
[package]
name = "agent-ease"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "agent"
path = "src/main.rs"

[lib]
name = "agent_ease"
path = "src/lib.rs"

[dependencies]
anyhow = "1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "chrono", "uuid", "json"] }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "fs", "io-util", "time"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4", "serde"] }
walkdir = "2"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 4: Add module exports and error type**

Create `/Users/BaiGod/Documents/agent-ease/src/lib.rs`:

```rust
pub mod cli;
pub mod config;
pub mod context;
pub mod db;
pub mod error;
pub mod mcp;
pub mod model;
pub mod provider;
pub mod runtime;
pub mod shadowbox;
pub mod skills;
pub mod tools;
```

Create `/Users/BaiGod/Documents/agent-ease/src/error.rs`:

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}
```

- [ ] **Step 5: Add CLI skeleton**

Create `/Users/BaiGod/Documents/agent-ease/src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

use crate::error::Result;

#[derive(Debug, Parser)]
#[command(name = "agent", version, about = "Local event-driven agent runtime")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Chat {
        #[arg(long, default_value = "default")]
        agent: String,
    },
    Run {
        #[arg(long, default_value = "default")]
        agent: String,
        prompt: Vec<String>,
    },
    Status,
    Events,
    Doctor,
}

pub async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command.unwrap_or(Command::Status) {
        Command::Init => {
            println!("agent init is not implemented yet");
        }
        Command::Chat { agent } => {
            println!("agent chat requested for {agent}");
        }
        Command::Run { agent, prompt } => {
            println!("agent run requested for {agent}: {}", prompt.join(" "));
        }
        Command::Status => {
            println!("agent runtime status: uninitialized");
        }
        Command::Events => {
            println!("agent events: unavailable before init");
        }
        Command::Doctor => {
            println!("agent doctor: basic CLI is available");
        }
    }
    Ok(())
}
```

Create `/Users/BaiGod/Documents/agent-ease/src/main.rs`:

```rust
use clap::Parser;

use agent_ease::cli::{dispatch, Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    dispatch(Cli::parse()).await?;
    Ok(())
}
```

Create empty modules so the crate compiles:

```bash
for f in config context db mcp model provider runtime shadowbox skills tools; do printf '' > "src/$f.rs"; done
```

- [ ] **Step 6: Run test to verify CLI skeleton passes**

Run:

```bash
cargo test --test cli_init cli_prints_version -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add Cargo.toml src tests/cli_init.rs
git commit -m "feat: bootstrap agent CLI crate"
```

Expected: commit succeeds and `.agents/` / `.codex/` remain untracked.

## Task 2: Workspace Init And Config Files

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/config.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/cli.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/cli_init.rs`

- [ ] **Step 1: Add failing `agent init` test**

Append to `/Users/BaiGod/Documents/agent-ease/tests/cli_init.rs`:

```rust
#[test]
fn init_creates_workspace_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("agent").expect("agent binary exists");
    cmd.current_dir(temp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized agent workspace"));

    assert!(temp.path().join(".agents/config.toml").is_file());
    assert!(temp.path().join(".agents/agents/lead.toml").is_file());
    assert!(temp.path().join(".agents/agents/default.toml").is_file());
    assert!(temp.path().join(".agents/prompts/compact/default.md").is_file());
    assert!(temp.path().join(".agents/mcp.toml").is_file());
    assert!(temp.path().join(".agents/shadowbox.toml").is_file());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test cli_init init_creates_workspace_files -- --nocapture
```

Expected: FAIL because `agent init` still prints a temporary message and creates no files.

- [ ] **Step 3: Implement config generation**

Replace `/Users/BaiGod/Documents/agent-ease/src/config.rs` with:

```rust
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{AgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace_name: String,
    pub default_agent: String,
    pub default_provider: String,
    pub database_path: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            workspace_name: "agent-workspace".to_string(),
            default_agent: "default".to_string(),
            default_provider: "mock".to_string(),
            database_path: ".agents/agent.db".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfileConfig {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub model: String,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub mcp_servers: Vec<String>,
}

pub fn agents_dir(workspace: &Path) -> PathBuf {
    workspace.join(".agents")
}

pub fn config_path(workspace: &Path) -> PathBuf {
    agents_dir(workspace).join("config.toml")
}

pub async fn init_workspace(workspace: &Path) -> Result<()> {
    let root = agents_dir(workspace);
    fs::create_dir_all(root.join("agents")).await?;
    fs::create_dir_all(root.join("skills")).await?;
    fs::create_dir_all(root.join("prompts/compact")).await?;

    write_toml_if_missing(&root.join("config.toml"), &WorkspaceConfig::default()).await?;
    write_toml_if_missing(
        &root.join("agents/default.toml"),
        &AgentProfileConfig {
            name: "default".to_string(),
            description: "General-purpose local agent.".to_string(),
            system_prompt: "You are a careful local agent working inside this workspace.".to_string(),
            model: "mock".to_string(),
            tools: vec![
                "bash".to_string(),
                "file_read".to_string(),
                "file_write".to_string(),
                "file_edit".to_string(),
                "search".to_string(),
                "skill".to_string(),
                "agent_message".to_string(),
                "compact".to_string(),
                "mcp_bridge".to_string(),
            ],
            skills: vec![],
            mcp_servers: vec![],
        },
    )
    .await?;
    write_toml_if_missing(
        &root.join("agents/lead.toml"),
        &AgentProfileConfig {
            name: "lead".to_string(),
            description: "Lead agent that coordinates worker agents.".to_string(),
            system_prompt: "You coordinate work, delegate carefully, and summarize results.".to_string(),
            model: "mock".to_string(),
            tools: vec!["agent_message".to_string(), "compact".to_string(), "skill".to_string()],
            skills: vec![],
            mcp_servers: vec![],
        },
    )
    .await?;
    write_if_missing(
        &root.join("prompts/compact/default.md"),
        DEFAULT_COMPACT_PROMPT,
    )
    .await?;
    write_if_missing(&root.join("mcp.toml"), "[servers]\n").await?;
    write_if_missing(
        &root.join("shadowbox.toml"),
        "workspace_boundary = true\ncommand_timeout_seconds = 30\nmax_output_bytes = 65536\n",
    )
    .await?;
    Ok(())
}

pub async fn load_workspace_config(workspace: &Path) -> Result<WorkspaceConfig> {
    let path = config_path(workspace);
    let content = fs::read_to_string(&path)
        .await
        .map_err(|_| AgentError::Config(format!("missing workspace config at {}", path.display())))?;
    Ok(toml::from_str(&content)?)
}

async fn write_toml_if_missing<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let content = toml::to_string_pretty(value)?;
    write_if_missing(path, &content).await
}

async fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, content).await?;
    Ok(())
}

const DEFAULT_COMPACT_PROMPT: &str = r#"# Compact Summary Instructions

Summarize the conversation so a future agent can continue the work.

Include:
- primary request and intent
- key technical decisions
- files and code sections
- errors and fixes
- pending tasks
- current work
- next step
"#;
```

- [ ] **Step 4: Wire `agent init`**

Modify the `Command::Init` arm in `/Users/BaiGod/Documents/agent-ease/src/cli.rs`:

```rust
Command::Init => {
    crate::config::init_workspace(&std::env::current_dir()?).await?;
    println!("initialized agent workspace");
}
```

- [ ] **Step 5: Run init test**

Run:

```bash
cargo test --test cli_init init_creates_workspace_files -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/config.rs src/cli.rs tests/cli_init.rs
git commit -m "feat: initialize agent workspace config"
```

Expected: commit succeeds.

## Task 3: SQLite Schema And Event Store

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/model.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/db.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/event_store.rs`

- [ ] **Step 1: Write failing event append test**

Create `/Users/BaiGod/Documents/agent-ease/tests/event_store.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test event_store appends_events_with_monotonic_sequence -- --nocapture
```

Expected: FAIL because `db` and `model` are empty.

- [ ] **Step 3: Add domain model types**

Replace `/Users/BaiGod/Documents/agent-ease/src/model.rs` with:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    UserMessage,
    RunRequested,
    RunStarted,
    ModelRequestStarted,
    AssistantMessage,
    ToolCallRequested,
    ToolCallCompleted,
    AgentMessageSent,
    ContextPatchCreated,
    CompactRequested,
    CompactStarted,
    CompactCompleted,
    CompactFailed,
    RunPaused,
    RunResumed,
    RunCancelled,
    RunCompleted,
    RunFailed,
}

impl EventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserMessage => "user_message",
            Self::RunRequested => "run_requested",
            Self::RunStarted => "run_started",
            Self::ModelRequestStarted => "model_request_started",
            Self::AssistantMessage => "assistant_message",
            Self::ToolCallRequested => "tool_call_requested",
            Self::ToolCallCompleted => "tool_call_completed",
            Self::AgentMessageSent => "agent_message_sent",
            Self::ContextPatchCreated => "context_patch_created",
            Self::CompactRequested => "compact_requested",
            Self::CompactStarted => "compact_started",
            Self::CompactCompleted => "compact_completed",
            Self::CompactFailed => "compact_failed",
            Self::RunPaused => "run_paused",
            Self::RunResumed => "run_resumed",
            Self::RunCancelled => "run_cancelled",
            Self::RunCompleted => "run_completed",
            Self::RunFailed => "run_failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    Json(Value),
}

impl EventPayload {
    pub fn into_value(self) -> Value {
        match self {
            Self::Json(value) => value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event_id: Uuid,
    pub workspace_id: String,
    pub conversation_id: Option<String>,
    pub run_id: Option<String>,
    pub agent_id: Option<String>,
    pub event_type: EventType,
    pub payload: Value,
    pub parent_event_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Implement SQLite migration and event store**

Replace `/Users/BaiGod/Documents/agent-ease/src/db.rs` with:

```rust
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use uuid::Uuid;

use crate::error::Result;
use crate::model::{EventPayload, EventRecord, EventType};

pub async fn connect_sqlite(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON;").execute(&pool).await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            event_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            conversation_id TEXT,
            run_id TEXT,
            agent_id TEXT,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            parent_event_id TEXT,
            causation_id TEXT,
            correlation_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_events_workspace_sequence
        ON events(workspace_id, sequence);
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Clone)]
pub struct EventStore {
    pool: SqlitePool,
}

impl EventStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn append_event(
        &self,
        workspace_id: &str,
        conversation_id: Option<&str>,
        run_id: Option<&str>,
        agent_id: Option<&str>,
        event_type: EventType,
        payload: EventPayload,
        parent_event_id: Option<Uuid>,
        causation_id: Option<Uuid>,
        correlation_id: Option<Uuid>,
    ) -> Result<EventRecord> {
        let mut tx = self.pool.begin().await?;
        let next_sequence: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM events WHERE workspace_id = ?",
        )
        .bind(workspace_id)
        .fetch_one(&mut *tx)
        .await?;

        let event_id = Uuid::new_v4();
        let correlation_id = correlation_id.unwrap_or(event_id);
        let created_at = Utc::now();
        let payload_value = payload.into_value();
        sqlx::query(
            r#"
            INSERT INTO events (
                event_id, workspace_id, conversation_id, run_id, agent_id,
                event_type, payload_json, parent_event_id, causation_id,
                correlation_id, sequence, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event_id.to_string())
        .bind(workspace_id)
        .bind(conversation_id)
        .bind(run_id)
        .bind(agent_id)
        .bind(event_type.as_str())
        .bind(payload_value.to_string())
        .bind(parent_event_id.map(|id| id.to_string()))
        .bind(causation_id.map(|id| id.to_string()))
        .bind(correlation_id.to_string())
        .bind(next_sequence)
        .bind(created_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(EventRecord {
            event_id,
            workspace_id: workspace_id.to_string(),
            conversation_id: conversation_id.map(str::to_string),
            run_id: run_id.map(str::to_string),
            agent_id: agent_id.map(str::to_string),
            event_type,
            payload: payload_value,
            parent_event_id,
            causation_id,
            correlation_id,
            sequence: next_sequence,
            created_at,
        })
    }

    pub async fn list_events(&self, workspace_id: &str) -> Result<Vec<EventRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, workspace_id, conversation_id, run_id, agent_id,
                   event_type, payload_json, parent_event_id, causation_id,
                   correlation_id, sequence, created_at
            FROM events
            WHERE workspace_id = ?
            ORDER BY sequence ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }
}

fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<EventRecord> {
    let event_type_text: String = row.try_get("event_type")?;
    let event_type = match event_type_text.as_str() {
        "user_message" => EventType::UserMessage,
        "run_requested" => EventType::RunRequested,
        "run_started" => EventType::RunStarted,
        "model_request_started" => EventType::ModelRequestStarted,
        "assistant_message" => EventType::AssistantMessage,
        "tool_call_requested" => EventType::ToolCallRequested,
        "tool_call_completed" => EventType::ToolCallCompleted,
        "agent_message_sent" => EventType::AgentMessageSent,
        "context_patch_created" => EventType::ContextPatchCreated,
        "compact_requested" => EventType::CompactRequested,
        "compact_started" => EventType::CompactStarted,
        "compact_completed" => EventType::CompactCompleted,
        "compact_failed" => EventType::CompactFailed,
        "run_paused" => EventType::RunPaused,
        "run_resumed" => EventType::RunResumed,
        "run_cancelled" => EventType::RunCancelled,
        "run_completed" => EventType::RunCompleted,
        _ => EventType::RunFailed,
    };
    let payload_json: String = row.try_get("payload_json")?;
    let created_at: String = row.try_get("created_at")?;
    Ok(EventRecord {
        event_id: Uuid::parse_str(&row.try_get::<String, _>("event_id")?).expect("valid event uuid"),
        workspace_id: row.try_get("workspace_id")?,
        conversation_id: row.try_get("conversation_id")?,
        run_id: row.try_get("run_id")?,
        agent_id: row.try_get("agent_id")?,
        event_type,
        payload: serde_json::from_str::<Value>(&payload_json)?,
        parent_event_id: parse_optional_uuid(row.try_get("parent_event_id")?),
        causation_id: parse_optional_uuid(row.try_get("causation_id")?),
        correlation_id: Uuid::parse_str(&row.try_get::<String, _>("correlation_id")?).expect("valid correlation uuid"),
        sequence: row.try_get("sequence")?,
        created_at: DateTime::parse_from_rfc3339(&created_at).expect("valid timestamp").with_timezone(&Utc),
    })
}

fn parse_optional_uuid(value: Option<String>) -> Option<Uuid> {
    value.and_then(|text| Uuid::parse_str(&text).ok())
}
```

- [ ] **Step 5: Run event store test**

Run:

```bash
cargo test --test event_store appends_events_with_monotonic_sequence -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/model.rs src/db.rs tests/event_store.rs
git commit -m "feat: add sqlite event store"
```

Expected: commit succeeds.

## Task 4: Checkpoints And Task Leases

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/model.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/db.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/event_store.rs`

- [ ] **Step 1: Add failing checkpoint and lease tests**

Append to `/Users/BaiGod/Documents/agent-ease/tests/event_store.rs`:

```rust
use agent_ease::db::{CheckpointStore, TaskStore};

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
        .enqueue("agent_loop_step", Some("run-1"), 10, json!({"run_id":"run-1"}))
        .await
        .expect("enqueue");

    let first = tasks.lease_next("worker-a", 30).await.expect("first lease");
    let second = tasks.lease_next("worker-b", 30).await.expect("second lease");

    assert_eq!(first.expect("leased task").task_id, task_id);
    assert!(second.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --test event_store checkpoint_round_trips_projection task_lease_is_exclusive -- --nocapture
```

Expected: FAIL because `CheckpointStore` and `TaskStore` are missing.

- [ ] **Step 3: Extend model types**

Append to `/Users/BaiGod/Documents/agent-ease/src/model.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub checkpoint_id: Uuid,
    pub workspace_id: String,
    pub run_id: String,
    pub after_sequence: i64,
    pub projection: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: Uuid,
    pub task_type: String,
    pub target_run_id: Option<String>,
    pub status: String,
    pub priority: i64,
    pub lease_owner: Option<String>,
    pub payload: Value,
}
```

- [ ] **Step 4: Add migrations for checkpoints and tasks**

In `/Users/BaiGod/Documents/agent-ease/src/db.rs`, add these `CREATE TABLE` statements inside `migrate` after the events index:

```rust
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS checkpoints (
        checkpoint_id TEXT PRIMARY KEY,
        workspace_id TEXT NOT NULL,
        run_id TEXT NOT NULL,
        after_sequence INTEGER NOT NULL,
        projection_json TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    "#,
)
.execute(pool)
.await?;
sqlx::query(
    r#"
    CREATE INDEX IF NOT EXISTS idx_checkpoints_run_sequence
    ON checkpoints(workspace_id, run_id, after_sequence DESC);
    "#,
)
.execute(pool)
.await?;
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS tasks (
        task_id TEXT PRIMARY KEY,
        task_type TEXT NOT NULL,
        target_run_id TEXT,
        status TEXT NOT NULL,
        priority INTEGER NOT NULL,
        lease_owner TEXT,
        lease_expires_at TEXT,
        attempt_count INTEGER NOT NULL DEFAULT 0,
        payload_json TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    "#,
)
.execute(pool)
.await?;
```

- [ ] **Step 5: Implement stores**

Append to `/Users/BaiGod/Documents/agent-ease/src/db.rs`:

```rust
use crate::model::{CheckpointRecord, TaskRecord};

#[derive(Clone)]
pub struct CheckpointStore {
    pool: SqlitePool,
}

impl CheckpointStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save_checkpoint(
        &self,
        workspace_id: &str,
        run_id: &str,
        after_sequence: i64,
        projection: Value,
    ) -> Result<Uuid> {
        let checkpoint_id = Uuid::new_v4();
        let created_at = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO checkpoints (
                checkpoint_id, workspace_id, run_id, after_sequence, projection_json, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(checkpoint_id.to_string())
        .bind(workspace_id)
        .bind(run_id)
        .bind(after_sequence)
        .bind(projection.to_string())
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(checkpoint_id)
    }

    pub async fn latest_checkpoint(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Option<CheckpointRecord>> {
        let row = sqlx::query(
            r#"
            SELECT checkpoint_id, workspace_id, run_id, after_sequence, projection_json, created_at
            FROM checkpoints
            WHERE workspace_id = ? AND run_id = ?
            ORDER BY after_sequence DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let created_at: String = row.try_get("created_at")?;
            let projection_json: String = row.try_get("projection_json")?;
            Ok(CheckpointRecord {
                checkpoint_id: Uuid::parse_str(&row.try_get::<String, _>("checkpoint_id")?).expect("valid checkpoint uuid"),
                workspace_id: row.try_get("workspace_id")?,
                run_id: row.try_get("run_id")?,
                after_sequence: row.try_get("after_sequence")?,
                projection: serde_json::from_str(&projection_json)?,
                created_at: DateTime::parse_from_rfc3339(&created_at).expect("valid timestamp").with_timezone(&Utc),
            })
        })
        .transpose()
    }
}

#[derive(Clone)]
pub struct TaskStore {
    pool: SqlitePool,
}

impl TaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn enqueue(
        &self,
        task_type: &str,
        target_run_id: Option<&str>,
        priority: i64,
        payload: Value,
    ) -> Result<Uuid> {
        let task_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO tasks (
                task_id, task_type, target_run_id, status, priority,
                payload_json, created_at
            )
            VALUES (?, ?, ?, 'pending', ?, ?, ?)
            "#,
        )
        .bind(task_id.to_string())
        .bind(task_type)
        .bind(target_run_id)
        .bind(priority)
        .bind(payload.to_string())
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(task_id)
    }

    pub async fn lease_next(&self, worker_id: &str, lease_seconds: i64) -> Result<Option<TaskRecord>> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT task_id, task_type, target_run_id, status, priority, lease_owner, payload_json
            FROM tasks
            WHERE status = 'pending'
            ORDER BY priority DESC, created_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let task_id_text: String = row.try_get("task_id")?;
        let expires = Utc::now() + chrono::Duration::seconds(lease_seconds);
        sqlx::query(
            "UPDATE tasks SET status = 'leased', lease_owner = ?, lease_expires_at = ? WHERE task_id = ? AND status = 'pending'",
        )
        .bind(worker_id)
        .bind(expires.to_rfc3339())
        .bind(&task_id_text)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        let payload_json: String = row.try_get("payload_json")?;
        Ok(Some(TaskRecord {
            task_id: Uuid::parse_str(&task_id_text).expect("valid task uuid"),
            task_type: row.try_get("task_type")?,
            target_run_id: row.try_get("target_run_id")?,
            status: "leased".to_string(),
            priority: row.try_get("priority")?,
            lease_owner: Some(worker_id.to_string()),
            payload: serde_json::from_str(&payload_json)?,
        }))
    }
}
```

- [ ] **Step 6: Run checkpoint and lease tests**

Run:

```bash
cargo test --test event_store checkpoint_round_trips_projection task_lease_is_exclusive -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add src/model.rs src/db.rs tests/event_store.rs
git commit -m "feat: add checkpoints and task leases"
```

Expected: commit succeeds.

## Task 5: Provider Trait And Mock Agent Loop

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/provider.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/runtime.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/agent_loop.rs`

- [ ] **Step 1: Write failing agent loop test**

Create `/Users/BaiGod/Documents/agent-ease/tests/agent_loop.rs`:

```rust
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
        .run_once("workspace-1", "conversation-1", "run-1", "default", "say hello")
        .await
        .expect("run once");

    let events = store.list_events("workspace-1").await.expect("events");
    assert!(events.iter().any(|event| event.payload.to_string().contains("hello from mock")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test agent_loop mock_agent_loop_records_assistant_message -- --nocapture
```

Expected: FAIL because provider and runtime loop do not exist.

- [ ] **Step 3: Implement provider trait and mock provider**

Replace `/Users/BaiGod/Documents/agent-ease/src/provider.rs` with:

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<ModelMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelOutput {
    Text(String),
    ToolCall { name: String, arguments: serde_json::Value },
}

#[async_trait]
pub trait LlmProvider: Send + Sync + Clone + 'static {
    async fn complete(&self, request: ModelRequest) -> Result<ModelOutput>;
}

#[derive(Debug, Clone)]
pub struct MockProvider {
    response: String,
}

impl MockProvider {
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _request: ModelRequest) -> Result<ModelOutput> {
        Ok(ModelOutput::Text(self.response.clone()))
    }
}
```

- [ ] **Step 4: Implement minimal agent loop**

Replace `/Users/BaiGod/Documents/agent-ease/src/runtime.rs` with:

```rust
use serde_json::json;

use crate::db::EventStore;
use crate::error::Result;
use crate::model::{EventPayload, EventType};
use crate::provider::{LlmProvider, ModelMessage, ModelOutput, ModelRequest};

#[derive(Clone)]
pub struct AgentLoop<P: LlmProvider> {
    store: EventStore,
    provider: P,
}

impl<P: LlmProvider> AgentLoop<P> {
    pub fn new(store: EventStore, provider: P) -> Self {
        Self { store, provider }
    }

    pub async fn run_once(
        &self,
        workspace_id: &str,
        conversation_id: &str,
        run_id: &str,
        agent_id: &str,
        prompt: &str,
    ) -> Result<()> {
        let requested = self
            .store
            .append_event(
                workspace_id,
                Some(conversation_id),
                Some(run_id),
                Some(agent_id),
                EventType::RunRequested,
                EventPayload::Json(json!({ "prompt": prompt })),
                None,
                None,
                None,
            )
            .await?;
        self.store
            .append_event(
                workspace_id,
                Some(conversation_id),
                Some(run_id),
                Some(agent_id),
                EventType::RunStarted,
                EventPayload::Json(json!({})),
                Some(requested.event_id),
                Some(requested.event_id),
                Some(requested.correlation_id),
            )
            .await?;
        let output = self
            .provider
            .complete(ModelRequest {
                model: "mock".to_string(),
                messages: vec![ModelMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                }],
            })
            .await?;
        match output {
            ModelOutput::Text(text) => {
                self.store
                    .append_event(
                        workspace_id,
                        Some(conversation_id),
                        Some(run_id),
                        Some(agent_id),
                        EventType::AssistantMessage,
                        EventPayload::Json(json!({ "content": text })),
                        Some(requested.event_id),
                        Some(requested.event_id),
                        Some(requested.correlation_id),
                    )
                    .await?;
            }
            ModelOutput::ToolCall { name, arguments } => {
                self.store
                    .append_event(
                        workspace_id,
                        Some(conversation_id),
                        Some(run_id),
                        Some(agent_id),
                        EventType::ToolCallRequested,
                        EventPayload::Json(json!({ "name": name, "arguments": arguments })),
                        Some(requested.event_id),
                        Some(requested.event_id),
                        Some(requested.correlation_id),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 5: Run agent loop test**

Run:

```bash
cargo test --test agent_loop mock_agent_loop_records_assistant_message -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/provider.rs src/runtime.rs tests/agent_loop.rs
git commit -m "feat: add mock provider agent loop"
```

Expected: commit succeeds.

## Task 6: Shadowbox And File/Search Tools

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/shadowbox.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/tools.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`

- [ ] **Step 1: Write failing Shadowbox path test**

Create `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`:

```rust
use agent_ease::shadowbox::Shadowbox;
use agent_ease::tools::{FileReadTool, SearchTool, Tool};
use serde_json::json;

#[tokio::test]
async fn shadowbox_blocks_outside_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let shadowbox = Shadowbox::new(temp.path());
    let outside = temp.path().parent().expect("parent").join("outside.txt");
    let err = shadowbox.ensure_inside_workspace(&outside).expect_err("outside rejected");
    assert!(err.to_string().contains("outside workspace"));
}

#[tokio::test]
async fn file_read_reads_workspace_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("note.txt");
    tokio::fs::write(&path, "hello file").await.expect("write");
    let shadowbox = Shadowbox::new(temp.path());
    let output = FileReadTool
        .call(&shadowbox, json!({"path":"note.txt"}))
        .await
        .expect("read");
    assert_eq!(output["content"], "hello file");
}

#[tokio::test]
async fn search_finds_text_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    tokio::fs::write(temp.path().join("a.txt"), "alpha beta").await.expect("write");
    let shadowbox = Shadowbox::new(temp.path());
    let output = SearchTool
        .call(&shadowbox, json!({"query":"beta"}))
        .await
        .expect("search");
    assert!(output["matches"].as_array().expect("array")[0]["path"].as_str().expect("path").ends_with("a.txt"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --test tools -- --nocapture
```

Expected: FAIL because Shadowbox and tools are empty.

- [ ] **Step 3: Implement Shadowbox path boundary**

Replace `/Users/BaiGod/Documents/agent-ease/src/shadowbox.rs` with:

```rust
use std::path::{Path, PathBuf};

use crate::error::{AgentError, Result};

#[derive(Debug, Clone)]
pub struct Shadowbox {
    workspace_root: PathBuf,
    max_output_bytes: usize,
}

impl Shadowbox {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            max_output_bytes: 65_536,
        }
    }

    pub fn resolve_workspace_path(&self, relative: &str) -> Result<PathBuf> {
        let candidate = self.workspace_root.join(relative);
        self.ensure_inside_workspace(&candidate)?;
        Ok(candidate)
    }

    pub fn ensure_inside_workspace(&self, path: &Path) -> Result<()> {
        let root = self.workspace_root.canonicalize()?;
        let absolute = if path.exists() {
            path.canonicalize()?
        } else {
            path.parent()
                .unwrap_or(&self.workspace_root)
                .canonicalize()?
                .join(path.file_name().unwrap_or_default())
        };
        if !absolute.starts_with(&root) {
            return Err(AgentError::PolicyDenied(format!(
                "path outside workspace: {}",
                path.display()
            )));
        }
        Ok(())
    }

    pub fn truncate_output(&self, text: &str) -> String {
        if text.len() <= self.max_output_bytes {
            return text.to_string();
        }
        format!("{}\\n[output truncated]", &text[..self.max_output_bytes])
    }
}
```

- [ ] **Step 4: Implement tool trait, file read, and search**

Replace `/Users/BaiGod/Documents/agent-ease/src/tools.rs` with:

```rust
use async_trait::async_trait;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::error::{AgentError, Result};
use crate::shadowbox::Shadowbox;

#[async_trait]
pub trait Tool {
    fn name(&self) -> &'static str;
    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value>;
}

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_read requires path".to_string()))?;
        let full_path = shadowbox.resolve_workspace_path(path)?;
        let content = tokio::fs::read_to_string(full_path).await?;
        Ok(json!({ "content": shadowbox.truncate_output(&content) }))
    }
}

pub struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &'static str {
        "search"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("search requires query".to_string()))?;
        let mut matches = Vec::new();
        for entry in WalkDir::new(".").into_iter().filter_map(std::result::Result::ok) {
            let path = entry.path();
            if !entry.file_type().is_file() {
                continue;
            }
            if path.components().any(|component| component.as_os_str() == ".git") {
                continue;
            }
            let full_path = shadowbox.resolve_workspace_path(&path.to_string_lossy())?;
            let Ok(content) = tokio::fs::read_to_string(&full_path).await else {
                continue;
            };
            if content.contains(query) {
                matches.push(json!({
                    "path": path.to_string_lossy(),
                    "snippet": shadowbox.truncate_output(&content),
                }));
            }
        }
        Ok(json!({ "matches": matches }))
    }
}
```

- [ ] **Step 5: Run tool tests**

Run:

```bash
cargo test --test tools -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/shadowbox.rs src/tools.rs tests/tools.rs
git commit -m "feat: add shadowbox file and search tools"
```

Expected: commit succeeds.

## Task 7: Bash Tool, Tool Audit, And Stale Edit Guard

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/model.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/db.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/tools.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`

- [ ] **Step 1: Add failing tests for bash and stale edit**

Append to `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`:

```rust
use agent_ease::tools::{BashTool, FileEditTool, FileState};

#[tokio::test]
async fn bash_runs_with_timeout_and_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let shadowbox = Shadowbox::new(temp.path());
    let output = BashTool
        .call(&shadowbox, json!({"command":"printf hello"}))
        .await
        .expect("bash");
    assert_eq!(output["exit_code"], 0);
    assert_eq!(output["stdout"], "hello");
}

#[tokio::test]
async fn file_edit_rejects_stale_version() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("note.txt");
    tokio::fs::write(&path, "old").await.expect("write");
    let shadowbox = Shadowbox::new(temp.path());
    let state = FileState::snapshot(&path).await.expect("snapshot");
    tokio::fs::write(&path, "changed").await.expect("external change");
    let err = FileEditTool::new(state)
        .call(&shadowbox, json!({"path":"note.txt","old":"old","new":"new"}))
        .await
        .expect_err("stale edit rejected");
    assert!(err.to_string().contains("file changed since read"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --test tools bash_runs_with_timeout_and_output file_edit_rejects_stale_version -- --nocapture
```

Expected: FAIL because `BashTool`, `FileState`, and `FileEditTool` are missing.

- [ ] **Step 3: Implement bash and stale edit types**

Append to `/Users/BaiGod/Documents/agent-ease/src/tools.rs`:

```rust
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("bash requires command".to_string()))?;
        let output = timeout(
            Duration::from_secs(30),
            Command::new("sh").arg("-c").arg(command).output(),
        )
        .await
        .map_err(|_| AgentError::Runtime("bash command timed out".to_string()))??;
        Ok(json!({
            "exit_code": output.status.code().unwrap_or(-1),
            "stdout": shadowbox.truncate_output(&String::from_utf8_lossy(&output.stdout)),
            "stderr": shadowbox.truncate_output(&String::from_utf8_lossy(&output.stderr)),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct FileState {
    path: PathBuf,
    modified: std::time::SystemTime,
}

impl FileState {
    pub async fn snapshot(path: &Path) -> Result<Self> {
        let metadata = tokio::fs::metadata(path).await?;
        Ok(Self {
            path: path.to_path_buf(),
            modified: metadata.modified()?,
        })
    }

    async fn ensure_fresh(&self) -> Result<()> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        if metadata.modified()? != self.modified {
            return Err(AgentError::PolicyDenied("file changed since read".to_string()));
        }
        Ok(())
    }
}

pub struct FileEditTool {
    state: FileState,
}

impl FileEditTool {
    pub fn new(state: FileState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &'static str {
        "file_edit"
    }

    async fn call(&self, shadowbox: &Shadowbox, input: Value) -> Result<Value> {
        self.state.ensure_fresh().await?;
        let path = input["path"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_edit requires path".to_string()))?;
        let old = input["old"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_edit requires old".to_string()))?;
        let new = input["new"]
            .as_str()
            .ok_or_else(|| AgentError::Runtime("file_edit requires new".to_string()))?;
        let full_path = shadowbox.resolve_workspace_path(path)?;
        let content = tokio::fs::read_to_string(&full_path).await?;
        if !content.contains(old) {
            return Err(AgentError::Runtime("old text not found".to_string()));
        }
        tokio::fs::write(&full_path, content.replacen(old, new, 1)).await?;
        Ok(json!({"edited": true}))
    }
}
```

- [ ] **Step 4: Add tool invocation audit migration**

Extend `/Users/BaiGod/Documents/agent-ease/src/db.rs` migration with:

```rust
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS tool_invocations (
        invocation_id TEXT PRIMARY KEY,
        workspace_id TEXT NOT NULL,
        run_id TEXT,
        tool_name TEXT NOT NULL,
        input_json TEXT NOT NULL,
        output_json TEXT,
        status TEXT NOT NULL,
        started_at TEXT NOT NULL,
        completed_at TEXT
    );
    "#,
)
.execute(pool)
.await?;
```

- [ ] **Step 5: Run bash and stale edit tests**

Run:

```bash
cargo test --test tools bash_runs_with_timeout_and_output file_edit_rejects_stale_version -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/model.rs src/db.rs src/tools.rs tests/tools.rs
git commit -m "feat: add bash tool and stale edit guard"
```

Expected: commit succeeds.

## Task 8: Skills Registry

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/skills.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`

- [ ] **Step 1: Add failing skill discovery test**

Append to `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`:

```rust
use agent_ease::skills::SkillRegistry;

#[tokio::test]
async fn skill_registry_loads_metadata_progressively() {
    let temp = tempfile::tempdir().expect("tempdir");
    let skill_dir = temp.path().join(".agents/skills/review");
    tokio::fs::create_dir_all(&skill_dir).await.expect("mkdir");
    tokio::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review code changes.\n---\n\nFull instructions.",
    )
    .await
    .expect("write skill");

    let registry = SkillRegistry::scan(temp.path()).await.expect("scan");
    assert_eq!(registry.metadata()[0].name, "review");
    assert_eq!(registry.load("review").await.expect("load").body.trim(), "Full instructions.");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test tools skill_registry_loads_metadata_progressively -- --nocapture
```

Expected: FAIL because `SkillRegistry` is missing.

- [ ] **Step 3: Implement skills registry**

Replace `/Users/BaiGod/Documents/agent-ease/src/skills.rs` with:

```rust
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;
use walkdir::WalkDir;

use crate::error::{AgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LoadedSkill {
    pub metadata: SkillMetadata,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct SkillRegistry {
    skills: Vec<SkillMetadata>,
}

impl SkillRegistry {
    pub async fn scan(workspace: &Path) -> Result<Self> {
        let root = workspace.join(".agents/skills");
        let mut skills = Vec::new();
        if !root.exists() {
            return Ok(Self { skills });
        }
        for entry in WalkDir::new(root).into_iter().filter_map(std::result::Result::ok) {
            if entry.file_name() != "SKILL.md" {
                continue;
            }
            let path = entry.path().to_path_buf();
            let content = fs::read_to_string(&path).await?;
            let metadata = parse_skill_metadata(&content, path)?;
            skills.push(metadata);
        }
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { skills })
    }

    pub fn metadata(&self) -> &[SkillMetadata] {
        &self.skills
    }

    pub async fn load(&self, name: &str) -> Result<LoadedSkill> {
        let metadata = self
            .skills
            .iter()
            .find(|skill| skill.name == name)
            .cloned()
            .ok_or_else(|| AgentError::NotFound(format!("skill {name}")))?;
        let content = fs::read_to_string(&metadata.path).await?;
        let body = content
            .split_once("---")
            .and_then(|(_, rest)| rest.split_once("---"))
            .map(|(_, body)| body.to_string())
            .unwrap_or(content);
        Ok(LoadedSkill { metadata, body })
    }
}

fn parse_skill_metadata(content: &str, path: PathBuf) -> Result<SkillMetadata> {
    let (_, rest) = content
        .split_once("---")
        .ok_or_else(|| AgentError::Config("skill missing front matter".to_string()))?;
    let (front_matter, _) = rest
        .split_once("---")
        .ok_or_else(|| AgentError::Config("skill missing closing front matter".to_string()))?;
    let value: toml::Value = front_matter.parse::<toml::Value>()?;
    let name = value
        .get("name")
        .and_then(|value| value.as_str())
        .ok_or_else(|| AgentError::Config("skill missing name".to_string()))?;
    let description = value
        .get("description")
        .and_then(|value| value.as_str())
        .ok_or_else(|| AgentError::Config("skill missing description".to_string()))?;
    Ok(SkillMetadata {
        name: name.to_string(),
        description: description.to_string(),
        path,
    })
}
```

- [ ] **Step 4: Run skill registry test**

Run:

```bash
cargo test --test tools skill_registry_loads_metadata_progressively -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/skills.rs tests/tools.rs
git commit -m "feat: add skill registry"
```

Expected: commit succeeds.

## Task 9: Agent Bus And Context Patches

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/context.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/db.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/multi_agent.rs`

- [ ] **Step 1: Write failing Agent Bus test**

Create `/Users/BaiGod/Documents/agent-ease/tests/multi_agent.rs`:

```rust
use agent_ease::context::{AgentBus, ContextPatchStore};
use agent_ease::db::{connect_sqlite, migrate};

#[tokio::test]
async fn agent_message_becomes_context_patch() {
    let pool = connect_sqlite("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let bus = AgentBus::new(pool.clone());
    let patches = ContextPatchStore::new(pool);

    bus.send_message("workspace-1", "conversation-1", "lead", "worker", "prefer config first")
        .await
        .expect("send");
    let pending = patches.pending_for_agent("workspace-1", "conversation-1", "worker").await.expect("pending");

    assert_eq!(pending[0].kind, "agent_message");
    assert!(pending[0].content.contains("prefer config first"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test multi_agent agent_message_becomes_context_patch -- --nocapture
```

Expected: FAIL because Agent Bus and patches are missing.

- [ ] **Step 3: Add context patch tables**

Extend `migrate` in `/Users/BaiGod/Documents/agent-ease/src/db.rs`:

```rust
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS context_patches (
        patch_id TEXT PRIMARY KEY,
        workspace_id TEXT NOT NULL,
        conversation_id TEXT NOT NULL,
        target_agent_id TEXT NOT NULL,
        kind TEXT NOT NULL,
        priority INTEGER NOT NULL,
        content TEXT NOT NULL,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    "#,
)
.execute(pool)
.await?;
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS agent_messages (
        message_id TEXT PRIMARY KEY,
        workspace_id TEXT NOT NULL,
        conversation_id TEXT NOT NULL,
        from_agent_id TEXT NOT NULL,
        to_agent_id TEXT NOT NULL,
        content TEXT NOT NULL,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    "#,
)
.execute(pool)
.await?;
```

- [ ] **Step 4: Implement Agent Bus and patch store**

Replace `/Users/BaiGod/Documents/agent-ease/src/context.rs` with:

```rust
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct ContextPatch {
    pub patch_id: Uuid,
    pub kind: String,
    pub content: String,
    pub priority: i64,
}

#[derive(Clone)]
pub struct AgentBus {
    pool: SqlitePool,
}

impl AgentBus {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn send_message(
        &self,
        workspace_id: &str,
        conversation_id: &str,
        from_agent_id: &str,
        to_agent_id: &str,
        content: &str,
    ) -> Result<Uuid> {
        let message_id = Uuid::new_v4();
        let patch_id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            INSERT INTO agent_messages (
                message_id, workspace_id, conversation_id, from_agent_id,
                to_agent_id, content, status, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, 'pending', ?)
            "#,
        )
        .bind(message_id.to_string())
        .bind(workspace_id)
        .bind(conversation_id)
        .bind(from_agent_id)
        .bind(to_agent_id)
        .bind(content)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO context_patches (
                patch_id, workspace_id, conversation_id, target_agent_id,
                kind, priority, content, status, created_at
            )
            VALUES (?, ?, ?, ?, 'agent_message', 100, ?, 'pending', ?)
            "#,
        )
        .bind(patch_id.to_string())
        .bind(workspace_id)
        .bind(conversation_id)
        .bind(to_agent_id)
        .bind(format!("Message from {from_agent_id}: {content}"))
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(message_id)
    }
}

#[derive(Clone)]
pub struct ContextPatchStore {
    pool: SqlitePool,
}

impl ContextPatchStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn pending_for_agent(
        &self,
        workspace_id: &str,
        conversation_id: &str,
        agent_id: &str,
    ) -> Result<Vec<ContextPatch>> {
        let rows = sqlx::query(
            r#"
            SELECT patch_id, kind, content, priority
            FROM context_patches
            WHERE workspace_id = ? AND conversation_id = ? AND target_agent_id = ? AND status = 'pending'
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .bind(workspace_id)
        .bind(conversation_id)
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(ContextPatch {
                    patch_id: Uuid::parse_str(&row.try_get::<String, _>("patch_id")?).expect("valid patch uuid"),
                    kind: row.try_get("kind")?,
                    content: row.try_get("content")?,
                    priority: row.try_get("priority")?,
                })
            })
            .collect()
    }
}
```

- [ ] **Step 5: Run Agent Bus test**

Run:

```bash
cargo test --test multi_agent agent_message_becomes_context_patch -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/context.rs src/db.rs tests/multi_agent.rs
git commit -m "feat: add agent bus context patches"
```

Expected: commit succeeds.

## Task 10: Manual Compaction Engine

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/context.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/cli.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/agent_loop.rs`

- [ ] **Step 1: Add failing compaction test**

Append to `/Users/BaiGod/Documents/agent-ease/tests/agent_loop.rs`:

```rust
use agent_ease::context::CompactionEngine;

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
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test agent_loop compaction_preserves_current_work_sections -- --nocapture
```

Expected: FAIL because `CompactionEngine` is missing.

- [ ] **Step 3: Implement deterministic compaction formatter**

Append to `/Users/BaiGod/Documents/agent-ease/src/context.rs`:

```rust
pub struct CompactionEngine;

impl CompactionEngine {
    pub fn summarize_for_test(messages: &[&str]) -> String {
        let joined = messages.join("\\n");
        format!(
            "Summary:\\n\\n1. Primary Request and Intent:\\n{joined}\\n\\n2. Key Technical Concepts:\\n- Event-driven runtime\\n- SQLite checkpoints\\n\\n3. Files and Code Sections:\\n- Not available in deterministic test mode\\n\\n4. Errors and fixes:\\n- None recorded\\n\\n5. Problem Solving:\\n- Preserved chronological work context\\n\\n6. All user messages:\\n{joined}\\n\\n7. Pending Tasks:\\n{joined}\\n\\n8. Current Work:\\n{joined}\\n\\n9. Optional Next Step:\\nContinue the most recent pending task"
        )
    }
}
```

- [ ] **Step 4: Add CLI compact command skeleton**

Extend `/Users/BaiGod/Documents/agent-ease/src/cli.rs` command enum:

```rust
Compact {
    instructions: Vec<String>,
},
```

Add dispatch arm:

```rust
Command::Compact { instructions } => {
    let summary = crate::context::CompactionEngine::summarize_for_test(&[&instructions.join(" ")]);
    println!("{summary}");
}
```

- [ ] **Step 5: Run compaction test**

Run:

```bash
cargo test --test agent_loop compaction_preserves_current_work_sections -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/context.rs src/cli.rs tests/agent_loop.rs
git commit -m "feat: add manual compaction foundation"
```

Expected: commit succeeds.

## Task 11: Stdio MCP Bridge Foundation

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/mcp.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`

- [ ] **Step 1: Add failing MCP JSON-RPC shape test**

Append to `/Users/BaiGod/Documents/agent-ease/tests/tools.rs`:

```rust
use agent_ease::mcp::McpRequest;

#[test]
fn mcp_request_serializes_json_rpc() {
    let request = McpRequest::new(1, "tools/list", serde_json::json!({}));
    let value = serde_json::to_value(request).expect("serialize");
    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], 1);
    assert_eq!(value["method"], "tools/list");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test tools mcp_request_serializes_json_rpc -- --nocapture
```

Expected: FAIL because `McpRequest` is missing.

- [ ] **Step 3: Implement MCP request type**

Replace `/Users/BaiGod/Documents/agent-ease/src/mcp.rs` with:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Value,
}

impl McpRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}
```

- [ ] **Step 4: Run MCP shape test**

Run:

```bash
cargo test --test tools mcp_request_serializes_json_rpc -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/mcp.rs tests/tools.rs
git commit -m "feat: add stdio mcp request foundation"
```

Expected: commit succeeds.

## Task 12: CLI Status, Events, Doctor, And Acceptance Smoke

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/src/cli.rs`
- Modify: `/Users/BaiGod/Documents/agent-ease/src/db.rs`
- Test: `/Users/BaiGod/Documents/agent-ease/tests/cli_init.rs`

- [ ] **Step 1: Add failing CLI smoke tests**

Append to `/Users/BaiGod/Documents/agent-ease/tests/cli_init.rs`:

```rust
#[test]
fn doctor_reports_basic_checks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("agent").expect("agent binary exists");
    cmd.current_dir(temp.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("CLI: ok"));
}

#[test]
fn status_reports_workspace_after_init() {
    let temp = tempfile::tempdir().expect("tempdir");
    Command::cargo_bin("agent")
        .expect("agent binary exists")
        .current_dir(temp.path())
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("agent")
        .expect("agent binary exists")
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("workspace: agent-workspace"));
}
```

- [ ] **Step 2: Run smoke tests to verify failures**

Run:

```bash
cargo test --test cli_init doctor_reports_basic_checks status_reports_workspace_after_init -- --nocapture
```

Expected: FAIL because `doctor` and `status` output still use temporary messages.

- [ ] **Step 3: Implement status and doctor output**

Modify `/Users/BaiGod/Documents/agent-ease/src/cli.rs`:

```rust
Command::Status => {
    let cwd = std::env::current_dir()?;
    let config = crate::config::load_workspace_config(&cwd).await?;
    println!("workspace: {}", config.workspace_name);
    println!("default_agent: {}", config.default_agent);
    println!("default_provider: {}", config.default_provider);
}
Command::Doctor => {
    println!("CLI: ok");
    println!("SQLite: configured after agent init");
    println!("MCP: stdio transport planned");
    println!("Shadowbox: soft sandbox policy enabled after init");
}
```

Keep the existing arms for `Init`, `Chat`, `Run`, and `Events`.

- [ ] **Step 4: Run CLI smoke tests**

Run:

```bash
cargo test --test cli_init doctor_reports_basic_checks status_reports_workspace_after_init -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run full test suite**

Run:

```bash
cargo test
```

Expected: PASS for all tests.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/cli.rs src/db.rs tests/cli_init.rs
git commit -m "feat: add cli status and doctor smoke"
```

Expected: commit succeeds.

## Task 13: Final Verification Against Spec

**Files:**
- Modify: `/Users/BaiGod/Documents/agent-ease/docs/superpowers/specs/2026-06-06-agent-runtime-kernel-design.md` only if verification finds a spec wording mismatch.
- Modify: `/Users/BaiGod/Documents/agent-ease/docs/superpowers/plans/2026-06-06-agent-runtime-kernel-v0-1.md` only if a plan step is proven incorrect during execution.

- [ ] **Step 1: Run all tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 2: Run CLI smoke manually in a temp workspace**

Run:

```bash
tmpdir="$(mktemp -d)"
cargo run --bin agent --manifest-path /Users/BaiGod/Documents/agent-ease/Cargo.toml -- init --quiet 2>/dev/null || cargo run --bin agent --manifest-path /Users/BaiGod/Documents/agent-ease/Cargo.toml -- init
cd "$tmpdir"
/Users/BaiGod/Documents/agent-ease/target/debug/agent init
/Users/BaiGod/Documents/agent-ease/target/debug/agent status
/Users/BaiGod/Documents/agent-ease/target/debug/agent doctor
```

Expected output includes:

```text
initialized agent workspace
workspace: agent-workspace
CLI: ok
```

- [ ] **Step 3: Verify no accidental staging of `.agents` or `.codex`**

Run:

```bash
git status --short
```

Expected: `.agents/` and `.codex/` may remain untracked; implementation files should be committed or intentionally staged for the final commit. Do not stage `.agents/` or `.codex/` unless the user explicitly changes scope.

- [ ] **Step 4: Commit final docs correction if needed**

If Tasks 1-12 required a correction to this plan or the spec, run:

```bash
git add docs/superpowers/specs/2026-06-06-agent-runtime-kernel-design.md docs/superpowers/plans/2026-06-06-agent-runtime-kernel-v0-1.md
git commit -m "docs: align runtime plan with implementation"
```

Expected: commit succeeds only if docs changed. If docs did not change, skip this command.

## Coverage Review

This foundation plan covers the v0.1 spec as follows:

- CLI-only product: Tasks 1, 2, and 12.
- Workspace and `.agents/` config: Task 2.
- SQLite event log, checkpoint, and task queue: Tasks 3 and 4.
- Agent profiles: Task 2 creates config files; the completion plan loads and enforces profile-specific tools, skills, model, and policy.
- Single-agent loop: Task 5.
- Built-in file/search/bash tools: Tasks 6 and 7.
- Stale edit guard: Task 7.
- Skill progressive loading: Task 8.
- Internal Agent Bus and ContextPatch: Task 9.
- Manual compaction foundation: Task 10.
- Stdio MCP foundation: Task 11.
- CLI debug and doctor: Task 12.
- Multi-agent foundation and smoke: Tasks 9, 12, and 13.

The completion plan must cover the remaining v0.1 requirements after this foundation passes: real OpenAI-compatible and DashScope-compatible adapters, automatic and reactive compaction, scheduler worker loops, stdio MCP process lifecycle, profile enforcement, full `agent run`, `agent events`, `agent send`, and resume/debug commands.

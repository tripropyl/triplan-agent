use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use uuid::Uuid;

use crate::error::{AgentError, Result};
use crate::model::{CheckpointRecord, EventPayload, EventRecord, EventType, TaskRecord};

pub async fn connect_sqlite(database_url: &str) -> Result<SqlitePool> {
    let max_connections = if database_url.contains(":memory:") {
        1
    } else {
        5
    };
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await?;
    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(&pool)
        .await?;
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

    #[expect(
        clippy::too_many_arguments,
        reason = "event append mirrors immutable event log columns in this foundation slice"
    )]
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

        row.map(row_to_checkpoint).transpose()
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

    pub async fn lease_next(
        &self,
        worker_id: &str,
        lease_seconds: i64,
    ) -> Result<Option<TaskRecord>> {
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
        let update = sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'leased',
                lease_owner = ?,
                lease_expires_at = ?,
                attempt_count = attempt_count + 1
            WHERE task_id = ? AND status = 'pending'
            "#,
        )
        .bind(worker_id)
        .bind(expires.to_rfc3339())
        .bind(&task_id_text)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        if update.rows_affected() == 0 {
            return Ok(None);
        }

        let payload_json: String = row.try_get("payload_json")?;
        Ok(Some(TaskRecord {
            task_id: parse_uuid(task_id_text, "task_id")?,
            task_type: row.try_get("task_type")?,
            target_run_id: row.try_get("target_run_id")?,
            status: "leased".to_string(),
            priority: row.try_get("priority")?,
            lease_owner: Some(worker_id.to_string()),
            payload: serde_json::from_str(&payload_json)?,
        }))
    }
}

fn row_to_checkpoint(row: sqlx::sqlite::SqliteRow) -> Result<CheckpointRecord> {
    let projection_json: String = row.try_get("projection_json")?;
    let created_at: String = row.try_get("created_at")?;

    Ok(CheckpointRecord {
        checkpoint_id: parse_uuid(row.try_get("checkpoint_id")?, "checkpoint_id")?,
        workspace_id: row.try_get("workspace_id")?,
        run_id: row.try_get("run_id")?,
        after_sequence: row.try_get("after_sequence")?,
        projection: serde_json::from_str::<Value>(&projection_json)?,
        created_at: parse_datetime(&created_at, "checkpoint timestamp")?,
    })
}

fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<EventRecord> {
    let event_type_text: String = row.try_get("event_type")?;
    let event_type = event_type_text
        .parse::<EventType>()
        .map_err(AgentError::Runtime)?;
    let payload_json: String = row.try_get("payload_json")?;
    let created_at: String = row.try_get("created_at")?;

    Ok(EventRecord {
        event_id: parse_uuid(row.try_get("event_id")?, "event_id")?,
        workspace_id: row.try_get("workspace_id")?,
        conversation_id: row.try_get("conversation_id")?,
        run_id: row.try_get("run_id")?,
        agent_id: row.try_get("agent_id")?,
        event_type,
        payload: serde_json::from_str::<Value>(&payload_json)?,
        parent_event_id: parse_optional_uuid(row.try_get("parent_event_id")?, "parent_event_id")?,
        causation_id: parse_optional_uuid(row.try_get("causation_id")?, "causation_id")?,
        correlation_id: parse_uuid(row.try_get("correlation_id")?, "correlation_id")?,
        sequence: row.try_get("sequence")?,
        created_at: parse_datetime(&created_at, "event timestamp")?,
    })
}

fn parse_uuid(value: String, column: &str) -> Result<Uuid> {
    Uuid::parse_str(&value)
        .map_err(|err| AgentError::Runtime(format!("invalid {column} uuid: {err}")))
}

fn parse_optional_uuid(value: Option<String>, column: &str) -> Result<Option<Uuid>> {
    value.map(|text| parse_uuid(text, column)).transpose()
}

fn parse_datetime(value: &str, label: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|err| AgentError::Runtime(format!("invalid {label}: {err}")))
}

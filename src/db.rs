use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use uuid::Uuid;

use crate::error::{AgentError, Result};
use crate::model::{EventPayload, EventRecord, EventType};

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
    let event_type = EventType::from_str(&event_type_text).ok_or_else(|| {
        AgentError::Runtime(format!("unknown event type in database: {event_type_text}"))
    })?;
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
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map_err(|err| AgentError::Runtime(format!("invalid event timestamp: {err}")))?
            .with_timezone(&Utc),
    })
}

fn parse_uuid(value: String, column: &str) -> Result<Uuid> {
    Uuid::parse_str(&value)
        .map_err(|err| AgentError::Runtime(format!("invalid {column} uuid: {err}")))
}

fn parse_optional_uuid(value: Option<String>, column: &str) -> Result<Option<Uuid>> {
    value.map(|text| parse_uuid(text, column)).transpose()
}

use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{AgentError, Result};

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
            WHERE workspace_id = ?
              AND conversation_id = ?
              AND target_agent_id = ?
              AND status = 'pending'
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
                let patch_id_text: String = row.try_get("patch_id")?;
                Ok(ContextPatch {
                    patch_id: Uuid::parse_str(&patch_id_text).map_err(|err| {
                        AgentError::Runtime(format!("invalid patch_id uuid: {err}"))
                    })?,
                    kind: row.try_get("kind")?,
                    content: row.try_get("content")?,
                    priority: row.try_get("priority")?,
                })
            })
            .collect()
    }
}

pub struct CompactionEngine;

impl CompactionEngine {
    pub fn summarize_for_test(messages: &[&str]) -> String {
        let joined = messages.join("\n");
        format!(
            "Summary:\n\n\
             1. Primary Request and Intent:\n{joined}\n\n\
             2. Key Technical Concepts:\n- Event-driven runtime\n- SQLite checkpoints\n\n\
             3. Files and Code Sections:\n- Not available in deterministic test mode\n\n\
             4. Errors and fixes:\n- None recorded\n\n\
             5. Problem Solving:\n- Preserved chronological work context\n\n\
             6. All user messages:\n{joined}\n\n\
             7. Pending Tasks:\n{joined}\n\n\
             8. Current Work:\n{joined}\n\n\
             9. Optional Next Step:\nContinue the most recent pending task"
        )
    }
}

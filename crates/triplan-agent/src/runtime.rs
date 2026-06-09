use serde_json::json;
use std::future::Future;

use crate::db::{EventStore, TaskStore};
use crate::error::Result;
use crate::model::{EventPayload, EventType, TaskRecord};
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

#[derive(Debug, Clone, Copy, Default)]
pub struct SchedulerStats {
    pub completed: i64,
}

#[derive(Clone)]
pub struct SchedulerWorker {
    tasks: TaskStore,
    worker_id: String,
    lease_seconds: i64,
}

impl SchedulerWorker {
    pub fn new(tasks: TaskStore, worker_id: impl Into<String>) -> Self {
        Self {
            tasks,
            worker_id: worker_id.into(),
            lease_seconds: 30,
        }
    }

    pub async fn drain_available<F, Fut>(&self, handler: F) -> Result<SchedulerStats>
    where
        F: Fn(TaskRecord) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let mut stats = SchedulerStats::default();
        while let Some(task) = self
            .tasks
            .lease_next(&self.worker_id, self.lease_seconds)
            .await?
        {
            let task_id = task.task_id;
            handler(task).await?;
            self.tasks.complete(task_id).await?;
            stats.completed += 1;
        }
        Ok(stats)
    }
}

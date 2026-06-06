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

    pub fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "user_message" => Self::UserMessage,
            "run_requested" => Self::RunRequested,
            "run_started" => Self::RunStarted,
            "model_request_started" => Self::ModelRequestStarted,
            "assistant_message" => Self::AssistantMessage,
            "tool_call_requested" => Self::ToolCallRequested,
            "tool_call_completed" => Self::ToolCallCompleted,
            "agent_message_sent" => Self::AgentMessageSent,
            "context_patch_created" => Self::ContextPatchCreated,
            "compact_requested" => Self::CompactRequested,
            "compact_started" => Self::CompactStarted,
            "compact_completed" => Self::CompactCompleted,
            "compact_failed" => Self::CompactFailed,
            "run_paused" => Self::RunPaused,
            "run_resumed" => Self::RunResumed,
            "run_cancelled" => Self::RunCancelled,
            "run_completed" => Self::RunCompleted,
            "run_failed" => Self::RunFailed,
            _ => return None,
        })
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

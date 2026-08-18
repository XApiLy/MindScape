use serde::{Deserialize, Serialize};

use crate::domain::{ContextSnapshot, RunState};

pub const RUNTIME_CONTRACT_VERSION: &str = "mindscape.runtime.v1";
pub const APPLICATION_INTERRUPTED_PROVIDER_CODE: &str = "application_interrupted";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityRequirement {
    TextInput,
    ImageInput,
    ToolCalling,
    UsageReporting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRunBudget {
    pub max_output_tokens: Option<u64>,
    pub max_cost_microunits: Option<u64>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRunRequest {
    pub contract_version: String,
    pub run_id: String,
    pub conversation_id: String,
    pub node_id: String,
    pub context_snapshot: ContextSnapshot,
    pub provider_id: String,
    pub model_id: String,
    pub capabilities: Vec<CapabilityRequirement>,
    pub budget: ModelRunBudget,
    pub idempotency_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub cost_microunits: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProviderErrorCategory {
    Authentication,
    RateLimit,
    InsufficientBalance,
    ModelUnavailable,
    InvalidRequest,
    Network,
    Timeout,
    ContentPolicy,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderError {
    pub category: ProviderErrorCategory,
    pub provider_code: Option<String>,
    pub safe_message: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
    pub provider_status: Option<u16>,
}

impl ProviderError {
    pub fn application_interrupted() -> Self {
        Self {
            category: ProviderErrorCategory::Unknown,
            provider_code: Some(APPLICATION_INTERRUPTED_PROVIDER_CODE.into()),
            safe_message: "MindScape exited before this response finished.".into(),
            retryable: true,
            retry_after_ms: None,
            provider_status: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FinishReason {
    Stop,
    Length,
    ContentPolicy,
    ToolCall,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RunCancelReason {
    UserRequested,
    Timeout,
    ApplicationShutdown,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ModelRunEvent {
    Started,
    TextDelta {
        delta: String,
    },
    UsageUpdated {
        usage: ModelUsage,
    },
    Completed {
        finish_reason: FinishReason,
        usage: ModelUsage,
    },
    Cancelled {
        reason: RunCancelReason,
        partial_content_retained: bool,
    },
    Failed {
        error: ProviderError,
        partial_content_retained: bool,
    },
}

impl ModelRunEvent {
    pub fn resulting_state(&self) -> RunState {
        match self {
            Self::Started | Self::TextDelta { .. } | Self::UsageUpdated { .. } => {
                RunState::Streaming
            }
            Self::Completed { .. } => RunState::Completed,
            Self::Cancelled { .. } => RunState::Cancelled,
            Self::Failed { .. } => RunState::Failed,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Cancelled { .. } | Self::Failed { .. }
        )
    }

    pub fn application_interrupted(partial_content_retained: bool) -> Self {
        Self::Failed {
            error: ProviderError::application_interrupted(),
            partial_content_retained,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRunEventEnvelope {
    pub contract_version: String,
    pub event_id: String,
    pub run_id: String,
    pub node_id: String,
    pub sequence: u64,
    pub occurred_at: String,
    pub event: ModelRunEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRunProjection {
    pub run_id: String,
    pub conversation_id: String,
    pub node_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub state: RunState,
    pub last_sequence: u64,
    pub partial_content: String,
    pub terminal_event: Option<ModelRunEvent>,
    pub updated_at: String,
}

use serde::{Deserialize, Serialize};

pub const EVENT_CONTRACT_VERSION: &str = "mindscape.event.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AggregateType {
    Workspace,
    Conversation,
    ImportSource,
    ModelRun,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DomainEventType {
    WorkspaceCreated,
    ConversationCreated,
    ConversationRenamed,
    TurnAppended,
    TurnCompleted,
    NodePositionUpdated,
    ModelRunCreated,
    ModelRunEventRecorded,
    ImportSourceCreated,
    ImportRevisionCreated,
    ContinuationCreated,
    ContinuationInvalidated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DomainEventEnvelope {
    pub contract_version: String,
    pub event_id: String,
    pub sequence: Option<u64>,
    pub aggregate_type: AggregateType,
    pub aggregate_id: String,
    pub event_type: DomainEventType,
    pub payload: serde_json::Value,
    pub idempotency_key: Option<String>,
    pub occurred_at: String,
}

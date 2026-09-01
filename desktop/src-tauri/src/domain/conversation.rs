use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use super::{ContentBlock, ContextSnapshot, KernelError, KernelResult};

pub const SCHEMA_VERSION: i64 = 16;

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

pub fn now_timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BranchType {
    Continues,
    Deepens,
    Diverges,
    Reframes,
    ImportedFrom,
}

impl BranchType {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Continues => "continues",
            Self::Deepens => "deepens",
            Self::Diverges => "diverges",
            Self::Reframes => "reframes",
            Self::ImportedFrom => "imported_from",
        }
    }
}

impl FromStr for BranchType {
    type Err = KernelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "continues" => Ok(Self::Continues),
            "deepens" => Ok(Self::Deepens),
            "diverges" => Ok(Self::Diverges),
            "reframes" => Ok(Self::Reframes),
            "imported_from" => Ok(Self::ImportedFrom),
            other => Err(KernelError::Integrity(format!(
                "unknown branch type: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunState {
    Pending,
    Streaming,
    Completed,
    Cancelled,
    Failed,
}

impl RunState {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Streaming => "streaming",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for RunState {
    type Err = KernelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "streaming" => Ok(Self::Streaming),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            "failed" => Ok(Self::Failed),
            other => Err(KernelError::Integrity(format!(
                "unknown run state: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Imported,
}

impl MessageRole {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Imported => "imported",
        }
    }
}

impl FromStr for MessageRole {
    type Err = KernelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "system" => Ok(Self::System),
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "imported" => Ok(Self::Imported),
            other => Err(KernelError::Integrity(format!(
                "unknown message role: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    #[serde(flatten)]
    pub conversation: Conversation,
    pub node_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub node_id: String,
    pub role: MessageRole,
    pub content_blocks: Vec<ContentBlock>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationNode {
    pub id: String,
    pub conversation_id: String,
    pub parent_node_id: Option<String>,
    pub branch_type: BranchType,
    pub title: String,
    pub user_message: Message,
    pub assistant_message: Option<Message>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub context_snapshot_id: String,
    pub run_state: RunState,
    pub created_at: String,
    pub updated_at: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationEdge {
    pub id: String,
    pub conversation_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation: BranchType,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasNodePosition {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasViewportState {
    pub conversation_id: String,
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveCanvasViewportInput {
    pub conversation_id: String,
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl SaveCanvasViewportInput {
    pub fn validate(&self) -> KernelResult<()> {
        if self.conversation_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "conversation ID is required".into(),
            ));
        }
        if !self.x.is_finite() || !self.y.is_finite() || !self.zoom.is_finite() || self.zoom <= 0.0
        {
            return Err(KernelError::Validation(
                "canvas viewport values must be finite and zoom must be positive".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationGraph {
    pub conversation: Conversation,
    pub nodes: Vec<ConversationNode>,
    pub edges: Vec<ConversationEdge>,
    pub positions: Vec<CanvasNodePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KernelBootstrap {
    pub schema_version: i64,
    pub database_path: String,
    pub workspace: Workspace,
    pub conversations: Vec<ConversationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConversationInput {
    pub workspace_id: String,
    pub title: String,
}

impl CreateConversationInput {
    pub fn validate(&self) -> KernelResult<()> {
        if self.workspace_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "workspace id cannot be empty".into(),
            ));
        }
        if self.title.trim().is_empty() {
            return Err(KernelError::Validation(
                "conversation title cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendTurnInput {
    pub conversation_id: String,
    pub parent_node_id: Option<String>,
    pub branch_type: BranchType,
    pub title: String,
    pub prompt: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartModelRunInput {
    pub conversation_id: String,
    pub parent_node_id: Option<String>,
    pub branch_type: BranchType,
    pub title: String,
    pub prompt: String,
    pub provider_id: String,
    pub model_id: String,
    pub capabilities: Vec<crate::domain::contracts::CapabilityRequirement>,
    pub budget: crate::domain::contracts::ModelRunBudget,
    #[serde(default)]
    pub effective_run_profile: Option<crate::domain::contracts::EffectiveRunProfile>,
    pub idempotency_key: String,
}

impl StartModelRunInput {
    pub fn validate(&self) -> KernelResult<()> {
        if self.provider_id.trim().is_empty() || self.model_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "provider and model are required".into(),
            ));
        }
        if self.idempotency_key.trim().is_empty() {
            return Err(KernelError::Validation(
                "idempotency key is required".into(),
            ));
        }
        AppendTurnInput {
            conversation_id: self.conversation_id.clone(),
            parent_node_id: self.parent_node_id.clone(),
            branch_type: self.branch_type,
            title: self.title.clone(),
            prompt: self.prompt.clone(),
            provider_id: Some(self.provider_id.clone()),
            model_id: Some(self.model_id.clone()),
        }
        .validate()
    }
}

impl AppendTurnInput {
    pub fn validate(&self) -> KernelResult<()> {
        if self.conversation_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "conversation id cannot be empty".into(),
            ));
        }
        if self.title.trim().is_empty() {
            return Err(KernelError::Validation("node title cannot be empty".into()));
        }
        if self.prompt.trim().is_empty() {
            return Err(KernelError::Validation("prompt cannot be empty".into()));
        }
        if self.parent_node_id.is_none() && self.branch_type != BranchType::Continues {
            return Err(KernelError::Validation(
                "a root turn must use the continues relation".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteTurnInput {
    pub node_id: String,
    pub content: String,
    pub provider_id: String,
    pub model_id: String,
}

impl CompleteTurnInput {
    pub fn validate(&self) -> KernelResult<()> {
        if self.node_id.trim().is_empty()
            || self.content.trim().is_empty()
            || self.provider_id.trim().is_empty()
            || self.model_id.trim().is_empty()
        {
            return Err(KernelError::Validation(
                "node, content, provider, and model are required".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNodePositionInput {
    pub conversation_id: String,
    pub node_id: String,
    pub x: f64,
    pub y: f64,
}

impl UpdateNodePositionInput {
    pub fn validate(&self) -> KernelResult<()> {
        if self.conversation_id.trim().is_empty() || self.node_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "conversation and node ids are required".into(),
            ));
        }
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(KernelError::Validation(
                "canvas coordinates must be finite".into(),
            ));
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn _assert_context_is_serializable(_: &ContextSnapshot) {}

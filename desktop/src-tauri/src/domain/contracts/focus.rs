use serde::{Deserialize, Serialize};

pub const FOCUS_CONTRACT_VERSION: &str = "mindscape.focus.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FocusContextPolicy {
    ContinueCurrent,
    FocusNew,
    BranchFromNode,
    ContinueImportedRaw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FocusBranchKind {
    Mainline,
    Exploration,
    Task,
    Retrospective,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusMemoryScope {
    pub branch_kind: FocusBranchKind,
    pub inherit_refs: Vec<String>,
    pub local_refs: Vec<String>,
    pub exclude_refs: Vec<String>,
    pub promote_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusFrame {
    pub contract_version: String,
    pub id: String,
    pub conversation_id: String,
    pub parent_node_id: Option<String>,
    pub objective: String,
    pub active_work_item: Option<String>,
    pub context_policy: FocusContextPolicy,
    pub memory_scope: FocusMemoryScope,
    pub include_refs: Vec<String>,
    pub exclude_refs: Vec<String>,
    pub memory_version: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmittedFocusRef {
    pub reference_id: String,
    pub reason: String,
}

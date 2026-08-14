use serde::{Deserialize, Serialize};

use crate::domain::{ContentBlock, MessageRole};

use super::EvidenceRef;

pub const IMPORT_CONTRACT_VERSION: &str = "mindscape.import.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportPlatform {
    ChatGpt,
    Claude,
    Codex,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportSource {
    pub id: String,
    pub conversation_id: String,
    pub platform: ImportPlatform,
    pub original_file_name: Option<String>,
    pub content_hash: String,
    pub storage_ref: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImportRevisionStatus {
    Parsing,
    Parsed,
    PartiallyParsed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportRevision {
    pub id: String,
    pub import_source_id: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub status: ImportRevisionStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryStatus {
    Recovered,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FieldRecovery {
    pub field: String,
    pub status: RecoveryStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportIssue {
    pub code: String,
    pub message: String,
    pub source_locator: Option<String>,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParseReport {
    pub import_revision_id: String,
    pub conversation_count: u64,
    pub message_count: u64,
    pub attachment_count: u64,
    pub tool_record_count: u64,
    pub field_recovery: Vec<FieldRecovery>,
    pub warnings: Vec<ImportIssue>,
    pub errors: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedMessage {
    pub id: String,
    pub import_revision_id: String,
    pub role: MessageRole,
    pub content_blocks: Vec<ContentBlock>,
    pub occurred_at: Option<String>,
    pub source_locator: String,
    pub parent_imported_message_id: Option<String>,
    pub platform_extension: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisMode {
    Quick,
    Detailed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContinuationStatus {
    Active,
    Superseded,
    Invalidated,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationClaim {
    pub id: String,
    pub kind: String,
    pub value: String,
    pub evidence: Vec<EvidenceRef>,
    pub user_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DerivedContinuation {
    pub id: String,
    pub import_source_id: String,
    pub import_revision_id: String,
    pub revision: u64,
    pub analysis_mode: AnalysisMode,
    pub generator_id: String,
    pub generator_version: String,
    pub status: ContinuationStatus,
    pub claims: Vec<ContinuationClaim>,
    pub created_at: String,
}

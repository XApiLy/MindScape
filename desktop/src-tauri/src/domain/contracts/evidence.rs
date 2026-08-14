use serde::{Deserialize, Serialize};

pub const EVIDENCE_CONTRACT_VERSION: &str = "mindscape.evidence.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EvidenceTarget {
    MessageBlock {
        message_id: String,
        content_block_index: u32,
    },
    ImportContent {
        import_source_id: String,
        import_revision_id: String,
        locator: String,
    },
    AttachmentContent {
        attachment_id: String,
        locator: Option<String>,
    },
    ToolResultBlock {
        tool_run_id: String,
        content_block_index: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRef {
    pub id: String,
    pub target: EvidenceTarget,
    pub content_hash: Option<String>,
    pub excerpt: Option<String>,
    pub created_at: String,
}

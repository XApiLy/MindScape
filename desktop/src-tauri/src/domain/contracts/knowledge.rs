use serde::{Deserialize, Serialize};

use super::EvidenceRef;

pub const KNOWLEDGE_CONTRACT_VERSION: &str = "mindscape.knowledge.v1";
pub const MARKDOWN_PROJECTION_CONTRACT_VERSION: &str = "mindscape.markdown-projection.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeEntityKind {
    Goal,
    Decision,
    Constraint,
    Question,
    Source,
    Project,
    Topic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeStatus {
    Candidate,
    Inferred,
    Confirmed,
    Rejected,
    Superseded,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum KnowledgeScope {
    Workspace {
        workspace_id: String,
    },
    Project {
        workspace_id: String,
        project_id: String,
    },
    Conversation {
        workspace_id: String,
        conversation_id: String,
    },
    FocusFrame {
        workspace_id: String,
        conversation_id: String,
        focus_frame_id: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GeneratorKind {
    User,
    DeterministicRule,
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorRef {
    pub kind: GeneratorKind,
    pub generator_id: String,
    pub generator_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopedEvidenceRef {
    pub id: String,
    pub evidence: EvidenceRef,
    pub scope: KnowledgeScope,
    pub status: KnowledgeStatus,
    pub revision: u64,
    pub generator: GeneratorRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEntity {
    pub contract_version: String,
    pub id: String,
    pub kind: KnowledgeEntityKind,
    pub name: String,
    pub aliases: Vec<String>,
    pub scope: KnowledgeScope,
    pub status: KnowledgeStatus,
    pub revision: u64,
    pub evidence: Vec<ScopedEvidenceRef>,
    pub generator: GeneratorRef,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeRelationKind {
    Mentions,
    BelongsTo,
    Supports,
    Contradicts,
    DependsOn,
    DerivedFrom,
    Supersedes,
    RelatedTo,
    ContinuedBy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRelation {
    pub contract_version: String,
    pub id: String,
    pub kind: KnowledgeRelationKind,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub scope: KnowledgeScope,
    pub status: KnowledgeStatus,
    pub revision: u64,
    pub evidence: Vec<ScopedEvidenceRef>,
    pub generator: GeneratorRef,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownProjection {
    pub contract_version: String,
    pub id: String,
    pub target_entity_id: String,
    pub relative_path: String,
    pub entity_revision: u64,
    pub projection_revision: u64,
    pub content_hash: String,
    pub frontmatter_version: String,
    pub created_at: String,
}

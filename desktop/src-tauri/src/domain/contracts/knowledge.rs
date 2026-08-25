use serde::{Deserialize, Serialize};

use crate::domain::{KernelError, KernelResult};

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

impl KnowledgeScope {
    pub fn validate(&self) -> KernelResult<()> {
        match self {
            Self::Workspace { workspace_id } => {
                require_non_empty(workspace_id, "KnowledgeScope workspace id")
            }
            Self::Project {
                workspace_id,
                project_id,
            } => {
                require_non_empty(workspace_id, "KnowledgeScope workspace id")?;
                require_non_empty(project_id, "KnowledgeScope project id")
            }
            Self::Conversation {
                workspace_id,
                conversation_id,
            } => {
                require_non_empty(workspace_id, "KnowledgeScope workspace id")?;
                require_non_empty(conversation_id, "KnowledgeScope conversation id")
            }
            Self::FocusFrame {
                workspace_id,
                conversation_id,
                focus_frame_id,
            } => {
                require_non_empty(workspace_id, "KnowledgeScope workspace id")?;
                require_non_empty(conversation_id, "KnowledgeScope conversation id")?;
                require_non_empty(focus_frame_id, "KnowledgeScope FocusFrame id")
            }
        }
    }

    pub fn validate_for_conversation(&self, conversation_id: &str) -> KernelResult<()> {
        require_non_empty(conversation_id, "conversation id")?;
        self.validate()?;
        match self {
            Self::Conversation {
                conversation_id: scoped_conversation_id,
                ..
            }
            | Self::FocusFrame {
                conversation_id: scoped_conversation_id,
                ..
            } if scoped_conversation_id != conversation_id => Err(KernelError::Integrity(
                "knowledge scope belongs to a different conversation".into(),
            )),
            _ => Ok(()),
        }
    }
}

impl GeneratorRef {
    pub fn validate(&self) -> KernelResult<()> {
        require_non_empty(&self.generator_id, "GeneratorRef id")?;
        require_non_empty(&self.generator_version, "GeneratorRef version")
    }
}

impl ScopedEvidenceRef {
    pub fn validate(&self) -> KernelResult<()> {
        require_non_empty(&self.id, "ScopedEvidenceRef id")?;
        self.evidence.validate()?;
        self.scope.validate()?;
        if self.revision == 0 {
            return Err(KernelError::Validation(
                "ScopedEvidenceRef revision must be greater than zero".into(),
            ));
        }
        self.generator.validate()
    }
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

impl KnowledgeEntity {
    pub fn validate(&self) -> KernelResult<()> {
        if self.contract_version != KNOWLEDGE_CONTRACT_VERSION {
            return Err(KernelError::Validation(format!(
                "unsupported KnowledgeEntity contract version: {}",
                self.contract_version
            )));
        }
        require_non_empty(&self.id, "KnowledgeEntity id")?;
        require_non_empty(&self.name, "KnowledgeEntity name")?;
        self.scope.validate()?;
        if self.revision == 0 {
            return Err(KernelError::Validation(
                "KnowledgeEntity revision must be greater than zero".into(),
            ));
        }
        self.generator.validate()?;
        validate_timestamps(&self.created_at, &self.updated_at, "KnowledgeEntity")?;
        validate_scoped_evidence(&self.evidence)
    }

    pub fn validate_for_conversation(&self, conversation_id: &str) -> KernelResult<()> {
        self.validate()?;
        self.scope.validate_for_conversation(conversation_id)?;
        for evidence in &self.evidence {
            evidence.scope.validate_for_conversation(conversation_id)?;
        }
        Ok(())
    }
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

impl KnowledgeRelation {
    pub fn validate(&self) -> KernelResult<()> {
        if self.contract_version != KNOWLEDGE_CONTRACT_VERSION {
            return Err(KernelError::Validation(format!(
                "unsupported KnowledgeRelation contract version: {}",
                self.contract_version
            )));
        }
        require_non_empty(&self.id, "KnowledgeRelation id")?;
        require_non_empty(&self.source_entity_id, "KnowledgeRelation source entity id")?;
        require_non_empty(&self.target_entity_id, "KnowledgeRelation target entity id")?;
        if self.source_entity_id == self.target_entity_id {
            return Err(KernelError::Validation(
                "KnowledgeRelation source and target must differ".into(),
            ));
        }
        self.scope.validate()?;
        if self.revision == 0 {
            return Err(KernelError::Validation(
                "KnowledgeRelation revision must be greater than zero".into(),
            ));
        }
        self.generator.validate()?;
        validate_timestamps(&self.created_at, &self.updated_at, "KnowledgeRelation")?;
        validate_scoped_evidence(&self.evidence)
    }

    pub fn validate_for_conversation(&self, conversation_id: &str) -> KernelResult<()> {
        self.validate()?;
        self.scope.validate_for_conversation(conversation_id)?;
        for evidence in &self.evidence {
            evidence.scope.validate_for_conversation(conversation_id)?;
        }
        Ok(())
    }
}

fn validate_scoped_evidence(evidence: &[ScopedEvidenceRef]) -> KernelResult<()> {
    let mut ids = std::collections::HashSet::new();
    for reference in evidence {
        reference.validate()?;
        if !ids.insert(reference.id.as_str()) {
            return Err(KernelError::Validation(format!(
                "duplicate ScopedEvidenceRef id: {}",
                reference.id
            )));
        }
    }
    Ok(())
}

fn validate_timestamps(created_at: &str, updated_at: &str, kind: &str) -> KernelResult<()> {
    require_non_empty(created_at, &format!("{kind} created at"))?;
    require_non_empty(updated_at, &format!("{kind} updated at"))
}

fn require_non_empty(value: &str, field: &str) -> KernelResult<()> {
    if value.trim().is_empty() {
        return Err(KernelError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generator() -> GeneratorRef {
        GeneratorRef {
            kind: GeneratorKind::User,
            generator_id: "user".into(),
            generator_version: "v1".into(),
        }
    }

    fn entity() -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "entity-1".into(),
            kind: KnowledgeEntityKind::Decision,
            name: "Use FocusFrame".into(),
            aliases: vec![],
            scope: KnowledgeScope::Conversation {
                workspace_id: "workspace-1".into(),
                conversation_id: "conversation-1".into(),
            },
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: generator(),
            created_at: "2026-08-26T00:00:00Z".into(),
            updated_at: "2026-08-26T00:00:00Z".into(),
        }
    }

    #[test]
    fn validates_entity_scope_revision_and_generator() {
        entity().validate().expect("valid entity");

        let mut invalid = entity();
        invalid.scope = KnowledgeScope::Project {
            workspace_id: "workspace-1".into(),
            project_id: " ".into(),
        };
        let error = invalid.validate().expect_err("blank project scope");
        assert!(error.to_string().contains("project id"));
    }

    #[test]
    fn rejects_unknown_contract_zero_revision_and_duplicate_scoped_evidence() {
        let mut invalid = entity();
        invalid.contract_version = "mindscape.knowledge.v0".into();
        let error = invalid.validate().expect_err("unknown contract");
        assert!(error.to_string().contains("unsupported KnowledgeEntity"));

        let mut invalid = entity();
        invalid.revision = 0;
        let error = invalid.validate().expect_err("zero revision");
        assert!(
            error
                .to_string()
                .contains("revision must be greater than zero")
        );

        let mut invalid = entity();
        let evidence = ScopedEvidenceRef {
            id: "scoped-1".into(),
            evidence: EvidenceRef {
                id: "evidence-1".into(),
                target: crate::domain::contracts::EvidenceTarget::MessageBlock {
                    message_id: "message-1".into(),
                    content_block_index: 0,
                },
                content_hash: None,
                excerpt: None,
                created_at: "2026-08-26T00:00:00Z".into(),
            },
            scope: KnowledgeScope::Conversation {
                workspace_id: "workspace-1".into(),
                conversation_id: "conversation-1".into(),
            },
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: generator(),
        };
        invalid.evidence = vec![evidence.clone(), evidence];
        let error = invalid.validate().expect_err("duplicate evidence");
        assert!(error.to_string().contains("duplicate ScopedEvidenceRef"));
    }

    #[test]
    fn rejects_self_relations_before_indexing() {
        let relation = KnowledgeRelation {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "relation-1".into(),
            kind: KnowledgeRelationKind::RelatedTo,
            source_entity_id: "entity-1".into(),
            target_entity_id: "entity-1".into(),
            scope: KnowledgeScope::Workspace {
                workspace_id: "workspace-1".into(),
            },
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: generator(),
            created_at: "2026-08-26T00:00:00Z".into(),
            updated_at: "2026-08-26T00:00:00Z".into(),
        };
        let error = relation.validate().expect_err("self relation");
        assert!(error.to_string().contains("source and target must differ"));
    }

    #[test]
    fn rejects_conversation_scoped_objects_from_another_conversation() {
        let entity = entity();
        let error = entity
            .validate_for_conversation("conversation-2")
            .expect_err("cross-conversation entity");
        assert!(error.to_string().contains("different conversation"));

        let error = KnowledgeScope::Workspace {
            workspace_id: "workspace-1".into(),
        }
        .validate_for_conversation("conversation-2");
        assert!(error.is_ok());
    }
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

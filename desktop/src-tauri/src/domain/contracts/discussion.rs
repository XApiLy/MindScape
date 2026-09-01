use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::{KernelError, KernelResult};

use super::EvidenceRef;

pub const DISCUSSION_LOG_CONTRACT_VERSION: &str = "mindscape.discussion-log.v1";
pub const DISCUSSION_LOG_PROJECTION_CONTRACT_VERSION: &str =
    "mindscape.discussion-log-projection.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DiscussionLogScope {
    Project {
        workspace_id: String,
        project_id: String,
    },
    Conversation {
        workspace_id: String,
        conversation_id: String,
        focus_frame_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionLog {
    pub contract_version: String,
    pub id: String,
    pub scope: DiscussionLogScope,
    pub title: String,
    pub body_markdown: String,
    pub related_entity_ids: Vec<String>,
    pub evidence: Vec<EvidenceRef>,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionLogProjection {
    pub contract_version: String,
    pub log: DiscussionLog,
    pub relative_path: String,
    pub content_hash: String,
}

impl DiscussionLogScope {
    pub fn validate(&self) -> KernelResult<()> {
        match self {
            Self::Project {
                workspace_id,
                project_id,
            } => {
                require_non_empty(workspace_id, "DiscussionLog workspace id")?;
                require_non_empty(project_id, "DiscussionLog project id")
            }
            Self::Conversation {
                workspace_id,
                conversation_id,
                focus_frame_id,
            } => {
                require_non_empty(workspace_id, "DiscussionLog workspace id")?;
                require_non_empty(conversation_id, "DiscussionLog conversation id")?;
                if focus_frame_id
                    .as_deref()
                    .is_some_and(|value| value.trim().is_empty())
                {
                    return Err(KernelError::Validation(
                        "DiscussionLog FocusFrame id must not be empty".into(),
                    ));
                }
                Ok(())
            }
        }
    }
}

impl DiscussionLog {
    pub fn validate(&self) -> KernelResult<()> {
        if self.contract_version != DISCUSSION_LOG_CONTRACT_VERSION {
            return Err(KernelError::Validation(format!(
                "unsupported DiscussionLog contract version: {}",
                self.contract_version
            )));
        }
        require_non_empty(&self.id, "DiscussionLog id")?;
        require_non_empty(&self.title, "DiscussionLog title")?;
        require_non_empty(&self.body_markdown, "DiscussionLog body")?;
        require_non_empty(&self.created_at, "DiscussionLog created at")?;
        require_non_empty(&self.updated_at, "DiscussionLog updated at")?;
        self.scope.validate()?;
        if self.revision == 0 {
            return Err(KernelError::Validation(
                "DiscussionLog revision must be greater than zero".into(),
            ));
        }
        if self.evidence.is_empty() {
            return Err(KernelError::Validation(
                "DiscussionLog requires at least one EvidenceRef".into(),
            ));
        }
        let mut evidence_ids = HashSet::new();
        for evidence in &self.evidence {
            evidence.validate()?;
            if !evidence_ids.insert(evidence.id.as_str()) {
                return Err(KernelError::Validation(format!(
                    "duplicate DiscussionLog EvidenceRef id: {}",
                    evidence.id
                )));
            }
        }
        let mut entity_ids = HashSet::new();
        for entity_id in &self.related_entity_ids {
            require_non_empty(entity_id, "DiscussionLog related entity id")?;
            if !entity_ids.insert(entity_id.as_str()) {
                return Err(KernelError::Validation(format!(
                    "duplicate DiscussionLog related entity id: {entity_id}"
                )));
            }
        }
        Ok(())
    }
}

impl DiscussionLogProjection {
    pub fn validate(&self) -> KernelResult<()> {
        if self.contract_version != DISCUSSION_LOG_PROJECTION_CONTRACT_VERSION {
            return Err(KernelError::Validation(format!(
                "unsupported DiscussionLog projection contract version: {}",
                self.contract_version
            )));
        }
        self.log.validate()?;
        if !self.relative_path.starts_with("logs/discussions/")
            || !self.relative_path.ends_with(".md")
            || self.relative_path.contains("..")
        {
            return Err(KernelError::Validation(
                "DiscussionLog projection path must stay below logs/discussions".into(),
            ));
        }
        if self.content_hash.len() != 64
            || !self
                .content_hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(KernelError::Validation(
                "DiscussionLog projection requires a SHA-256 content hash".into(),
            ));
        }
        Ok(())
    }
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
    use crate::domain::contracts::EvidenceTarget;

    fn log() -> DiscussionLog {
        DiscussionLog {
            contract_version: DISCUSSION_LOG_CONTRACT_VERSION.into(),
            id: "discussion-1".into(),
            scope: DiscussionLogScope::Conversation {
                workspace_id: "workspace-1".into(),
                conversation_id: "conversation-1".into(),
                focus_frame_id: Some("focus-1".into()),
            },
            title: "SQLite decision".into(),
            body_markdown: "## Objective\n\nKeep local state durable.".into(),
            related_entity_ids: vec!["entity-1".into()],
            evidence: vec![EvidenceRef {
                id: "evidence-1".into(),
                target: EvidenceTarget::MessageBlock {
                    message_id: "message-1".into(),
                    content_block_index: 0,
                },
                content_hash: None,
                excerpt: Some("Use SQLite".into()),
                created_at: "2026-08-30T00:00:00Z".into(),
            }],
            revision: 1,
            created_at: "2026-08-30T00:00:00Z".into(),
            updated_at: "2026-08-30T00:00:00Z".into(),
        }
    }

    #[test]
    fn validates_traceable_conversation_log() {
        log().validate().expect("valid discussion log");
    }

    #[test]
    fn rejects_log_without_evidence() {
        let mut invalid = log();
        invalid.evidence.clear();
        let error = invalid.validate().expect_err("missing evidence");
        assert!(error.to_string().contains("at least one EvidenceRef"));
    }
}

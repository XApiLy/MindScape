use serde::{Deserialize, Serialize};

use crate::domain::{KernelError, KernelResult};

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

impl EvidenceRef {
    pub fn validate(&self) -> KernelResult<()> {
        if self.id.trim().is_empty() {
            return Err(KernelError::Validation(
                "EvidenceRef id must not be empty".into(),
            ));
        }
        if self.created_at.trim().is_empty() {
            return Err(KernelError::Validation(
                "EvidenceRef created at must not be empty".into(),
            ));
        }
        if self
            .content_hash
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(KernelError::Validation(
                "EvidenceRef content hash must not be empty".into(),
            ));
        }
        if self
            .excerpt
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(KernelError::Validation(
                "EvidenceRef excerpt must not be empty".into(),
            ));
        }
        match &self.target {
            EvidenceTarget::MessageBlock { message_id, .. }
            | EvidenceTarget::ToolResultBlock {
                tool_run_id: message_id,
                ..
            } => require_non_empty(message_id, "EvidenceRef target id")?,
            EvidenceTarget::ImportContent {
                import_source_id,
                import_revision_id,
                locator,
            } => {
                require_non_empty(import_source_id, "EvidenceRef import source id")?;
                require_non_empty(import_revision_id, "EvidenceRef import revision id")?;
                require_non_empty(locator, "EvidenceRef locator")?;
            }
            EvidenceTarget::AttachmentContent {
                attachment_id,
                locator,
            } => {
                require_non_empty(attachment_id, "EvidenceRef attachment id")?;
                if locator
                    .as_deref()
                    .is_some_and(|value| value.trim().is_empty())
                {
                    return Err(KernelError::Validation(
                        "EvidenceRef attachment locator must not be empty".into(),
                    ));
                }
            }
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

    fn evidence() -> EvidenceRef {
        EvidenceRef {
            id: "evidence-1".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-1".into(),
                content_block_index: 0,
            },
            content_hash: Some("sha256:abc".into()),
            excerpt: Some("excerpt".into()),
            created_at: "2026-08-26T00:00:00Z".into(),
        }
    }

    #[test]
    fn validates_explicit_target_and_source_metadata() {
        evidence().validate().expect("valid evidence");

        let mut invalid = evidence();
        invalid.target = EvidenceTarget::ImportContent {
            import_source_id: "source-1".into(),
            import_revision_id: "revision-1".into(),
            locator: " ".into(),
        };
        let error = invalid.validate().expect_err("empty locator");
        assert!(error.to_string().contains("locator must not be empty"));
    }

    #[test]
    fn rejects_empty_identity_and_optional_metadata() {
        let mut invalid = evidence();
        invalid.id.clear();
        let error = invalid.validate().expect_err("empty evidence id");
        assert!(error.to_string().contains("EvidenceRef id"));

        let mut invalid = evidence();
        invalid.excerpt = Some("  ".into());
        let error = invalid.validate().expect_err("blank excerpt");
        assert!(error.to_string().contains("excerpt must not be empty"));
    }
}

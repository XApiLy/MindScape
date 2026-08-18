mod events;
mod evidence;
mod imports;
mod runtime;

pub use events::{AggregateType, DomainEventEnvelope, DomainEventType, EVENT_CONTRACT_VERSION};
pub use evidence::{EVIDENCE_CONTRACT_VERSION, EvidenceRef, EvidenceTarget};
pub use imports::{
    AnalysisMode, ContinuationClaim, ContinuationStatus, DerivedContinuation, FieldRecovery,
    IMPORT_CONTRACT_VERSION, ImportIssue, ImportPlatform, ImportRevision, ImportRevisionStatus,
    ImportSource, ImportedMessage, ParseReport, RecoveryStatus,
};
pub use runtime::{
    APPLICATION_INTERRUPTED_PROVIDER_CODE, CapabilityRequirement, FinishReason, ModelRunBudget,
    ModelRunEvent, ModelRunEventEnvelope, ModelRunProjection, ModelRunRequest, ModelUsage,
    ProviderError, ProviderErrorCategory, RUNTIME_CONTRACT_VERSION, RunCancelReason,
};

pub const DOMAIN_CONTRACT_VERSION: &str = "mindscape.domain.v1";

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::{ContentBlock, MessageRole, RunState};

    #[test]
    fn model_run_events_have_stable_discriminators() {
        let delta = serde_json::to_value(ModelRunEvent::TextDelta {
            delta: "hello".into(),
        })
        .expect("serialize delta");
        assert_eq!(delta, json!({ "type": "text_delta", "delta": "hello" }));

        let failed = serde_json::to_value(ModelRunEvent::Failed {
            error: ProviderError {
                category: ProviderErrorCategory::Authentication,
                provider_code: Some("invalid_key".into()),
                safe_message: "The configured credential was rejected.".into(),
                retryable: false,
                retry_after_ms: None,
                provider_status: Some(401),
            },
            partial_content_retained: false,
        })
        .expect("serialize failure");
        assert_eq!(failed["type"], "failed");
        assert_eq!(failed["error"]["category"], "authentication");
        assert_eq!(failed["partialContentRetained"], false);
        assert!(failed.get("rawResponse").is_none());
    }

    #[test]
    fn terminal_events_have_one_domain_state_mapping() {
        let completed = ModelRunEvent::Completed {
            finish_reason: FinishReason::Stop,
            usage: ModelUsage::default(),
        };
        let cancelled = ModelRunEvent::Cancelled {
            reason: RunCancelReason::UserRequested,
            partial_content_retained: true,
        };
        let interrupted = ModelRunEvent::application_interrupted(true);

        assert_eq!(completed.resulting_state(), RunState::Completed);
        assert_eq!(cancelled.resulting_state(), RunState::Cancelled);
        assert_eq!(interrupted.resulting_state(), RunState::Failed);
        assert!(completed.is_terminal());
        assert!(cancelled.is_terminal());
        assert!(interrupted.is_terminal());
        assert!(matches!(
            interrupted,
            ModelRunEvent::Failed {
                error: ProviderError {
                    provider_code: Some(ref code),
                    retryable: true,
                    ..
                },
                partial_content_retained: true,
            } if code == APPLICATION_INTERRUPTED_PROVIDER_CODE
        ));
    }

    #[test]
    fn imported_unknown_content_is_preserved_without_flattening() {
        let message = ImportedMessage {
            id: "imported-message-1".into(),
            import_revision_id: "revision-1".into(),
            role: MessageRole::Imported,
            content_blocks: vec![ContentBlock::Unsupported {
                original_type: "vendor_widget".into(),
                raw_json: json!({ "vendorId": 42 }),
            }],
            occurred_at: None,
            source_locator: "$.messages[0]".into(),
            parent_imported_message_id: None,
            platform_extension: json!({ "thread": "root" }),
        };

        let value = serde_json::to_value(message).expect("serialize imported message");
        assert_eq!(value["contentBlocks"][0]["type"], "unsupported");
        assert_eq!(value["contentBlocks"][0]["originalType"], "vendor_widget");
        assert_eq!(value["contentBlocks"][0]["rawJson"]["vendorId"], 42);
    }

    #[test]
    fn evidence_targets_are_explicit_and_versioned() {
        let evidence = EvidenceRef {
            id: "evidence-1".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-1".into(),
                content_block_index: 2,
            },
            content_hash: Some("sha256:example".into()),
            excerpt: Some("source excerpt".into()),
            created_at: "2026-08-14T00:00:00Z".into(),
        };

        let value = serde_json::to_value(evidence).expect("serialize evidence");
        assert_eq!(value["target"]["type"], "messageBlock");
        assert_eq!(value["target"]["contentBlockIndex"], 2);
        assert_eq!(EVIDENCE_CONTRACT_VERSION, "mindscape.evidence.v1");
    }
}

mod events;
mod evidence;
mod focus;
mod imports;
mod knowledge;
mod runtime;

pub use events::{AggregateType, DomainEventEnvelope, DomainEventType, EVENT_CONTRACT_VERSION};
pub use evidence::{EVIDENCE_CONTRACT_VERSION, EvidenceRef, EvidenceTarget};
pub use focus::{
    FOCUS_CONTRACT_VERSION, FocusBranchKind, FocusContextPolicy, FocusFrame, FocusMemoryScope,
    FocusPromotionCandidateSet, OmittedFocusRef,
};
pub use imports::{
    AnalysisMode, ContinuationClaim, ContinuationStatus, DerivedContinuation, FieldRecovery,
    GENERIC_IMPORT_CONTRACT_VERSION, GenericImportDescriptor, IMPORT_CONTRACT_VERSION,
    ImportAnalysisPolicy, ImportFormat, ImportGraphProjection, ImportIngress, ImportIssue,
    ImportPlatform, ImportRevision, ImportRevisionStatus, ImportSource, ImportedMessage,
    ParseReport, RawTrackEntry, RecoveryStatus,
};
pub use knowledge::{
    GeneratorKind, GeneratorRef, KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntity, KnowledgeEntityKind,
    KnowledgeRelation, KnowledgeRelationKind, KnowledgeScope, KnowledgeStatus,
    MARKDOWN_PROJECTION_CONTRACT_VERSION, MarkdownProjection, ScopedEvidenceRef,
};
pub use runtime::{
    APPLICATION_INTERRUPTED_PROVIDER_CODE, CapabilityRequirement,
    EFFECTIVE_RUN_PROFILE_CONTRACT_VERSION, EffectiveRunProfile, FinishReason,
    GenerationParameters, ModelRunBudget, ModelRunEvent, ModelRunEventEnvelope, ModelRunProjection,
    ModelRunRequest, ModelUsage, ProviderError, ProviderErrorCategory, RUNTIME_CONTRACT_VERSION,
    ReasoningMode, RunBudgetEnvelope, RunCancelReason, RunCapabilitySnapshot, RunValueOrigin,
    ToolPermission,
};

pub const DOMAIN_CONTRACT_VERSION: &str = "mindscape.domain.v1";

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

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

    #[test]
    fn effective_run_profile_serializes_actual_values_and_origins() {
        let profile = EffectiveRunProfile {
            contract_version: EFFECTIVE_RUN_PROFILE_CONTRACT_VERSION.into(),
            provider_id: "deepseek".into(),
            model_id: "deepseek-v4-flash".into(),
            reasoning_mode: ReasoningMode::Deep,
            reasoning_budget: Some(4_096),
            generation_parameters: GenerationParameters {
                temperature: Some(0.2),
                top_p: None,
                max_output_tokens: Some(2_048),
                response_format: None,
                seed: None,
                vendor_parameters: BTreeMap::from([("reasoningEffort".into(), json!("high"))]),
            },
            context_policy: "focusNew".into(),
            allowed_capabilities: vec![CapabilityRequirement::TextInput],
            tool_permission: ToolPermission::AskEachTime,
            budget_envelope: RunBudgetEnvelope {
                max_input_tokens: Some(8_192),
                max_reasoning_tokens: Some(4_096),
                max_output_tokens: Some(2_048),
                max_cost_microunits: None,
                timeout_ms: 30_000,
            },
            value_origins: BTreeMap::from([("reasoningMode".into(), RunValueOrigin::User)]),
            capability_snapshot: RunCapabilitySnapshot {
                catalog_version: "provider-catalog-v1".into(),
                context_window_tokens: Some(64_000),
                supported_capabilities: vec![CapabilityRequirement::TextInput],
                unsupported_parameters: vec!["seed".into()],
            },
        };

        let value = serde_json::to_value(profile).expect("serialize effective run profile");
        assert_eq!(value["reasoningMode"], "deep");
        assert_eq!(value["generationParameters"]["temperature"], 0.2);
        assert_eq!(value["valueOrigins"]["reasoningMode"], "user");
        assert_eq!(
            value["capabilitySnapshot"]["unsupportedParameters"][0],
            "seed"
        );
    }

    #[test]
    fn confirmed_knowledge_keeps_scope_revision_generator_and_evidence() {
        let entity = KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "decision-focus-frame".into(),
            kind: KnowledgeEntityKind::Decision,
            name: "Use FocusFrame".into(),
            aliases: vec![],
            scope: KnowledgeScope::Project {
                workspace_id: "workspace-1".into(),
                project_id: "project-mindscape".into(),
            },
            status: KnowledgeStatus::Confirmed,
            revision: 2,
            evidence: vec![ScopedEvidenceRef {
                id: "scoped-evidence-1".into(),
                evidence: EvidenceRef {
                    id: "evidence-1".into(),
                    target: EvidenceTarget::MessageBlock {
                        message_id: "message-1".into(),
                        content_block_index: 0,
                    },
                    content_hash: Some("sha256:source".into()),
                    excerpt: None,
                    created_at: "2026-08-24T00:00:00Z".into(),
                },
                scope: KnowledgeScope::Conversation {
                    workspace_id: "workspace-1".into(),
                    conversation_id: "conversation-1".into(),
                },
                status: KnowledgeStatus::Confirmed,
                revision: 1,
                generator: GeneratorRef {
                    kind: GeneratorKind::User,
                    generator_id: "user-confirmation".into(),
                    generator_version: "v1".into(),
                },
            }],
            generator: GeneratorRef {
                kind: GeneratorKind::User,
                generator_id: "user-confirmation".into(),
                generator_version: "v1".into(),
            },
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T01:00:00Z".into(),
        };

        let value = serde_json::to_value(entity).expect("serialize knowledge entity");
        assert_eq!(value["scope"]["type"], "project");
        assert_eq!(value["status"], "confirmed");
        assert_eq!(value["revision"], 2);
        assert_eq!(value["evidence"][0]["generator"]["kind"], "user");
    }

    #[test]
    fn raw_import_projection_cannot_request_generative_analysis() {
        let projection = ImportGraphProjection {
            import_source_id: "import-source-1".into(),
            import_revision_id: "import-revision-1".into(),
            conversation_id: "conversation-1".into(),
            entry_node_id: "node-import-entry".into(),
            raw_track_entry_ids: vec!["raw-track-1".into()],
            analysis_policy: ImportAnalysisPolicy::Disabled,
        };

        let value = serde_json::to_value(projection).expect("serialize import graph projection");
        assert_eq!(value["analysisPolicy"], "disabled");
        assert!(value.get("platformSpecificFields").is_none());
    }
}

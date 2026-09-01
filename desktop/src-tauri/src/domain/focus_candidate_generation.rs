use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    FocusFrameLifecycleSnapshot, FocusFrameLifecycleStatus,
    contracts::{
        FocusBranchKind, GeneratorKind, GeneratorRef, KnowledgeEntity, KnowledgeScope,
        KnowledgeStatus,
    },
};

pub const FOCUS_PROMOTION_GENERATION_CONTRACT_VERSION: &str =
    "mindscape.focus-promotion-generation.v1";

/// User-authored request to select existing branch-local knowledge candidates.
/// The application layer must load every referenced entity from authoritative
/// storage before calling [`plan_focus_promotion_candidate_generation`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusPromotionCandidateGenerationCommandInput {
    pub generation_id: String,
    pub focus_frame_id: String,
    pub expected_memory_version: u64,
    pub expected_lifecycle_revision: u64,
    pub candidate_refs: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusPromotionCandidateSourceRevision {
    pub candidate_ref: String,
    pub entity_revision: u64,
}

/// Kernel-authored receipt for one explicit candidate selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusPromotionCandidateGenerationProjection {
    pub contract_version: String,
    pub generation_id: String,
    pub focus_frame_id: String,
    pub conversation_id: String,
    pub branch_kind: FocusBranchKind,
    pub candidate_refs: Vec<String>,
    pub source_entity_revisions: Vec<FocusPromotionCandidateSourceRevision>,
    pub memory_version: u64,
    pub lifecycle_revision: u64,
    pub selected_by: GeneratorRef,
    pub generated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusPromotionCandidateGenerationPlan {
    pub lifecycle: FocusFrameLifecycleSnapshot,
    pub generation: FocusPromotionCandidateGenerationProjection,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FocusPromotionCandidateGenerationError {
    #[error("focus promotion generation field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("focus promotion generation requires at least one candidate")]
    EmptySelection,
    #[error("focus promotion candidate reference is invalid: {candidate_ref}")]
    InvalidCandidateRef { candidate_ref: String },
    #[error("focus promotion generation candidate {candidate_ref} appears more than once")]
    DuplicateCandidate { candidate_ref: String },
    #[error("focus promotion generation expected versions must be greater than zero")]
    InvalidExpectedVersion,
    #[error("invalid focus lifecycle for candidate generation: {reason}")]
    InvalidLifecycle { reason: String },
    #[error("focus promotion candidates can only be selected while the FocusFrame is active")]
    ActiveFocusFrameRequired,
    #[error("mainline FocusFrame cannot generate promotion candidates")]
    MainlineNotAllowed,
    #[error("focus promotion generation does not match the requested FocusFrame")]
    FocusFrameMismatch,
    #[error("focus memory version conflict: expected {expected}, actual {actual}")]
    StaleMemoryVersion { expected: u64, actual: u64 },
    #[error("focus lifecycle revision conflict: expected {expected}, actual {actual}")]
    StaleLifecycleRevision { expected: u64, actual: u64 },
    #[error("focus promotion candidate selection must be authored by a valid user generator")]
    InvalidActor,
    #[error("focus promotion candidate {candidate_ref} was not found in authoritative storage")]
    CandidateNotFound { candidate_ref: String },
    #[error("authoritative entity {candidate_ref} was not selected by the user")]
    UnexpectedCandidate { candidate_ref: String },
    #[error("invalid focus promotion candidate {candidate_ref}: {reason}")]
    InvalidCandidate {
        candidate_ref: String,
        reason: String,
    },
    #[error("focus promotion candidate {candidate_ref} must belong to the same FocusFrame")]
    CandidateScopeMismatch { candidate_ref: String },
    #[error("focus promotion candidate {candidate_ref} must be candidate or inferred")]
    InvalidCandidateStatus { candidate_ref: String },
    #[error("focus promotion candidate {candidate_ref} requires at least one EvidenceRef")]
    MissingEvidence { candidate_ref: String },
    #[error("focus promotion candidate selection is unchanged")]
    UnchangedSelection,
    #[error("focus memory version overflowed")]
    MemoryVersionOverflow,
    #[error("focus lifecycle revision overflowed")]
    LifecycleRevisionOverflow,
    #[error("generated focus lifecycle is invalid: {reason}")]
    InvalidGeneratedLifecycle { reason: String },
}

/// Builds the only valid mutation plan for turning an explicit user selection
/// into `promoteRefs`. Candidate identities, revisions, scope, status and
/// evidence are verified against kernel-loaded entities; the UI cannot create
/// a candidate by sending an arbitrary ID.
///
/// # Errors
///
/// Returns a typed error when the selection is stale, duplicated, missing from
/// authoritative storage, outside the active FocusFrame, not candidate-like,
/// or lacks evidence.
pub fn plan_focus_promotion_candidate_generation(
    input: &FocusPromotionCandidateGenerationCommandInput,
    lifecycle: &FocusFrameLifecycleSnapshot,
    selected_entities: &[KnowledgeEntity],
    actor: &GeneratorRef,
) -> Result<FocusPromotionCandidateGenerationPlan, FocusPromotionCandidateGenerationError> {
    validate_command(input)?;
    lifecycle.validate().map_err(|error| {
        FocusPromotionCandidateGenerationError::InvalidLifecycle {
            reason: error.to_string(),
        }
    })?;
    validate_context(input, lifecycle, actor)?;

    let selected_refs = input
        .candidate_refs
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut entities_by_id = HashMap::with_capacity(selected_entities.len());
    for entity in selected_entities {
        if !selected_refs.contains(entity.id.as_str()) {
            return Err(
                FocusPromotionCandidateGenerationError::UnexpectedCandidate {
                    candidate_ref: entity.id.clone(),
                },
            );
        }
        if entities_by_id.insert(entity.id.as_str(), entity).is_some() {
            return Err(FocusPromotionCandidateGenerationError::DuplicateCandidate {
                candidate_ref: entity.id.clone(),
            });
        }
    }

    let mut candidate_refs = input.candidate_refs.clone();
    candidate_refs.sort_unstable();
    let mut source_entity_revisions = Vec::with_capacity(candidate_refs.len());
    for candidate_ref in &candidate_refs {
        let entity = entities_by_id.get(candidate_ref.as_str()).ok_or_else(|| {
            FocusPromotionCandidateGenerationError::CandidateNotFound {
                candidate_ref: candidate_ref.clone(),
            }
        })?;
        validate_candidate(entity, lifecycle)?;
        source_entity_revisions.push(FocusPromotionCandidateSourceRevision {
            candidate_ref: candidate_ref.clone(),
            entity_revision: entity.revision,
        });
    }

    let mut current_refs = lifecycle.frame.memory_scope.promote_refs.clone();
    current_refs.sort_unstable();
    if current_refs == candidate_refs {
        return Err(FocusPromotionCandidateGenerationError::UnchangedSelection);
    }

    let memory_version = lifecycle
        .frame
        .memory_version
        .checked_add(1)
        .ok_or(FocusPromotionCandidateGenerationError::MemoryVersionOverflow)?;
    let lifecycle_revision = lifecycle
        .revision
        .checked_add(1)
        .ok_or(FocusPromotionCandidateGenerationError::LifecycleRevisionOverflow)?;
    let mut updated_lifecycle = lifecycle.clone();
    updated_lifecycle.frame.memory_scope.promote_refs = candidate_refs.clone();
    updated_lifecycle.frame.memory_version = memory_version;
    updated_lifecycle.revision = lifecycle_revision;
    updated_lifecycle.updated_at = input.generated_at.clone();
    updated_lifecycle.validate().map_err(|error| {
        FocusPromotionCandidateGenerationError::InvalidGeneratedLifecycle {
            reason: error.to_string(),
        }
    })?;

    Ok(FocusPromotionCandidateGenerationPlan {
        lifecycle: updated_lifecycle,
        generation: FocusPromotionCandidateGenerationProjection {
            contract_version: FOCUS_PROMOTION_GENERATION_CONTRACT_VERSION.into(),
            generation_id: input.generation_id.clone(),
            focus_frame_id: lifecycle.frame.id.clone(),
            conversation_id: lifecycle.frame.conversation_id.clone(),
            branch_kind: lifecycle.frame.memory_scope.branch_kind,
            candidate_refs,
            source_entity_revisions,
            memory_version,
            lifecycle_revision,
            selected_by: actor.clone(),
            generated_at: input.generated_at.clone(),
        },
    })
}

fn validate_command(
    input: &FocusPromotionCandidateGenerationCommandInput,
) -> Result<(), FocusPromotionCandidateGenerationError> {
    for (field, value) in [
        ("generationId", input.generation_id.as_str()),
        ("focusFrameId", input.focus_frame_id.as_str()),
        ("generatedAt", input.generated_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FocusPromotionCandidateGenerationError::EmptyField { field });
        }
    }
    if input.expected_memory_version == 0 || input.expected_lifecycle_revision == 0 {
        return Err(FocusPromotionCandidateGenerationError::InvalidExpectedVersion);
    }
    if input.candidate_refs.is_empty() {
        return Err(FocusPromotionCandidateGenerationError::EmptySelection);
    }
    let mut unique = HashSet::new();
    for candidate_ref in &input.candidate_refs {
        if candidate_ref.trim().is_empty() || candidate_ref.trim() != candidate_ref {
            return Err(
                FocusPromotionCandidateGenerationError::InvalidCandidateRef {
                    candidate_ref: candidate_ref.clone(),
                },
            );
        }
        if !unique.insert(candidate_ref.as_str()) {
            return Err(FocusPromotionCandidateGenerationError::DuplicateCandidate {
                candidate_ref: candidate_ref.clone(),
            });
        }
    }
    Ok(())
}

fn validate_context(
    input: &FocusPromotionCandidateGenerationCommandInput,
    lifecycle: &FocusFrameLifecycleSnapshot,
    actor: &GeneratorRef,
) -> Result<(), FocusPromotionCandidateGenerationError> {
    if lifecycle.status != FocusFrameLifecycleStatus::Active {
        return Err(FocusPromotionCandidateGenerationError::ActiveFocusFrameRequired);
    }
    if lifecycle.frame.memory_scope.branch_kind == FocusBranchKind::Mainline {
        return Err(FocusPromotionCandidateGenerationError::MainlineNotAllowed);
    }
    if lifecycle.frame.id != input.focus_frame_id {
        return Err(FocusPromotionCandidateGenerationError::FocusFrameMismatch);
    }
    if lifecycle.frame.memory_version != input.expected_memory_version {
        return Err(FocusPromotionCandidateGenerationError::StaleMemoryVersion {
            expected: input.expected_memory_version,
            actual: lifecycle.frame.memory_version,
        });
    }
    if lifecycle.revision != input.expected_lifecycle_revision {
        return Err(
            FocusPromotionCandidateGenerationError::StaleLifecycleRevision {
                expected: input.expected_lifecycle_revision,
                actual: lifecycle.revision,
            },
        );
    }
    if actor.kind != GeneratorKind::User || actor.validate().is_err() {
        return Err(FocusPromotionCandidateGenerationError::InvalidActor);
    }
    Ok(())
}

fn validate_candidate(
    entity: &KnowledgeEntity,
    lifecycle: &FocusFrameLifecycleSnapshot,
) -> Result<(), FocusPromotionCandidateGenerationError> {
    entity
        .validate_for_conversation(&lifecycle.frame.conversation_id)
        .map_err(
            |error| FocusPromotionCandidateGenerationError::InvalidCandidate {
                candidate_ref: entity.id.clone(),
                reason: error.to_string(),
            },
        )?;
    match &entity.scope {
        KnowledgeScope::FocusFrame {
            conversation_id,
            focus_frame_id,
            ..
        } if conversation_id == &lifecycle.frame.conversation_id
            && focus_frame_id == &lifecycle.frame.id => {}
        _ => {
            return Err(
                FocusPromotionCandidateGenerationError::CandidateScopeMismatch {
                    candidate_ref: entity.id.clone(),
                },
            );
        }
    }
    if !matches!(
        entity.status,
        KnowledgeStatus::Candidate | KnowledgeStatus::Inferred
    ) {
        return Err(
            FocusPromotionCandidateGenerationError::InvalidCandidateStatus {
                candidate_ref: entity.id.clone(),
            },
        );
    }
    if entity.evidence.is_empty() {
        return Err(FocusPromotionCandidateGenerationError::MissingEvidence {
            candidate_ref: entity.id.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        FOCUS_LIFECYCLE_CONTRACT_VERSION,
        contracts::{
            EvidenceRef, EvidenceTarget, FOCUS_CONTRACT_VERSION, FocusContextPolicy, FocusFrame,
            FocusMemoryScope, KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntityKind, ScopedEvidenceRef,
        },
    };

    fn actor() -> GeneratorRef {
        GeneratorRef {
            kind: GeneratorKind::User,
            generator_id: "user-1".into(),
            generator_version: "v1".into(),
        }
    }

    fn lifecycle(branch_kind: FocusBranchKind) -> FocusFrameLifecycleSnapshot {
        FocusFrameLifecycleSnapshot {
            contract_version: FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
            frame: FocusFrame {
                contract_version: FOCUS_CONTRACT_VERSION.into(),
                id: "focus-task-1".into(),
                conversation_id: "conversation-1".into(),
                parent_node_id: Some("node-1".into()),
                objective: "Return the verified branch result".into(),
                active_work_item: None,
                context_policy: FocusContextPolicy::BranchFromNode,
                memory_scope: FocusMemoryScope {
                    branch_kind,
                    inherit_refs: vec![],
                    local_refs: vec![],
                    exclude_refs: vec![],
                    promote_refs: vec![],
                },
                include_refs: vec![],
                exclude_refs: vec![],
                memory_version: 3,
                created_at: "2026-09-01T00:00:00Z".into(),
            },
            status: FocusFrameLifecycleStatus::Active,
            revision: 2,
            updated_at: "2026-09-01T00:00:00Z".into(),
            closed_at: None,
        }
    }

    fn candidate(id: &str, status: KnowledgeStatus) -> KnowledgeEntity {
        let scope = KnowledgeScope::FocusFrame {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-1".into(),
            focus_frame_id: "focus-task-1".into(),
        };
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: id.into(),
            kind: KnowledgeEntityKind::Decision,
            name: format!("Candidate {id}"),
            aliases: vec![],
            scope: scope.clone(),
            status,
            revision: 4,
            evidence: vec![ScopedEvidenceRef {
                id: format!("scoped-{id}"),
                evidence: EvidenceRef {
                    id: format!("evidence-{id}"),
                    target: EvidenceTarget::MessageBlock {
                        message_id: "message-1".into(),
                        content_block_index: 0,
                    },
                    content_hash: Some("sha256:source".into()),
                    excerpt: None,
                    created_at: "2026-09-01T00:00:00Z".into(),
                },
                scope,
                status: KnowledgeStatus::Candidate,
                revision: 1,
                generator: actor(),
            }],
            generator: actor(),
            created_at: "2026-09-01T00:00:00Z".into(),
            updated_at: "2026-09-01T00:00:00Z".into(),
        }
    }

    fn command(candidate_refs: Vec<String>) -> FocusPromotionCandidateGenerationCommandInput {
        FocusPromotionCandidateGenerationCommandInput {
            generation_id: "generation-1".into(),
            focus_frame_id: "focus-task-1".into(),
            expected_memory_version: 3,
            expected_lifecycle_revision: 2,
            candidate_refs,
            generated_at: "2026-09-01T01:00:00Z".into(),
        }
    }

    #[test]
    fn generation_sorts_authoritative_candidates_and_increments_versions() {
        let plan = plan_focus_promotion_candidate_generation(
            &command(vec!["entity-b".into(), "entity-a".into()]),
            &lifecycle(FocusBranchKind::Task),
            &[
                candidate("entity-b", KnowledgeStatus::Candidate),
                candidate("entity-a", KnowledgeStatus::Inferred),
            ],
            &actor(),
        )
        .expect("generate candidates");

        assert_eq!(plan.generation.candidate_refs, ["entity-a", "entity-b"]);
        assert_eq!(plan.generation.memory_version, 4);
        assert_eq!(plan.generation.lifecycle_revision, 3);
        assert_eq!(
            plan.lifecycle.frame.memory_scope.promote_refs,
            ["entity-a", "entity-b"]
        );
    }

    #[test]
    fn generation_rejects_an_id_missing_from_authoritative_storage() {
        let error = plan_focus_promotion_candidate_generation(
            &command(vec!["entity-missing".into()]),
            &lifecycle(FocusBranchKind::Task),
            &[],
            &actor(),
        )
        .expect_err("missing entity");

        assert_eq!(
            error,
            FocusPromotionCandidateGenerationError::CandidateNotFound {
                candidate_ref: "entity-missing".into(),
            }
        );
    }

    #[test]
    fn generation_rejects_non_branch_local_entities() {
        let mut entity = candidate("entity-1", KnowledgeStatus::Candidate);
        entity.scope = KnowledgeScope::Conversation {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-1".into(),
        };
        let error = plan_focus_promotion_candidate_generation(
            &command(vec![entity.id.clone()]),
            &lifecycle(FocusBranchKind::Task),
            &[entity],
            &actor(),
        )
        .expect_err("scope mismatch");

        assert!(matches!(
            error,
            FocusPromotionCandidateGenerationError::CandidateScopeMismatch { .. }
        ));
    }

    #[test]
    fn generation_rejects_confirmed_entities() {
        let confirmed = candidate("entity-confirmed", KnowledgeStatus::Confirmed);
        let error = plan_focus_promotion_candidate_generation(
            &command(vec![confirmed.id.clone()]),
            &lifecycle(FocusBranchKind::Task),
            &[confirmed],
            &actor(),
        )
        .expect_err("confirmed is not a new branch candidate");
        assert!(matches!(
            error,
            FocusPromotionCandidateGenerationError::InvalidCandidateStatus { .. }
        ));
    }

    #[test]
    fn generation_rejects_ungrounded_entities() {
        let mut ungrounded = candidate("entity-ungrounded", KnowledgeStatus::Candidate);
        ungrounded.evidence.clear();
        let error = plan_focus_promotion_candidate_generation(
            &command(vec![ungrounded.id.clone()]),
            &lifecycle(FocusBranchKind::Task),
            &[ungrounded],
            &actor(),
        )
        .expect_err("evidence required");
        assert!(matches!(
            error,
            FocusPromotionCandidateGenerationError::MissingEvidence { .. }
        ));
    }

    #[test]
    fn generation_rejects_closed_focus_frame() {
        let entity = candidate("entity-1", KnowledgeStatus::Candidate);
        let mut closed = lifecycle(FocusBranchKind::Task);
        closed.status = FocusFrameLifecycleStatus::Closed;
        closed.closed_at = Some("2026-09-01T00:30:00Z".into());
        assert_eq!(
            plan_focus_promotion_candidate_generation(
                &command(vec![entity.id.clone()]),
                &closed,
                std::slice::from_ref(&entity),
                &actor(),
            )
            .expect_err("closed selection"),
            FocusPromotionCandidateGenerationError::ActiveFocusFrameRequired
        );
    }

    #[test]
    fn generation_rejects_mainline_focus_frame() {
        let entity = candidate("entity-1", KnowledgeStatus::Candidate);
        assert_eq!(
            plan_focus_promotion_candidate_generation(
                &command(vec![entity.id.clone()]),
                &lifecycle(FocusBranchKind::Mainline),
                std::slice::from_ref(&entity),
                &actor(),
            )
            .expect_err("mainline selection"),
            FocusPromotionCandidateGenerationError::MainlineNotAllowed
        );
    }

    #[test]
    fn generation_rejects_stale_memory_version() {
        let entity = candidate("entity-1", KnowledgeStatus::Candidate);
        let mut stale = command(vec![entity.id.clone()]);
        stale.expected_memory_version = 2;
        assert!(matches!(
            plan_focus_promotion_candidate_generation(
                &stale,
                &lifecycle(FocusBranchKind::Task),
                &[entity],
                &actor(),
            ),
            Err(FocusPromotionCandidateGenerationError::StaleMemoryVersion { .. })
        ));
    }

    #[test]
    fn generation_serializes_the_frozen_typed_shape() {
        let value = serde_json::to_value(command(vec!["entity-1".into()]))
            .expect("serialize generation input");

        assert_eq!(value["generationId"], "generation-1");
        assert_eq!(value["focusFrameId"], "focus-task-1");
        assert_eq!(value["expectedMemoryVersion"], 3);
        assert_eq!(value["expectedLifecycleRevision"], 2);
        assert_eq!(value["candidateRefs"][0], "entity-1");
    }
}

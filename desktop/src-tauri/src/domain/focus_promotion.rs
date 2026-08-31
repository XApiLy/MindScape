use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    FocusFrameLifecycleSnapshot, FocusFrameLifecycleStatus, KnowledgeAction, KnowledgeTransition,
    KnowledgeTransitionError,
    contracts::{
        FOCUS_CONTRACT_VERSION, FocusPromotionCandidateSet, GeneratorKind, GeneratorRef,
        KnowledgeEntity, KnowledgeScope, KnowledgeStatus,
    },
    transition_entity,
};

pub const FOCUS_PROMOTION_DECISION_CONTRACT_VERSION: &str = "mindscape.focus-promotion-decision.v1";

/// A promotion decision is immutable once persisted. Repeating the same
/// `decision_id` is an idempotent read of that decision; a different decision
/// for the same FocusFrame/candidate pair must be rejected by storage.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FocusPromotionDecisionAction {
    /// Confirm the candidate only inside its current FocusFrame scope.
    Confirm,
    /// Confirm the source candidate and create a confirmed conversation/project entity.
    Promote,
    /// Mark the candidate as rejected and exclude it from retrieval.
    Reject,
    /// Delete the source entity while retaining the immutable decision tombstone.
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FocusPromotionTargetScope {
    Conversation {
        workspace_id: String,
        conversation_id: String,
    },
    Project {
        workspace_id: String,
        project_id: String,
    },
}

impl FocusPromotionTargetScope {
    fn into_knowledge_scope(self) -> KnowledgeScope {
        match self {
            Self::Conversation {
                workspace_id,
                conversation_id,
            } => KnowledgeScope::Conversation {
                workspace_id,
                conversation_id,
            },
            Self::Project {
                workspace_id,
                project_id,
            } => KnowledgeScope::Project {
                workspace_id,
                project_id,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusPromotionDecisionCommandInput {
    /// Stable idempotency key generated once by the client.
    pub decision_id: String,
    pub focus_frame_id: String,
    pub candidate_ref: String,
    pub expected_memory_version: u64,
    pub expected_lifecycle_revision: u64,
    pub expected_entity_revision: u64,
    /// V1 decisions are create-only, so callers must send zero.
    pub expected_decision_revision: u64,
    pub action: FocusPromotionDecisionAction,
    pub target_scope: Option<FocusPromotionTargetScope>,
    /// Required only for `promote`; the source entity keeps its stable ID.
    pub promoted_entity_id: Option<String>,
    pub decided_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusPromotionDecisionProjection {
    pub contract_version: String,
    pub decision_id: String,
    pub focus_frame_id: String,
    pub conversation_id: String,
    pub candidate_ref: String,
    pub action: FocusPromotionDecisionAction,
    pub target_scope: Option<FocusPromotionTargetScope>,
    pub promoted_entity_id: Option<String>,
    pub source_entity_revision: Option<u64>,
    pub decision_revision: u64,
    pub memory_version: u64,
    pub lifecycle_revision: u64,
    pub decided_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusPromotionEntityMutation {
    UpsertSource(Box<KnowledgeEntity>),
    Promote {
        source: Box<KnowledgeEntity>,
        promoted: Box<KnowledgeEntity>,
    },
    DeleteSource {
        entity_id: String,
        expected_revision: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusPromotionDecisionPlan {
    pub decision: FocusPromotionDecisionProjection,
    pub entity_mutation: FocusPromotionEntityMutation,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FocusPromotionDecisionError {
    #[error("focus promotion decision field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("focus promotion decision expected versions must be greater than zero")]
    InvalidExpectedVersion,
    #[error(
        "focus promotion decisions are create-only and require expected decision revision zero"
    )]
    DecisionAlreadyExpected,
    #[error("focus promotion decision requires a closed FocusFrame")]
    FocusFrameNotClosed,
    #[error("focus promotion decision does not match the lifecycle FocusFrame")]
    FocusFrameMismatch,
    #[error("focus promotion candidate set does not match the lifecycle snapshot")]
    CandidateSetMismatch,
    #[error("focus promotion memory version is stale: expected {expected}, actual {actual}")]
    StaleMemoryVersion { expected: u64, actual: u64 },
    #[error("focus promotion lifecycle revision is stale: expected {expected}, actual {actual}")]
    StaleLifecycleRevision { expected: u64, actual: u64 },
    #[error("focus promotion entity revision is stale: expected {expected}, actual {actual}")]
    StaleEntityRevision { expected: u64, actual: u64 },
    #[error("focus promotion candidate {candidate_ref} is not in the frozen candidate set")]
    CandidateNotFound { candidate_ref: String },
    #[error("focus promotion candidate must be a FocusFrame-scoped entity from the same branch")]
    CandidateScopeMismatch,
    #[error("focus promotion candidate must be candidate or inferred")]
    InvalidCandidateStatus,
    #[error("focus promotion decisions must be attributed to a user generator")]
    InvalidActor,
    #[error("invalid FocusFrame lifecycle snapshot: {reason}")]
    InvalidLifecycle { reason: String },
    #[error("invalid promotion candidate entity: {reason}")]
    InvalidEntity { reason: String },
    #[error("focus promotion target fields are invalid for the selected action")]
    InvalidTarget,
    #[error(transparent)]
    KnowledgeTransition(#[from] KnowledgeTransitionError),
}

/// Builds the complete domain mutation that storage must commit atomically
/// with the immutable decision projection. This function intentionally does
/// no I/O: SQLite/Vault/index adapters consume the plan under one transaction.
pub fn plan_focus_promotion_decision(
    input: &FocusPromotionDecisionCommandInput,
    candidates: &FocusPromotionCandidateSet,
    lifecycle: &FocusFrameLifecycleSnapshot,
    entity: &KnowledgeEntity,
    actor: &GeneratorRef,
) -> Result<FocusPromotionDecisionPlan, FocusPromotionDecisionError> {
    validate_command(input)?;
    validate_context(input, candidates, lifecycle, entity, actor)?;

    let transition_action = match input.action {
        FocusPromotionDecisionAction::Confirm | FocusPromotionDecisionAction::Promote => {
            KnowledgeAction::Confirm
        }
        FocusPromotionDecisionAction::Reject => KnowledgeAction::Reject,
        FocusPromotionDecisionAction::Delete => {
            let decision = projection(input, candidates, lifecycle, None);
            return Ok(FocusPromotionDecisionPlan {
                decision,
                entity_mutation: FocusPromotionEntityMutation::DeleteSource {
                    entity_id: entity.id.clone(),
                    expected_revision: entity.revision,
                },
            });
        }
    };

    let source = transition_entity(
        entity,
        &KnowledgeTransition {
            action: transition_action,
            replacement_entity_id: None,
        },
        actor,
        &input.decided_at,
    )?;

    let entity_mutation = if input.action == FocusPromotionDecisionAction::Promote {
        let Some(promoted_entity_id) = input.promoted_entity_id.as_ref() else {
            return Err(FocusPromotionDecisionError::InvalidTarget);
        };
        let Some(target_scope) = input.target_scope.clone() else {
            return Err(FocusPromotionDecisionError::InvalidTarget);
        };
        let promoted = KnowledgeEntity {
            contract_version: source.contract_version.clone(),
            id: promoted_entity_id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            aliases: source.aliases.clone(),
            scope: target_scope.into_knowledge_scope(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: source.evidence.clone(),
            generator: actor.clone(),
            created_at: input.decided_at.clone(),
            updated_at: input.decided_at.clone(),
        };
        FocusPromotionEntityMutation::Promote {
            source: Box::new(source),
            promoted: Box::new(promoted),
        }
    } else {
        FocusPromotionEntityMutation::UpsertSource(Box::new(source))
    };

    let source_entity_revision = match &entity_mutation {
        FocusPromotionEntityMutation::UpsertSource(source) => Some(source.revision),
        FocusPromotionEntityMutation::Promote { source, .. } => Some(source.revision),
        FocusPromotionEntityMutation::DeleteSource { .. } => None,
    };
    Ok(FocusPromotionDecisionPlan {
        decision: projection(input, candidates, lifecycle, source_entity_revision),
        entity_mutation,
    })
}

fn validate_command(
    input: &FocusPromotionDecisionCommandInput,
) -> Result<(), FocusPromotionDecisionError> {
    for (field, value) in [
        ("decisionId", input.decision_id.as_str()),
        ("focusFrameId", input.focus_frame_id.as_str()),
        ("candidateRef", input.candidate_ref.as_str()),
        ("decidedAt", input.decided_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FocusPromotionDecisionError::EmptyField { field });
        }
    }
    if input.expected_memory_version == 0
        || input.expected_lifecycle_revision == 0
        || input.expected_entity_revision == 0
    {
        return Err(FocusPromotionDecisionError::InvalidExpectedVersion);
    }
    if input.expected_decision_revision != 0 {
        return Err(FocusPromotionDecisionError::DecisionAlreadyExpected);
    }

    match input.action {
        FocusPromotionDecisionAction::Promote
            if input.target_scope.is_none()
                || input
                    .promoted_entity_id
                    .as_deref()
                    .is_none_or(|id| id.trim().is_empty() || id == input.candidate_ref) =>
        {
            return Err(FocusPromotionDecisionError::InvalidTarget);
        }
        FocusPromotionDecisionAction::Promote => {}
        _ if input.target_scope.is_some() || input.promoted_entity_id.is_some() => {
            return Err(FocusPromotionDecisionError::InvalidTarget);
        }
        _ => {}
    }
    Ok(())
}

fn validate_context(
    input: &FocusPromotionDecisionCommandInput,
    candidates: &FocusPromotionCandidateSet,
    lifecycle: &FocusFrameLifecycleSnapshot,
    entity: &KnowledgeEntity,
    actor: &GeneratorRef,
) -> Result<(), FocusPromotionDecisionError> {
    lifecycle
        .validate()
        .map_err(|error| FocusPromotionDecisionError::InvalidLifecycle {
            reason: error.to_string(),
        })?;
    entity
        .validate()
        .map_err(|error| FocusPromotionDecisionError::InvalidEntity {
            reason: error.to_string(),
        })?;
    if lifecycle.status != FocusFrameLifecycleStatus::Closed {
        return Err(FocusPromotionDecisionError::FocusFrameNotClosed);
    }
    if lifecycle.frame.id != input.focus_frame_id {
        return Err(FocusPromotionDecisionError::FocusFrameMismatch);
    }
    if candidates.focus_frame_id != lifecycle.frame.id
        || candidates.conversation_id != lifecycle.frame.conversation_id
        || candidates.branch_kind != lifecycle.frame.memory_scope.branch_kind
        || candidates.memory_version != lifecycle.frame.memory_version
        || candidates.contract_version != FOCUS_CONTRACT_VERSION
    {
        return Err(FocusPromotionDecisionError::CandidateSetMismatch);
    }
    let mut unique_candidates = std::collections::HashSet::new();
    if candidates.candidate_refs.iter().any(|candidate_ref| {
        candidate_ref.trim().is_empty()
            || candidate_ref.trim() != candidate_ref
            || !unique_candidates.insert(candidate_ref)
    }) {
        return Err(FocusPromotionDecisionError::CandidateSetMismatch);
    }
    if input.expected_memory_version != candidates.memory_version {
        return Err(FocusPromotionDecisionError::StaleMemoryVersion {
            expected: input.expected_memory_version,
            actual: candidates.memory_version,
        });
    }
    if input.expected_lifecycle_revision != lifecycle.revision {
        return Err(FocusPromotionDecisionError::StaleLifecycleRevision {
            expected: input.expected_lifecycle_revision,
            actual: lifecycle.revision,
        });
    }
    if input.expected_entity_revision != entity.revision {
        return Err(FocusPromotionDecisionError::StaleEntityRevision {
            expected: input.expected_entity_revision,
            actual: entity.revision,
        });
    }
    if !candidates
        .candidate_refs
        .iter()
        .any(|candidate_ref| candidate_ref == &input.candidate_ref)
    {
        return Err(FocusPromotionDecisionError::CandidateNotFound {
            candidate_ref: input.candidate_ref.clone(),
        });
    }
    if entity.id != input.candidate_ref {
        return Err(FocusPromotionDecisionError::CandidateNotFound {
            candidate_ref: input.candidate_ref.clone(),
        });
    }
    let (workspace_id, conversation_id, focus_frame_id) = match &entity.scope {
        KnowledgeScope::FocusFrame {
            workspace_id,
            conversation_id,
            focus_frame_id,
        } => (workspace_id, conversation_id, focus_frame_id),
        _ => return Err(FocusPromotionDecisionError::CandidateScopeMismatch),
    };
    if conversation_id != &lifecycle.frame.conversation_id || focus_frame_id != &lifecycle.frame.id
    {
        return Err(FocusPromotionDecisionError::CandidateScopeMismatch);
    }
    if !matches!(
        entity.status,
        KnowledgeStatus::Candidate | KnowledgeStatus::Inferred
    ) {
        return Err(FocusPromotionDecisionError::InvalidCandidateStatus);
    }
    if actor.kind != GeneratorKind::User {
        return Err(FocusPromotionDecisionError::InvalidActor);
    }
    actor
        .validate()
        .map_err(|_| FocusPromotionDecisionError::InvalidActor)?;

    if let Some(target_scope) = &input.target_scope {
        let valid = match target_scope {
            FocusPromotionTargetScope::Conversation {
                workspace_id: target_workspace_id,
                conversation_id: target_conversation_id,
            } => {
                !target_workspace_id.trim().is_empty()
                    && target_workspace_id == workspace_id
                    && target_conversation_id == conversation_id
            }
            FocusPromotionTargetScope::Project {
                workspace_id: target_workspace_id,
                project_id,
            } => {
                !target_workspace_id.trim().is_empty()
                    && target_workspace_id == workspace_id
                    && !project_id.trim().is_empty()
            }
        };
        if !valid {
            return Err(FocusPromotionDecisionError::InvalidTarget);
        }
    }
    Ok(())
}

fn projection(
    input: &FocusPromotionDecisionCommandInput,
    candidates: &FocusPromotionCandidateSet,
    lifecycle: &FocusFrameLifecycleSnapshot,
    source_entity_revision: Option<u64>,
) -> FocusPromotionDecisionProjection {
    FocusPromotionDecisionProjection {
        contract_version: FOCUS_PROMOTION_DECISION_CONTRACT_VERSION.into(),
        decision_id: input.decision_id.clone(),
        focus_frame_id: input.focus_frame_id.clone(),
        conversation_id: candidates.conversation_id.clone(),
        candidate_ref: input.candidate_ref.clone(),
        action: input.action,
        target_scope: input.target_scope.clone(),
        promoted_entity_id: input.promoted_entity_id.clone(),
        source_entity_revision,
        decision_revision: 1,
        memory_version: candidates.memory_version,
        lifecycle_revision: lifecycle.revision,
        decided_at: input.decided_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        KnowledgeRetrievalContext, KnowledgeRetrievalDecision, close_focus_frame,
        contracts::{
            FOCUS_CONTRACT_VERSION, FocusBranchKind, FocusContextPolicy, FocusFrame,
            FocusMemoryScope, GeneratorRef, KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntityKind,
        },
        reopen_focus_frame, retrieval_decision,
    };

    fn actor() -> GeneratorRef {
        GeneratorRef {
            kind: GeneratorKind::User,
            generator_id: "user-1".into(),
            generator_version: "v1".into(),
        }
    }

    fn lifecycle() -> FocusFrameLifecycleSnapshot {
        let active = FocusFrameLifecycleSnapshot {
            contract_version: super::super::FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
            frame: FocusFrame {
                contract_version: FOCUS_CONTRACT_VERSION.into(),
                id: "focus-task-1".into(),
                conversation_id: "conversation-1".into(),
                parent_node_id: Some("node-1".into()),
                objective: "Return the branch result".into(),
                active_work_item: None,
                context_policy: FocusContextPolicy::BranchFromNode,
                memory_scope: FocusMemoryScope {
                    branch_kind: FocusBranchKind::Task,
                    inherit_refs: vec![],
                    local_refs: vec![],
                    exclude_refs: vec![],
                    promote_refs: vec!["entity-result-1".into()],
                },
                include_refs: vec![],
                exclude_refs: vec![],
                memory_version: 4,
                created_at: "2026-08-30T00:00:00Z".into(),
            },
            status: FocusFrameLifecycleStatus::Active,
            revision: 1,
            updated_at: "2026-08-30T00:00:00Z".into(),
            closed_at: None,
        };
        close_focus_frame(&active, "2026-08-30T01:00:00Z").expect("close branch")
    }

    fn candidate(status: KnowledgeStatus) -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "entity-result-1".into(),
            kind: KnowledgeEntityKind::Decision,
            name: "Keep the verified branch outcome".into(),
            aliases: vec![],
            scope: KnowledgeScope::FocusFrame {
                workspace_id: "workspace-1".into(),
                conversation_id: "conversation-1".into(),
                focus_frame_id: "focus-task-1".into(),
            },
            status,
            revision: 3,
            evidence: vec![],
            generator: GeneratorRef {
                kind: GeneratorKind::Model,
                generator_id: "extractor".into(),
                generator_version: "v1".into(),
            },
            created_at: "2026-08-30T00:30:00Z".into(),
            updated_at: "2026-08-30T00:30:00Z".into(),
        }
    }

    fn command(action: FocusPromotionDecisionAction) -> FocusPromotionDecisionCommandInput {
        FocusPromotionDecisionCommandInput {
            decision_id: "promotion-decision-1".into(),
            focus_frame_id: "focus-task-1".into(),
            candidate_ref: "entity-result-1".into(),
            expected_memory_version: 4,
            expected_lifecycle_revision: 2,
            expected_entity_revision: 3,
            expected_decision_revision: 0,
            action,
            target_scope: None,
            promoted_entity_id: None,
            decided_at: "2026-08-30T02:00:00Z".into(),
        }
    }

    #[test]
    fn confirm_builds_a_versioned_branch_local_mutation() {
        let lifecycle = lifecycle();
        let candidates = lifecycle
            .promotion_candidates()
            .expect("candidate projection")
            .expect("closed candidates");
        let plan = plan_focus_promotion_decision(
            &command(FocusPromotionDecisionAction::Confirm),
            &candidates,
            &lifecycle,
            &candidate(KnowledgeStatus::Candidate),
            &actor(),
        )
        .expect("confirm candidate");

        let FocusPromotionEntityMutation::UpsertSource(source) = plan.entity_mutation else {
            panic!("confirm must update the source entity");
        };
        assert_eq!(source.status, KnowledgeStatus::Confirmed);
        assert_eq!(source.revision, 4);
        assert!(matches!(source.scope, KnowledgeScope::FocusFrame { .. }));
        assert_eq!(plan.decision.source_entity_revision, Some(4));
    }

    #[test]
    fn promote_confirms_the_source_and_creates_a_target_copy() {
        let lifecycle = lifecycle();
        let candidates = lifecycle
            .promotion_candidates()
            .expect("candidate projection")
            .expect("closed candidates");
        let mut input = command(FocusPromotionDecisionAction::Promote);
        input.target_scope = Some(FocusPromotionTargetScope::Project {
            workspace_id: "workspace-1".into(),
            project_id: "project-1".into(),
        });
        input.promoted_entity_id = Some("entity-project-result-1".into());

        let plan = plan_focus_promotion_decision(
            &input,
            &candidates,
            &lifecycle,
            &candidate(KnowledgeStatus::Inferred),
            &actor(),
        )
        .expect("promote candidate");

        let FocusPromotionEntityMutation::Promote { source, promoted } = plan.entity_mutation
        else {
            panic!("promote must update the source and create the target");
        };
        assert_eq!(source.status, KnowledgeStatus::Confirmed);
        assert_eq!(promoted.id, "entity-project-result-1");
        assert_eq!(promoted.status, KnowledgeStatus::Confirmed);
        assert_eq!(promoted.revision, 1);
        assert!(matches!(
            promoted.scope,
            KnowledgeScope::Project { ref project_id, .. } if project_id == "project-1"
        ));
    }

    #[test]
    fn reject_plan_is_excluded_by_the_retrieval_gate() {
        let lifecycle = lifecycle();
        let candidates = lifecycle
            .promotion_candidates()
            .expect("candidate projection")
            .expect("closed candidates");
        let plan = plan_focus_promotion_decision(
            &command(FocusPromotionDecisionAction::Reject),
            &candidates,
            &lifecycle,
            &candidate(KnowledgeStatus::Candidate),
            &actor(),
        )
        .expect("reject candidate");
        let FocusPromotionEntityMutation::UpsertSource(source) = plan.entity_mutation else {
            panic!("reject must update the source entity");
        };
        let retrieval_context = KnowledgeRetrievalContext {
            workspace_id: "workspace-1".into(),
            project_id: Some("project-1".into()),
            conversation_id: "conversation-1".into(),
            focus_frame_id: "focus-task-1".into(),
        };

        assert_eq!(
            retrieval_decision(&source, &lifecycle.frame, &retrieval_context),
            KnowledgeRetrievalDecision::Excluded
        );
    }

    #[test]
    fn delete_builds_a_tombstoned_source_delete_plan() {
        let lifecycle = lifecycle();
        let candidates = lifecycle
            .promotion_candidates()
            .expect("candidate projection")
            .expect("closed candidates");
        let plan = plan_focus_promotion_decision(
            &command(FocusPromotionDecisionAction::Delete),
            &candidates,
            &lifecycle,
            &candidate(KnowledgeStatus::Candidate),
            &actor(),
        )
        .expect("delete candidate");

        assert_eq!(plan.decision.source_entity_revision, None);
        assert_eq!(
            plan.entity_mutation,
            FocusPromotionEntityMutation::DeleteSource {
                entity_id: "entity-result-1".into(),
                expected_revision: 3,
            }
        );
    }

    #[test]
    fn stale_versions_and_non_candidates_are_rejected() {
        let lifecycle = lifecycle();
        let candidates = lifecycle
            .promotion_candidates()
            .expect("candidate projection")
            .expect("closed candidates");
        let mut stale = command(FocusPromotionDecisionAction::Confirm);
        stale.expected_memory_version = 3;
        assert!(matches!(
            plan_focus_promotion_decision(
                &stale,
                &candidates,
                &lifecycle,
                &candidate(KnowledgeStatus::Candidate),
                &actor()
            ),
            Err(FocusPromotionDecisionError::StaleMemoryVersion { .. })
        ));

        assert_eq!(
            plan_focus_promotion_decision(
                &command(FocusPromotionDecisionAction::Confirm),
                &candidates,
                &lifecycle,
                &candidate(KnowledgeStatus::Confirmed),
                &actor()
            )
            .expect_err("confirmed entity cannot receive a new decision"),
            FocusPromotionDecisionError::InvalidCandidateStatus
        );
    }

    #[test]
    fn reopen_hides_candidates_and_prevents_a_late_decision() {
        let closed = lifecycle();
        let candidates = closed
            .promotion_candidates()
            .expect("candidate projection")
            .expect("closed candidates");
        let reopened = reopen_focus_frame(&closed, "2026-08-30T03:00:00Z").expect("reopen branch");

        assert_eq!(
            reopened
                .promotion_candidates()
                .expect("valid reopened lifecycle"),
            None
        );
        assert_eq!(
            plan_focus_promotion_decision(
                &command(FocusPromotionDecisionAction::Confirm),
                &candidates,
                &reopened,
                &candidate(KnowledgeStatus::Candidate),
                &actor()
            )
            .expect_err("active frame cannot accept decisions"),
            FocusPromotionDecisionError::FocusFrameNotClosed
        );
    }

    #[test]
    fn promote_rejects_cross_workspace_and_cross_conversation_targets() {
        let lifecycle = lifecycle();
        let candidates = lifecycle
            .promotion_candidates()
            .expect("candidate projection")
            .expect("closed candidates");
        let mut input = command(FocusPromotionDecisionAction::Promote);
        input.promoted_entity_id = Some("entity-promoted".into());
        input.target_scope = Some(FocusPromotionTargetScope::Conversation {
            workspace_id: "workspace-other".into(),
            conversation_id: "conversation-other".into(),
        });

        assert_eq!(
            plan_focus_promotion_decision(
                &input,
                &candidates,
                &lifecycle,
                &candidate(KnowledgeStatus::Candidate),
                &actor()
            )
            .expect_err("cross-scope target"),
            FocusPromotionDecisionError::InvalidTarget
        );
    }

    #[test]
    fn typed_command_serializes_the_frozen_ipc_shape() {
        let mut input = command(FocusPromotionDecisionAction::Promote);
        input.target_scope = Some(FocusPromotionTargetScope::Conversation {
            workspace_id: "workspace-1".into(),
            conversation_id: "conversation-1".into(),
        });
        input.promoted_entity_id = Some("entity-mainline-result-1".into());

        let value = serde_json::to_value(input).expect("serialize typed command");

        assert_eq!(value["decisionId"], "promotion-decision-1");
        assert_eq!(value["expectedMemoryVersion"], 4);
        assert_eq!(value["expectedDecisionRevision"], 0);
        assert_eq!(value["action"], "promote");
        assert_eq!(value["targetScope"]["type"], "conversation");
        assert_eq!(value["promotedEntityId"], "entity-mainline-result-1");
    }
}

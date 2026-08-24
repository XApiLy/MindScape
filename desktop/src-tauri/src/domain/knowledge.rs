use thiserror::Error;

use super::contracts::{
    FocusBranchKind, FocusFrame, GeneratorRef, KnowledgeEntity, KnowledgeScope, KnowledgeStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeAction {
    Infer,
    Confirm,
    Reject,
    Supersede,
    MarkStale,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum KnowledgeTransitionError {
    #[error("knowledge entity {entity_id} cannot transition from {status:?} via {action:?}")]
    InvalidTransition {
        entity_id: String,
        status: KnowledgeStatus,
        action: KnowledgeAction,
    },
    #[error("knowledge entity {entity_id} requires a different replacement entity")]
    InvalidReplacement { entity_id: String },
    #[error("knowledge entity {entity_id} revision overflowed")]
    RevisionOverflow { entity_id: String },
    #[error("knowledge entity update time must not be empty")]
    EmptyUpdatedAt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeTransition {
    pub action: KnowledgeAction,
    pub replacement_entity_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeRetrievalDecision {
    Confirmed,
    CandidateOnly,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeRetrievalContext {
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub conversation_id: String,
    pub focus_frame_id: String,
}

pub fn transition_entity(
    entity: &KnowledgeEntity,
    transition: &KnowledgeTransition,
    generator: &GeneratorRef,
    updated_at: &str,
) -> Result<KnowledgeEntity, KnowledgeTransitionError> {
    if updated_at.is_empty() {
        return Err(KnowledgeTransitionError::EmptyUpdatedAt);
    }

    let next_status = match (entity.status, transition.action) {
        (KnowledgeStatus::Candidate, KnowledgeAction::Infer) => KnowledgeStatus::Inferred,
        (KnowledgeStatus::Candidate | KnowledgeStatus::Inferred, KnowledgeAction::Confirm) => {
            KnowledgeStatus::Confirmed
        }
        (KnowledgeStatus::Candidate | KnowledgeStatus::Inferred, KnowledgeAction::Reject) => {
            KnowledgeStatus::Rejected
        }
        (
            KnowledgeStatus::Candidate
            | KnowledgeStatus::Inferred
            | KnowledgeStatus::Confirmed
            | KnowledgeStatus::Rejected
            | KnowledgeStatus::Superseded,
            KnowledgeAction::MarkStale,
        ) => KnowledgeStatus::Stale,
        (KnowledgeStatus::Confirmed, KnowledgeAction::Supersede) => {
            let Some(replacement) = transition.replacement_entity_id.as_deref() else {
                return Err(KnowledgeTransitionError::InvalidReplacement {
                    entity_id: entity.id.clone(),
                });
            };
            if replacement == entity.id || replacement.is_empty() {
                return Err(KnowledgeTransitionError::InvalidReplacement {
                    entity_id: entity.id.clone(),
                });
            }
            KnowledgeStatus::Superseded
        }
        (status, action) => {
            return Err(KnowledgeTransitionError::InvalidTransition {
                entity_id: entity.id.clone(),
                status,
                action,
            });
        }
    };

    let revision = entity.revision.checked_add(1).ok_or_else(|| {
        KnowledgeTransitionError::RevisionOverflow {
            entity_id: entity.id.clone(),
        }
    })?;
    let mut updated = entity.clone();
    updated.status = next_status;
    updated.revision = revision;
    updated.generator = generator.clone();
    updated.updated_at = updated_at.into();
    Ok(updated)
}

pub fn retrieval_decision(
    entity: &KnowledgeEntity,
    focus_frame: &FocusFrame,
    context: &KnowledgeRetrievalContext,
) -> KnowledgeRetrievalDecision {
    if focus_frame.id != context.focus_frame_id {
        return KnowledgeRetrievalDecision::Excluded;
    }

    if matches!(
        entity.status,
        KnowledgeStatus::Rejected | KnowledgeStatus::Superseded | KnowledgeStatus::Stale
    ) {
        return KnowledgeRetrievalDecision::Excluded;
    }

    let explicit = focus_frame
        .include_refs
        .iter()
        .chain(&focus_frame.memory_scope.inherit_refs)
        .chain(&focus_frame.memory_scope.local_refs)
        .any(|reference| reference == &entity.id);
    let excluded = focus_frame
        .exclude_refs
        .iter()
        .chain(&focus_frame.memory_scope.exclude_refs)
        .any(|reference| reference == &entity.id);
    if excluded || !scope_matches(&entity.scope, focus_frame, context) {
        return KnowledgeRetrievalDecision::Excluded;
    }

    let implicit_scope_allowed = focus_frame.memory_scope.branch_kind == FocusBranchKind::Mainline;
    if !explicit && !implicit_scope_allowed {
        return KnowledgeRetrievalDecision::Excluded;
    }

    if entity.status == KnowledgeStatus::Confirmed {
        KnowledgeRetrievalDecision::Confirmed
    } else {
        KnowledgeRetrievalDecision::CandidateOnly
    }
}

fn scope_matches(
    scope: &KnowledgeScope,
    focus_frame: &FocusFrame,
    context: &KnowledgeRetrievalContext,
) -> bool {
    match scope {
        KnowledgeScope::Workspace { workspace_id } => workspace_id == &context.workspace_id,
        KnowledgeScope::Project {
            workspace_id,
            project_id,
        } => {
            workspace_id == &context.workspace_id && context.project_id.as_ref() == Some(project_id)
        }
        KnowledgeScope::Conversation {
            workspace_id,
            conversation_id,
        } => workspace_id == &context.workspace_id && conversation_id == &context.conversation_id,
        KnowledgeScope::FocusFrame {
            workspace_id,
            conversation_id,
            focus_frame_id,
        } => {
            workspace_id == &context.workspace_id
                && conversation_id == &context.conversation_id
                && focus_frame_id == &focus_frame.id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{
        FOCUS_CONTRACT_VERSION, FocusContextPolicy, FocusMemoryScope, GeneratorKind,
        KnowledgeEntityKind,
    };

    fn generator(kind: GeneratorKind, id: &str) -> GeneratorRef {
        GeneratorRef {
            kind,
            generator_id: id.into(),
            generator_version: "v1".into(),
        }
    }

    fn entity(status: KnowledgeStatus, scope: KnowledgeScope) -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: "mindscape.knowledge.v1".into(),
            id: "entity-1".into(),
            kind: KnowledgeEntityKind::Decision,
            name: "Use FocusFrame".into(),
            aliases: vec![],
            scope,
            status,
            revision: 1,
            evidence: vec![],
            generator: generator(GeneratorKind::Model, "extractor"),
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T00:00:00Z".into(),
        }
    }

    fn frame(kind: FocusBranchKind) -> FocusFrame {
        FocusFrame {
            contract_version: FOCUS_CONTRACT_VERSION.into(),
            id: "focus-1".into(),
            conversation_id: "conversation-1".into(),
            parent_node_id: Some("node-1".into()),
            objective: "test focus".into(),
            active_work_item: None,
            context_policy: FocusContextPolicy::FocusNew,
            memory_scope: FocusMemoryScope {
                branch_kind: kind,
                inherit_refs: vec![],
                local_refs: vec![],
                exclude_refs: vec![],
                promote_refs: vec![],
            },
            include_refs: vec![],
            exclude_refs: vec![],
            memory_version: 1,
            created_at: "2026-08-24T00:00:00Z".into(),
        }
    }

    fn context() -> KnowledgeRetrievalContext {
        KnowledgeRetrievalContext {
            workspace_id: "workspace-1".into(),
            project_id: Some("project-1".into()),
            conversation_id: "conversation-1".into(),
            focus_frame_id: "focus-1".into(),
        }
    }

    #[test]
    fn candidate_can_be_confirmed_and_revision_is_incremented() {
        let candidate = entity(
            KnowledgeStatus::Candidate,
            KnowledgeScope::Project {
                workspace_id: "workspace-1".into(),
                project_id: "project-1".into(),
            },
        );
        let confirmed = transition_entity(
            &candidate,
            &KnowledgeTransition {
                action: KnowledgeAction::Confirm,
                replacement_entity_id: None,
            },
            &generator(GeneratorKind::User, "user-confirmation"),
            "2026-08-24T01:00:00Z",
        )
        .expect("confirm candidate");

        assert_eq!(confirmed.status, KnowledgeStatus::Confirmed);
        assert_eq!(confirmed.revision, 2);
        assert_eq!(confirmed.generator.kind, GeneratorKind::User);
    }

    #[test]
    fn confirmed_entity_can_only_be_superseded_or_staled() {
        let confirmed = entity(
            KnowledgeStatus::Confirmed,
            KnowledgeScope::Project {
                workspace_id: "workspace-1".into(),
                project_id: "project-1".into(),
            },
        );
        let error = transition_entity(
            &confirmed,
            &KnowledgeTransition {
                action: KnowledgeAction::Reject,
                replacement_entity_id: None,
            },
            &generator(GeneratorKind::User, "user"),
            "2026-08-24T01:00:00Z",
        )
        .expect_err("rejecting a confirmed fact must be explicit through a replacement");
        assert!(matches!(
            error,
            KnowledgeTransitionError::InvalidTransition {
                status: KnowledgeStatus::Confirmed,
                action: KnowledgeAction::Reject,
                ..
            }
        ));

        let superseded = transition_entity(
            &confirmed,
            &KnowledgeTransition {
                action: KnowledgeAction::Supersede,
                replacement_entity_id: Some("entity-2".into()),
            },
            &generator(GeneratorKind::User, "user"),
            "2026-08-24T01:00:00Z",
        )
        .expect("supersede confirmed fact");
        assert_eq!(superseded.status, KnowledgeStatus::Superseded);
    }

    #[test]
    fn rejected_entity_cannot_be_revived_by_a_later_confirm() {
        let rejected = entity(
            KnowledgeStatus::Rejected,
            KnowledgeScope::Project {
                workspace_id: "workspace-1".into(),
                project_id: "project-1".into(),
            },
        );
        let error = transition_entity(
            &rejected,
            &KnowledgeTransition {
                action: KnowledgeAction::Confirm,
                replacement_entity_id: None,
            },
            &generator(GeneratorKind::User, "user"),
            "2026-08-24T01:00:00Z",
        )
        .expect_err("rejected facts cannot be revived");
        assert!(matches!(
            error,
            KnowledgeTransitionError::InvalidTransition { .. }
        ));
    }

    #[test]
    fn task_focus_does_not_recall_project_fact_without_explicit_inheritance() {
        let project_fact = entity(
            KnowledgeStatus::Confirmed,
            KnowledgeScope::Project {
                workspace_id: "workspace-1".into(),
                project_id: "project-1".into(),
            },
        );
        let task = frame(FocusBranchKind::Task);
        assert_eq!(
            retrieval_decision(&project_fact, &task, &context()),
            KnowledgeRetrievalDecision::Excluded
        );

        let mut inherited_task = task;
        inherited_task.memory_scope.inherit_refs = vec!["entity-1".into()];
        assert_eq!(
            retrieval_decision(&project_fact, &inherited_task, &context()),
            KnowledgeRetrievalDecision::Confirmed
        );
    }

    #[test]
    fn candidate_is_never_returned_as_a_confirmed_fact() {
        let candidate = entity(
            KnowledgeStatus::Candidate,
            KnowledgeScope::Project {
                workspace_id: "workspace-1".into(),
                project_id: "project-1".into(),
            },
        );
        let mut task = frame(FocusBranchKind::Task);
        task.include_refs = vec!["entity-1".into()];
        assert_eq!(
            retrieval_decision(&candidate, &task, &context()),
            KnowledgeRetrievalDecision::CandidateOnly
        );
    }

    #[test]
    fn rejected_or_excluded_entities_never_reach_retrieval() {
        let rejected = entity(
            KnowledgeStatus::Rejected,
            KnowledgeScope::Project {
                workspace_id: "workspace-1".into(),
                project_id: "project-1".into(),
            },
        );
        let mut mainline = frame(FocusBranchKind::Mainline);
        mainline.exclude_refs = vec!["entity-1".into()];
        assert_eq!(
            retrieval_decision(&rejected, &mainline, &context()),
            KnowledgeRetrievalDecision::Excluded
        );
    }

    #[test]
    fn entity_from_another_focus_frame_is_excluded() {
        let project_fact = entity(
            KnowledgeStatus::Confirmed,
            KnowledgeScope::FocusFrame {
                workspace_id: "workspace-1".into(),
                conversation_id: "conversation-1".into(),
                focus_frame_id: "focus-1".into(),
            },
        );
        let mut other_frame = frame(FocusBranchKind::Mainline);
        other_frame.id = "focus-2".into();
        let mut other_context = context();
        other_context.focus_frame_id = "focus-2".into();

        assert_eq!(
            retrieval_decision(&project_fact, &other_frame, &other_context),
            KnowledgeRetrievalDecision::Excluded
        );
    }
}

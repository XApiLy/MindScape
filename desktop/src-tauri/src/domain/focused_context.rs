use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    ContextCompileInput, ContextSnapshot, ContextTurn, KernelError, KernelResult,
    OmittedContextRef, compile_context,
    context::SYSTEM_CONTRACT_VERSION,
    contracts::{FocusContextPolicy, FocusFrame, OmittedFocusRef},
    knowledge::{
        KnowledgeContextCompileInput, KnowledgeContextSelection, compile_knowledge_context,
    },
};

pub const FOCUSED_CONTEXT_CONTRACT_VERSION: &str = "mindscape.focused-context.v1";

#[derive(Debug, Clone)]
pub struct FocusedContextCompileInput {
    pub context: ContextCompileInput,
    pub focus_frame: FocusFrame,
    pub knowledge: Option<KnowledgeContextCompileInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusedContextSnapshot {
    pub contract_version: String,
    pub focus_frame: FocusFrame,
    pub context_snapshot: ContextSnapshot,
    pub selected_memory_refs: Vec<String>,
    pub omitted_memory_refs: Vec<OmittedFocusRef>,
    pub knowledge_context: Option<KnowledgeContextSelection>,
}

/// Validate a persisted or queried focused-context snapshot before it crosses
/// the application/UI boundary. This keeps the immutable frame and the
/// derived context from being silently stitched across conversations or
/// revisions by storage adapters.
pub fn validate_focused_context_snapshot(snapshot: &FocusedContextSnapshot) -> KernelResult<()> {
    if snapshot.contract_version != FOCUSED_CONTEXT_CONTRACT_VERSION {
        return Err(KernelError::Validation(format!(
            "unsupported FocusedContext contract version: {}",
            snapshot.contract_version
        )));
    }
    snapshot.focus_frame.validate()?;

    let context = &snapshot.context_snapshot;
    for (field, value) in [
        ("ContextSnapshot id", context.id.as_str()),
        (
            "ContextSnapshot conversation id",
            context.conversation_id.as_str(),
        ),
        ("ContextSnapshot created at", context.created_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(KernelError::Validation(format!(
                "{field} must not be empty"
            )));
        }
    }
    if context.system_contract_version != SYSTEM_CONTRACT_VERSION {
        return Err(KernelError::Validation(format!(
            "unsupported ContextSnapshot contract version: {}",
            context.system_contract_version
        )));
    }
    if context.estimated_tokens < 0 {
        return Err(KernelError::Validation(
            "ContextSnapshot estimated tokens must not be negative".into(),
        ));
    }
    if context.conversation_id != snapshot.focus_frame.conversation_id {
        return Err(KernelError::Integrity(
            "FocusedContext snapshot belongs to a different conversation".into(),
        ));
    }
    if context.parent_node_id != snapshot.focus_frame.parent_node_id {
        return Err(KernelError::Integrity(
            "FocusedContext snapshot references a different parent node".into(),
        ));
    }

    let mut memory_refs = HashSet::new();
    for reference in &snapshot.selected_memory_refs {
        if reference.trim().is_empty() || !memory_refs.insert(reference.as_str()) {
            return Err(KernelError::Validation(
                "FocusedContext selected memory references must be non-empty and unique".into(),
            ));
        }
    }
    for reference in &snapshot.omitted_memory_refs {
        if reference.reference_id.trim().is_empty() || reference.reason.trim().is_empty() {
            return Err(KernelError::Validation(
                "FocusedContext omitted memory references require an id and reason".into(),
            ));
        }
        if !memory_refs.insert(reference.reference_id.as_str()) {
            return Err(KernelError::Validation(format!(
                "FocusedContext memory reference appears in both selected and omitted sets: {}",
                reference.reference_id
            )));
        }
    }

    if let Some(knowledge) = &snapshot.knowledge_context {
        validate_knowledge_selection(knowledge)?;
    }
    Ok(())
}

fn validate_knowledge_selection(selection: &KnowledgeContextSelection) -> KernelResult<()> {
    if selection.contract_version != super::knowledge::KNOWLEDGE_CONTEXT_CONTRACT_VERSION {
        return Err(KernelError::Validation(format!(
            "unsupported KnowledgeContext contract version: {}",
            selection.contract_version
        )));
    }
    if selection.retrieval_version.trim().is_empty() {
        return Err(KernelError::Validation(
            "KnowledgeContext retrieval version must not be empty".into(),
        ));
    }
    if selection.estimated_tokens < 0 {
        return Err(KernelError::Validation(
            "KnowledgeContext estimated tokens must not be negative".into(),
        ));
    }

    let mut references = HashSet::new();
    let mut selected_tokens = 0_i64;
    for reference in &selection.selected {
        if reference.entity_id.trim().is_empty() || reference.estimated_tokens <= 0 {
            return Err(KernelError::Validation(
                "KnowledgeContext selected references require an id and positive token estimate"
                    .into(),
            ));
        }
        if !references.insert(reference.entity_id.as_str()) {
            return Err(KernelError::Validation(format!(
                "KnowledgeContext reference appears more than once: {}",
                reference.entity_id
            )));
        }
        selected_tokens = selected_tokens
            .checked_add(reference.estimated_tokens)
            .ok_or_else(|| {
                KernelError::Validation("KnowledgeContext token estimate overflowed".into())
            })?;
    }
    for omitted in &selection.omitted {
        if omitted.reference_id.trim().is_empty() || omitted.reason.trim().is_empty() {
            return Err(KernelError::Validation(
                "KnowledgeContext omitted references require an id and reason".into(),
            ));
        }
        if !references.insert(omitted.reference_id.as_str()) {
            return Err(KernelError::Validation(format!(
                "KnowledgeContext reference appears in both selected and omitted sets: {}",
                omitted.reference_id
            )));
        }
    }
    if selected_tokens != selection.estimated_tokens {
        return Err(KernelError::Integrity(
            "KnowledgeContext estimated tokens do not match selected references".into(),
        ));
    }
    Ok(())
}

pub fn compile_focused_context(
    input: FocusedContextCompileInput,
) -> KernelResult<FocusedContextSnapshot> {
    validate_focus_frame(&input.focus_frame, &input.context)?;
    let knowledge_context = input
        .knowledge
        .map(|knowledge| compile_knowledge_context(knowledge, &input.focus_frame))
        .transpose()
        .map_err(|error| KernelError::Validation(error.to_string()))?;

    let included_refs = included_refs(&input.focus_frame);
    let excluded_refs = excluded_refs(&input.focus_frame);
    let select_all = input.focus_frame.context_policy == FocusContextPolicy::ContinueCurrent;
    let mut selected_memory_refs = Vec::new();
    let mut omitted_memory_refs = Vec::new();
    let mut omitted_messages = Vec::new();
    let mut selected_path = Vec::new();

    for turn in input.context.path {
        let turn_refs = turn_refs(&turn);
        let excluded = turn_refs
            .iter()
            .any(|reference| excluded_refs.contains(reference.as_str()));
        let included = select_all
            || turn_refs
                .iter()
                .any(|reference| included_refs.contains(reference.as_str()));

        if excluded || !included {
            let reason = if excluded {
                "excluded by the FocusFrame memory scope"
            } else {
                "not selected by the FocusFrame context policy"
            };
            record_omitted_turn(&turn, reason, &mut omitted_messages);
            omitted_memory_refs.extend(turn_refs.into_iter().map(|reference_id| OmittedFocusRef {
                reference_id,
                reason: reason.into(),
            }));
            continue;
        }

        selected_memory_refs.extend(
            turn_refs
                .iter()
                .filter(|reference| included_refs.contains(reference.as_str()))
                .cloned(),
        );
        selected_path.push(turn);
    }

    selected_memory_refs.extend(
        input
            .focus_frame
            .memory_scope
            .local_refs
            .iter()
            .filter(|reference| !excluded_refs.contains(reference.as_str()))
            .cloned(),
    );
    selected_memory_refs.sort();
    selected_memory_refs.dedup();
    omitted_memory_refs.sort_by(|left, right| left.reference_id.cmp(&right.reference_id));
    omitted_memory_refs.dedup_by(|left, right| left.reference_id == right.reference_id);

    let mut context_snapshot = compile_context(ContextCompileInput {
        path: selected_path,
        ..input.context
    })?;
    context_snapshot
        .omitted_messages
        .splice(0..0, omitted_messages);

    Ok(FocusedContextSnapshot {
        contract_version: FOCUSED_CONTEXT_CONTRACT_VERSION.into(),
        focus_frame: input.focus_frame,
        context_snapshot,
        selected_memory_refs,
        omitted_memory_refs,
        knowledge_context,
    })
}

fn validate_focus_frame(frame: &FocusFrame, context: &ContextCompileInput) -> KernelResult<()> {
    frame.validate()?;
    if frame.conversation_id != context.conversation_id {
        return Err(KernelError::Validation(
            "FocusFrame and context must belong to the same conversation".into(),
        ));
    }
    if frame.parent_node_id != context.parent_node_id {
        return Err(KernelError::Validation(
            "FocusFrame and context must reference the same parent node".into(),
        ));
    }
    Ok(())
}

fn included_refs(frame: &FocusFrame) -> HashSet<&str> {
    frame
        .memory_scope
        .inherit_refs
        .iter()
        .chain(&frame.memory_scope.local_refs)
        .chain(&frame.include_refs)
        .map(String::as_str)
        .collect()
}

fn excluded_refs(frame: &FocusFrame) -> HashSet<&str> {
    frame
        .memory_scope
        .exclude_refs
        .iter()
        .chain(&frame.exclude_refs)
        .map(String::as_str)
        .collect()
}

fn turn_refs(turn: &ContextTurn) -> Vec<String> {
    let mut references = vec![turn.node_id.clone(), turn.user_message_id.clone()];
    if let Some(message_id) = &turn.assistant_message_id {
        references.push(message_id.clone());
    }
    references
}

fn record_omitted_turn(turn: &ContextTurn, reason: &str, omitted: &mut Vec<OmittedContextRef>) {
    omitted.push(OmittedContextRef {
        message_id: turn.user_message_id.clone(),
        reason: reason.into(),
    });
    if let Some(message_id) = &turn.assistant_message_id {
        omitted.push(OmittedContextRef {
            message_id: message_id.clone(),
            reason: reason.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{
        FOCUS_CONTRACT_VERSION, FocusBranchKind, FocusMemoryScope, GeneratorKind, GeneratorRef,
        KnowledgeEntity, KnowledgeEntityKind, KnowledgeScope, KnowledgeStatus,
    };
    use crate::domain::{
        BranchType, ContentBlock, KnowledgeContextCompileInput, KnowledgeRetrievalCandidate,
        KnowledgeRetrievalContext,
    };

    fn turn(id: &str) -> ContextTurn {
        ContextTurn {
            node_id: format!("node-{id}"),
            user_message_id: format!("message-{id}-user"),
            user_content_blocks: vec![ContentBlock::text(format!("question {id}"))],
            assistant_message_id: Some(format!("message-{id}-assistant")),
            assistant_content_blocks: Some(vec![ContentBlock::text(format!("answer {id}"))]),
        }
    }

    fn frame(id: &str, policy: FocusContextPolicy) -> FocusFrame {
        FocusFrame {
            contract_version: FOCUS_CONTRACT_VERSION.into(),
            id: id.into(),
            conversation_id: "conversation-1".into(),
            parent_node_id: Some("node-2".into()),
            objective: format!("objective {id}"),
            active_work_item: None,
            context_policy: policy,
            memory_scope: FocusMemoryScope {
                branch_kind: FocusBranchKind::Task,
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

    fn context() -> ContextCompileInput {
        ContextCompileInput {
            conversation_id: "conversation-1".into(),
            parent_node_id: Some("node-2".into()),
            branch_type: BranchType::Deepens,
            current_input: "next".into(),
            path: vec![turn("1"), turn("2")],
            max_context_tokens: None,
        }
    }

    fn knowledge_candidate() -> KnowledgeRetrievalCandidate {
        KnowledgeRetrievalCandidate {
            entity: KnowledgeEntity {
                contract_version: "mindscape.knowledge.v1".into(),
                id: "entity-1".into(),
                kind: KnowledgeEntityKind::Decision,
                name: "Use FocusFrame".into(),
                aliases: vec![],
                scope: KnowledgeScope::Project {
                    workspace_id: "workspace-1".into(),
                    project_id: "project-1".into(),
                },
                status: KnowledgeStatus::Confirmed,
                revision: 2,
                evidence: vec![],
                generator: GeneratorRef {
                    kind: GeneratorKind::User,
                    generator_id: "user".into(),
                    generator_version: "v1".into(),
                },
                created_at: "2026-08-24T00:00:00Z".into(),
                updated_at: "2026-08-24T01:00:00Z".into(),
            },
            evidence: vec![],
            retrieval_score: 100,
            estimated_tokens: 4,
        }
    }

    #[test]
    fn same_parent_with_different_focus_frames_selects_different_context() {
        let continued = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame: frame("focus-continue", FocusContextPolicy::ContinueCurrent),
            knowledge: None,
        })
        .expect("compile continued focus");
        let mut focused_frame = frame("focus-new", FocusContextPolicy::FocusNew);
        focused_frame.memory_scope.inherit_refs = vec!["node-1".into()];
        focused_frame.exclude_refs = vec!["node-2".into()];
        let focused = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame: focused_frame,
            knowledge: None,
        })
        .expect("compile new focus");

        assert_eq!(
            continued.context_snapshot.parent_node_id,
            focused.context_snapshot.parent_node_id
        );
        assert_eq!(continued.context_snapshot.selected_messages.len(), 4);
        assert_eq!(focused.context_snapshot.selected_messages.len(), 2);
        assert!(
            focused
                .context_snapshot
                .selected_messages
                .iter()
                .all(|message| message.source_node_id == "node-1")
        );
        assert!(
            focused
                .omitted_memory_refs
                .iter()
                .any(|omitted| omitted.reference_id == "node-2")
        );
    }

    #[test]
    fn excluded_message_drops_the_complete_turn() {
        let mut focus_frame = frame("focus-exclude", FocusContextPolicy::ContinueCurrent);
        focus_frame.exclude_refs = vec!["message-2-assistant".into()];
        let snapshot = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame,
            knowledge: None,
        })
        .expect("compile excluded focus");

        assert_eq!(snapshot.context_snapshot.selected_messages.len(), 2);
        assert!(
            snapshot
                .context_snapshot
                .selected_messages
                .iter()
                .all(|message| message.source_node_id == "node-1")
        );
    }

    #[test]
    fn overlapping_memory_sets_are_rejected() {
        let mut focus_frame = frame("focus-invalid", FocusContextPolicy::FocusNew);
        focus_frame.memory_scope.inherit_refs = vec!["entity-1".into()];
        focus_frame.exclude_refs = vec!["entity-1".into()];
        let error = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame,
            knowledge: None,
        })
        .expect_err("reject overlapping memory sets");

        assert!(error.to_string().contains("more than one set"));
    }

    #[test]
    fn focused_snapshot_carries_a_budgeted_knowledge_selection() {
        let mut focus_frame = frame("focus-knowledge", FocusContextPolicy::FocusNew);
        focus_frame.include_refs = vec!["entity-1".into()];
        let snapshot = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame,
            knowledge: Some(KnowledgeContextCompileInput {
                candidates: vec![knowledge_candidate()],
                retrieval_context: KnowledgeRetrievalContext {
                    workspace_id: "workspace-1".into(),
                    project_id: Some("project-1".into()),
                    conversation_id: "conversation-1".into(),
                    focus_frame_id: "focus-knowledge".into(),
                },
                retrieval_version: "fts-v1".into(),
                max_tokens: Some(8),
            }),
        })
        .expect("compile focused knowledge context");

        let knowledge = snapshot
            .knowledge_context
            .expect("knowledge context selection");
        assert_eq!(knowledge.estimated_tokens, 4);
        assert_eq!(knowledge.selected[0].entity_id, "entity-1");
        assert!(knowledge.omitted.is_empty());
    }

    #[test]
    fn persisted_snapshot_validation_accepts_compiled_context_and_knowledge() {
        let mut focus_frame = frame("focus-valid", FocusContextPolicy::FocusNew);
        focus_frame.include_refs = vec!["entity-1".into()];
        let snapshot = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame,
            knowledge: Some(KnowledgeContextCompileInput {
                candidates: vec![knowledge_candidate()],
                retrieval_context: KnowledgeRetrievalContext {
                    workspace_id: "workspace-1".into(),
                    project_id: Some("project-1".into()),
                    conversation_id: "conversation-1".into(),
                    focus_frame_id: "focus-valid".into(),
                },
                retrieval_version: "fts-v1".into(),
                max_tokens: Some(8),
            }),
        })
        .expect("compile focused snapshot");

        validate_focused_context_snapshot(&snapshot).expect("validate compiled snapshot");
    }

    #[test]
    fn persisted_snapshot_validation_rejects_a_parent_node_mismatch() {
        let snapshot = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame: frame("focus-parent", FocusContextPolicy::ContinueCurrent),
            knowledge: None,
        })
        .expect("compile focused snapshot");
        let mut invalid = snapshot;
        invalid.context_snapshot.parent_node_id = Some("node-other".into());

        let error = validate_focused_context_snapshot(&invalid)
            .expect_err("reject a snapshot stitched to another parent");
        assert!(error.to_string().contains("different parent node"));
    }

    #[test]
    fn persisted_snapshot_validation_rejects_selected_knowledge_token_drift() {
        let mut focus_frame = frame("focus-token-drift", FocusContextPolicy::FocusNew);
        focus_frame.include_refs = vec!["entity-1".into()];
        let mut snapshot = compile_focused_context(FocusedContextCompileInput {
            context: context(),
            focus_frame,
            knowledge: Some(KnowledgeContextCompileInput {
                candidates: vec![knowledge_candidate()],
                retrieval_context: KnowledgeRetrievalContext {
                    workspace_id: "workspace-1".into(),
                    project_id: Some("project-1".into()),
                    conversation_id: "conversation-1".into(),
                    focus_frame_id: "focus-token-drift".into(),
                },
                retrieval_version: "fts-v1".into(),
                max_tokens: Some(8),
            }),
        })
        .expect("compile focused snapshot");
        snapshot
            .knowledge_context
            .as_mut()
            .expect("knowledge selection")
            .estimated_tokens = 999;

        let error = validate_focused_context_snapshot(&snapshot)
            .expect_err("reject stale selected-token aggregate");
        assert!(error.to_string().contains("estimated tokens do not match"));
    }
}

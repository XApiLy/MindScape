use serde::{Deserialize, Serialize};

use super::{
    FocusFrameLifecycleSnapshot, FocusedContextSnapshot, KernelError, KernelResult,
    validate_focused_context_snapshot,
};

pub const FOCUS_QUERY_CONTRACT_VERSION: &str = "mindscape.focus-query.v1";

/// Read-only projection consumed by UI/query adapters.
///
/// The lifecycle snapshot is authoritative for frame status. A focused context
/// snapshot is optional because a frame can exist before its first compilation,
/// or after the last compiled snapshot has been archived.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusFrameQueryProjection {
    pub contract_version: String,
    pub lifecycle: FocusFrameLifecycleSnapshot,
    pub focused_context: Option<FocusedContextSnapshot>,
}

pub fn validate_focus_frame_query_projection(
    projection: &FocusFrameQueryProjection,
) -> KernelResult<()> {
    if projection.contract_version != FOCUS_QUERY_CONTRACT_VERSION {
        return Err(KernelError::Validation(format!(
            "unsupported FocusFrame query contract version: {}",
            projection.contract_version
        )));
    }
    if projection.lifecycle.frame.id.trim().is_empty() {
        return Err(KernelError::Validation(
            "FocusFrame query requires a non-empty frame id".into(),
        ));
    }
    if let Some(context) = &projection.focused_context {
        validate_focused_context_snapshot(context)?;
        if context.focus_frame.id != projection.lifecycle.frame.id {
            return Err(KernelError::Integrity(
                "FocusFrame query lifecycle and context refer to different frames".into(),
            ));
        }
        if context.focus_frame.conversation_id != projection.lifecycle.frame.conversation_id {
            return Err(KernelError::Integrity(
                "FocusFrame query lifecycle and context refer to different conversations".into(),
            ));
        }
        if context.context_snapshot.conversation_id != projection.lifecycle.frame.conversation_id {
            return Err(KernelError::Integrity(
                "FocusedContext snapshot belongs to a different conversation".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{
        FOCUS_CONTRACT_VERSION, FocusBranchKind, FocusContextPolicy, FocusFrame, FocusMemoryScope,
    };

    fn projection() -> FocusFrameQueryProjection {
        let frame = FocusFrame {
            contract_version: FOCUS_CONTRACT_VERSION.into(),
            id: "focus-1".into(),
            conversation_id: "conversation-1".into(),
            parent_node_id: None,
            objective: "Review imported context".into(),
            active_work_item: None,
            context_policy: FocusContextPolicy::FocusNew,
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
            created_at: "2026-08-25T00:00:00Z".into(),
        };
        FocusFrameQueryProjection {
            contract_version: FOCUS_QUERY_CONTRACT_VERSION.into(),
            lifecycle: FocusFrameLifecycleSnapshot {
                contract_version: super::super::FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
                frame,
                status: super::super::FocusFrameLifecycleStatus::Active,
                revision: 1,
                updated_at: "2026-08-25T00:00:00Z".into(),
                closed_at: None,
            },
            focused_context: None,
        }
    }

    #[test]
    fn accepts_a_frame_without_a_compiled_context() {
        validate_focus_frame_query_projection(&projection()).expect("valid query projection");
    }

    #[test]
    fn rejects_unknown_query_contract_versions() {
        let mut query = projection();
        query.contract_version = "mindscape.focus-query.v0".into();
        let error = validate_focus_frame_query_projection(&query).expect_err("reject old query");
        assert!(
            error
                .to_string()
                .contains("unsupported FocusFrame query contract version")
        );
    }

    #[test]
    fn rejects_empty_frame_ids_before_ui_consumes_the_projection() {
        let mut query = projection();
        query.lifecycle.frame.id.clear();
        let error = validate_focus_frame_query_projection(&query).expect_err("reject empty id");
        assert!(error.to_string().contains("non-empty frame id"));
    }
}

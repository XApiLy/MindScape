use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{KernelError, KernelResult, contracts::FocusFrame};

pub const FOCUS_LIFECYCLE_CONTRACT_VERSION: &str = "mindscape.focus-lifecycle.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FocusFrameLifecycleStatus {
    Active,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusFrameLifecycleSnapshot {
    pub contract_version: String,
    pub frame: FocusFrame,
    pub status: FocusFrameLifecycleStatus,
    pub revision: u64,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

impl FocusFrameLifecycleSnapshot {
    /// Validate a lifecycle record restored from storage before it is exposed
    /// as query truth. Lifecycle transitions only operate on valid snapshots;
    /// this also prevents an active frame with stale close metadata from
    /// leaking into the UI after restart.
    pub fn validate(&self) -> KernelResult<()> {
        if self.contract_version != FOCUS_LIFECYCLE_CONTRACT_VERSION {
            return Err(KernelError::Validation(format!(
                "unsupported FocusFrame lifecycle contract version: {}",
                self.contract_version
            )));
        }
        self.frame.validate()?;
        if self.revision == 0 {
            return Err(KernelError::Validation(
                "FocusFrame lifecycle revision must be greater than zero".into(),
            ));
        }
        if self.updated_at.trim().is_empty() {
            return Err(KernelError::Validation(
                "FocusFrame lifecycle updated at must not be empty".into(),
            ));
        }
        match (self.status, self.closed_at.as_deref()) {
            (FocusFrameLifecycleStatus::Active, None) => Ok(()),
            (FocusFrameLifecycleStatus::Active, Some(_)) => Err(KernelError::Integrity(
                "active FocusFrame lifecycle cannot have closedAt".into(),
            )),
            (FocusFrameLifecycleStatus::Closed, Some(closed_at))
                if !closed_at.trim().is_empty() =>
            {
                Ok(())
            }
            (FocusFrameLifecycleStatus::Closed, _) => Err(KernelError::Validation(
                "closed FocusFrame lifecycle requires a non-empty closedAt".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusFrameLifecycleCommandInput {
    pub focus_frame_id: String,
    pub expected_revision: u64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusFrameLifecycleAction {
    Close,
    Reopen,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FocusFrameLifecycleError {
    #[error("focus frame {focus_frame_id} cannot transition from {status:?} via {action:?}")]
    InvalidTransition {
        focus_frame_id: String,
        status: FocusFrameLifecycleStatus,
        action: FocusFrameLifecycleAction,
    },
    #[error("focus frame {focus_frame_id} revision overflowed")]
    RevisionOverflow { focus_frame_id: String },
    #[error("focus frame lifecycle update time must not be empty")]
    EmptyUpdatedAt,
}

pub fn close_focus_frame(
    snapshot: &FocusFrameLifecycleSnapshot,
    updated_at: &str,
) -> Result<FocusFrameLifecycleSnapshot, FocusFrameLifecycleError> {
    transition_focus_frame(snapshot, FocusFrameLifecycleAction::Close, updated_at)
}

pub fn reopen_focus_frame(
    snapshot: &FocusFrameLifecycleSnapshot,
    updated_at: &str,
) -> Result<FocusFrameLifecycleSnapshot, FocusFrameLifecycleError> {
    transition_focus_frame(snapshot, FocusFrameLifecycleAction::Reopen, updated_at)
}

pub fn transition_focus_frame(
    snapshot: &FocusFrameLifecycleSnapshot,
    action: FocusFrameLifecycleAction,
    updated_at: &str,
) -> Result<FocusFrameLifecycleSnapshot, FocusFrameLifecycleError> {
    if updated_at.trim().is_empty() {
        return Err(FocusFrameLifecycleError::EmptyUpdatedAt);
    }

    let (status, closed_at) = match (snapshot.status, action) {
        (FocusFrameLifecycleStatus::Active, FocusFrameLifecycleAction::Close) => (
            FocusFrameLifecycleStatus::Closed,
            Some(updated_at.to_owned()),
        ),
        (FocusFrameLifecycleStatus::Closed, FocusFrameLifecycleAction::Reopen) => {
            (FocusFrameLifecycleStatus::Active, None)
        }
        (status, action) => {
            return Err(FocusFrameLifecycleError::InvalidTransition {
                focus_frame_id: snapshot.frame.id.clone(),
                status,
                action,
            });
        }
    };

    let revision = snapshot.revision.checked_add(1).ok_or_else(|| {
        FocusFrameLifecycleError::RevisionOverflow {
            focus_frame_id: snapshot.frame.id.clone(),
        }
    })?;
    Ok(FocusFrameLifecycleSnapshot {
        contract_version: FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
        frame: snapshot.frame.clone(),
        status,
        revision,
        updated_at: updated_at.into(),
        closed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::{
        FOCUS_CONTRACT_VERSION, FocusBranchKind, FocusContextPolicy, FocusMemoryScope,
    };

    fn snapshot(status: FocusFrameLifecycleStatus) -> FocusFrameLifecycleSnapshot {
        FocusFrameLifecycleSnapshot {
            contract_version: FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
            frame: FocusFrame {
                contract_version: FOCUS_CONTRACT_VERSION.into(),
                id: "focus-1".into(),
                conversation_id: "conversation-1".into(),
                parent_node_id: Some("node-1".into()),
                objective: "test focus".into(),
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
            },
            status,
            revision: 1,
            updated_at: "2026-08-25T00:00:00Z".into(),
            closed_at: None,
        }
    }

    #[test]
    fn close_then_reopen_preserves_frame_identity_and_increments_revision() {
        let active = snapshot(FocusFrameLifecycleStatus::Active);
        let closed = close_focus_frame(&active, "2026-08-25T01:00:00Z").expect("close focus");
        assert_eq!(closed.status, FocusFrameLifecycleStatus::Closed);
        assert_eq!(closed.revision, 2);
        assert_eq!(closed.closed_at.as_deref(), Some("2026-08-25T01:00:00Z"));
        assert_eq!(closed.frame.id, active.frame.id);

        let reopened = reopen_focus_frame(&closed, "2026-08-25T02:00:00Z").expect("reopen focus");
        assert_eq!(reopened.status, FocusFrameLifecycleStatus::Active);
        assert_eq!(reopened.revision, 3);
        assert_eq!(reopened.closed_at, None);
    }

    #[test]
    fn active_frame_cannot_be_reopened_and_closed_frame_cannot_be_closed_twice() {
        let active = snapshot(FocusFrameLifecycleStatus::Active);
        assert!(matches!(
            reopen_focus_frame(&active, "2026-08-25T01:00:00Z"),
            Err(FocusFrameLifecycleError::InvalidTransition {
                status: FocusFrameLifecycleStatus::Active,
                action: FocusFrameLifecycleAction::Reopen,
                ..
            })
        ));
        let closed = close_focus_frame(&active, "2026-08-25T01:00:00Z").expect("close focus");
        assert!(matches!(
            close_focus_frame(&closed, "2026-08-25T02:00:00Z"),
            Err(FocusFrameLifecycleError::InvalidTransition {
                status: FocusFrameLifecycleStatus::Closed,
                action: FocusFrameLifecycleAction::Close,
                ..
            })
        ));
    }

    #[test]
    fn lifecycle_rejects_empty_time_and_revision_overflow() {
        let active = snapshot(FocusFrameLifecycleStatus::Active);
        assert_eq!(
            close_focus_frame(&active, " ").expect_err("empty time"),
            FocusFrameLifecycleError::EmptyUpdatedAt
        );
        let overflow = FocusFrameLifecycleSnapshot {
            revision: u64::MAX,
            ..active
        };
        assert!(matches!(
            close_focus_frame(&overflow, "2026-08-25T01:00:00Z"),
            Err(FocusFrameLifecycleError::RevisionOverflow { .. })
        ));
    }

    #[test]
    fn lifecycle_validation_rejects_impossible_restored_states() {
        let mut active = snapshot(FocusFrameLifecycleStatus::Active);
        active.validate().expect("valid active lifecycle");

        active.closed_at = Some("2026-08-25T01:00:00Z".into());
        let error = active
            .validate()
            .expect_err("active lifecycle must not carry close metadata");
        assert!(error.to_string().contains("cannot have closedAt"));

        let mut closed = snapshot(FocusFrameLifecycleStatus::Closed);
        let error = closed
            .validate()
            .expect_err("closed lifecycle requires close metadata");
        assert!(error.to_string().contains("requires a non-empty closedAt"));
        closed.closed_at = Some("2026-08-25T01:00:00Z".into());
        closed.validate().expect("valid closed lifecycle");
    }

    #[test]
    fn lifecycle_validation_rejects_zero_revision_and_unknown_contract() {
        let mut invalid = snapshot(FocusFrameLifecycleStatus::Active);
        invalid.revision = 0;
        let error = invalid.validate().expect_err("zero revision");
        assert!(
            error
                .to_string()
                .contains("revision must be greater than zero")
        );

        invalid.revision = 1;
        invalid.contract_version = "mindscape.focus-lifecycle.v0".into();
        let error = invalid.validate().expect_err("unknown lifecycle contract");
        assert!(
            error
                .to_string()
                .contains("unsupported FocusFrame lifecycle")
        );
    }
}

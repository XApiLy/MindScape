use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::{KernelError, KernelResult};

pub const FOCUS_CONTRACT_VERSION: &str = "mindscape.focus.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FocusContextPolicy {
    ContinueCurrent,
    FocusNew,
    BranchFromNode,
    ContinueImportedRaw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FocusBranchKind {
    Mainline,
    Exploration,
    Task,
    Retrospective,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusMemoryScope {
    pub branch_kind: FocusBranchKind,
    pub inherit_refs: Vec<String>,
    pub local_refs: Vec<String>,
    pub exclude_refs: Vec<String>,
    pub promote_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusFrame {
    pub contract_version: String,
    pub id: String,
    pub conversation_id: String,
    pub parent_node_id: Option<String>,
    pub objective: String,
    pub active_work_item: Option<String>,
    pub context_policy: FocusContextPolicy,
    pub memory_scope: FocusMemoryScope,
    pub include_refs: Vec<String>,
    pub exclude_refs: Vec<String>,
    pub memory_version: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FocusPromotionCandidateSet {
    pub contract_version: String,
    pub focus_frame_id: String,
    pub conversation_id: String,
    pub branch_kind: FocusBranchKind,
    pub memory_version: u64,
    pub candidate_refs: Vec<String>,
}

impl FocusFrame {
    pub fn validate(&self) -> KernelResult<()> {
        if self.contract_version != FOCUS_CONTRACT_VERSION {
            return Err(KernelError::Validation(format!(
                "unsupported FocusFrame contract version: {}",
                self.contract_version
            )));
        }
        for (field, value) in [
            ("FocusFrame id", self.id.as_str()),
            ("FocusFrame conversation id", self.conversation_id.as_str()),
            ("FocusFrame objective", self.objective.as_str()),
            ("FocusFrame created at", self.created_at.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(KernelError::Validation(format!(
                    "{field} must not be empty"
                )));
            }
        }
        if self.parent_node_id.as_deref().is_some_and(str::is_empty) {
            return Err(KernelError::Validation(
                "FocusFrame parent node id must not be empty".into(),
            ));
        }
        if self.memory_version == 0 {
            return Err(KernelError::Validation(
                "FocusFrame memory version must be greater than zero".into(),
            ));
        }
        if self.memory_scope.branch_kind == FocusBranchKind::Mainline
            && !self.memory_scope.promote_refs.is_empty()
        {
            return Err(KernelError::Validation(
                "mainline FocusFrame cannot declare promotion candidates".into(),
            ));
        }

        let groups = [
            &self.memory_scope.inherit_refs,
            &self.memory_scope.local_refs,
            &self.memory_scope.exclude_refs,
            &self.memory_scope.promote_refs,
        ];
        let mut unique = HashSet::new();
        for reference in groups
            .into_iter()
            .flatten()
            .chain(&self.include_refs)
            .chain(&self.exclude_refs)
        {
            if reference.trim().is_empty() {
                return Err(KernelError::Validation(
                    "FocusFrame memory references must not be empty".into(),
                ));
            }
            if reference.trim() != reference {
                return Err(KernelError::Validation(
                    "FocusFrame memory references must not contain surrounding whitespace".into(),
                ));
            }
            if !unique.insert(reference) {
                return Err(KernelError::Validation(format!(
                    "FocusFrame memory reference appears in more than one set: {reference}"
                )));
            }
        }
        Ok(())
    }

    pub fn promotion_candidates(&self) -> KernelResult<FocusPromotionCandidateSet> {
        self.validate()?;
        if self.memory_scope.branch_kind == FocusBranchKind::Mainline {
            return Err(KernelError::Validation(
                "mainline FocusFrame cannot produce promotion candidates".into(),
            ));
        }
        if self.memory_scope.promote_refs.is_empty() {
            return Err(KernelError::Validation(
                "branch FocusFrame has no promotion candidates".into(),
            ));
        }
        Ok(FocusPromotionCandidateSet {
            contract_version: FOCUS_CONTRACT_VERSION.into(),
            focus_frame_id: self.id.clone(),
            conversation_id: self.conversation_id.clone(),
            branch_kind: self.memory_scope.branch_kind,
            memory_version: self.memory_version,
            candidate_refs: self.memory_scope.promote_refs.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmittedFocusRef {
    pub reference_id: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(branch_kind: FocusBranchKind, promote_refs: Vec<String>) -> FocusFrame {
        FocusFrame {
            contract_version: FOCUS_CONTRACT_VERSION.into(),
            id: "focus-task-1".into(),
            conversation_id: "conversation-1".into(),
            parent_node_id: Some("node-1".into()),
            objective: "Return the verified branch result".into(),
            active_work_item: Some("promotion review".into()),
            context_policy: FocusContextPolicy::BranchFromNode,
            memory_scope: FocusMemoryScope {
                branch_kind,
                inherit_refs: vec![],
                local_refs: vec![],
                exclude_refs: vec![],
                promote_refs,
            },
            include_refs: vec![],
            exclude_refs: vec![],
            memory_version: 3,
            created_at: "2026-08-28T05:10:00Z".into(),
        }
    }

    #[test]
    fn branch_builds_immutable_promotion_candidate_set() {
        let candidates = frame(FocusBranchKind::Task, vec!["entity-result-1".into()])
            .promotion_candidates()
            .expect("promotion candidates");

        assert_eq!(candidates.contract_version, FOCUS_CONTRACT_VERSION);
        assert_eq!(candidates.focus_frame_id, "focus-task-1");
        assert_eq!(candidates.conversation_id, "conversation-1");
        assert_eq!(candidates.branch_kind, FocusBranchKind::Task);
        assert_eq!(candidates.memory_version, 3);
        assert_eq!(candidates.candidate_refs, ["entity-result-1"]);
    }

    #[test]
    fn mainline_cannot_declare_promotion_candidates() {
        let error = frame(FocusBranchKind::Mainline, vec!["entity-result-1".into()])
            .validate()
            .expect_err("mainline promotion must be rejected");

        assert!(error.to_string().contains("mainline FocusFrame"));
    }

    #[test]
    fn branch_without_candidates_cannot_build_candidate_set() {
        let error = frame(FocusBranchKind::Exploration, vec![])
            .promotion_candidates()
            .expect_err("empty promotion set must be rejected");

        assert!(error.to_string().contains("no promotion candidates"));
    }

    #[test]
    fn memory_reference_whitespace_cannot_bypass_set_uniqueness() {
        let mut frame = frame(FocusBranchKind::Task, vec!["entity-result-1".into()]);
        frame.memory_scope.local_refs = vec![" entity-result-1 ".into()];

        let error = frame.validate().expect_err("surrounding whitespace");

        assert!(error.to_string().contains("surrounding whitespace"));
    }
}

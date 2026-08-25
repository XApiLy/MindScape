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
            if !unique.insert(reference) {
                return Err(KernelError::Validation(format!(
                    "FocusFrame memory reference appears in more than one set: {reference}"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmittedFocusRef {
    pub reference_id: String,
    pub reason: String,
}

use serde::{Deserialize, Serialize};

use super::{
    BranchType, ContentBlock, MessageRole, blocks_plain_text, contracts::EvidenceRef, new_id,
    now_timestamp,
};

pub const SYSTEM_CONTRACT_VERSION: &str = "mindscape.context.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextMessageRef {
    pub message_id: String,
    pub role: MessageRole,
    pub content_blocks: Vec<ContentBlock>,
    pub source_node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmittedContextRef {
    pub message_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextConstraint {
    pub text: String,
    pub evidence: Vec<EvidenceRef>,
    pub user_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshot {
    pub id: String,
    pub conversation_id: String,
    pub parent_node_id: Option<String>,
    pub branch_type: BranchType,
    pub current_input: String,
    pub selected_messages: Vec<ContextMessageRef>,
    pub selected_import_refs: Vec<EvidenceRef>,
    pub explicit_constraints: Vec<ContextConstraint>,
    pub omitted_messages: Vec<OmittedContextRef>,
    pub system_contract_version: String,
    pub estimated_tokens: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextTurn {
    pub node_id: String,
    pub user_message_id: String,
    pub user_content_blocks: Vec<ContentBlock>,
    pub assistant_message_id: Option<String>,
    pub assistant_content_blocks: Option<Vec<ContentBlock>>,
}

#[derive(Debug, Clone)]
pub struct ContextCompileInput {
    pub conversation_id: String,
    pub parent_node_id: Option<String>,
    pub branch_type: BranchType,
    pub current_input: String,
    pub path: Vec<ContextTurn>,
}

pub fn compile_context(input: ContextCompileInput) -> ContextSnapshot {
    let mut selected_messages = Vec::new();
    let mut omitted_messages = Vec::new();
    let last_index = input.path.len().saturating_sub(1);

    for (index, turn) in input.path.iter().enumerate() {
        selected_messages.push(ContextMessageRef {
            message_id: turn.user_message_id.clone(),
            role: MessageRole::User,
            content_blocks: turn.user_content_blocks.clone(),
            source_node_id: turn.node_id.clone(),
        });

        if let (Some(message_id), Some(content_blocks)) =
            (&turn.assistant_message_id, &turn.assistant_content_blocks)
        {
            let excludes_current_conclusion = matches!(
                input.branch_type,
                BranchType::Diverges | BranchType::Reframes
            ) && index == last_index;

            if excludes_current_conclusion {
                omitted_messages.push(OmittedContextRef {
                    message_id: message_id.clone(),
                    reason: match input.branch_type {
                        BranchType::Diverges => {
                            "diverging branches share background without inheriting the current conclusion"
                        }
                        BranchType::Reframes => {
                            "reframed branches exclude the current answer as an accepted premise"
                        }
                        _ => unreachable!(),
                    }
                    .to_string(),
                });
            } else {
                selected_messages.push(ContextMessageRef {
                    message_id: message_id.clone(),
                    role: MessageRole::Assistant,
                    content_blocks: content_blocks.clone(),
                    source_node_id: turn.node_id.clone(),
                });
            }
        }
    }

    let estimated_characters: usize = selected_messages
        .iter()
        .map(|message| blocks_plain_text(&message.content_blocks).chars().count())
        .sum::<usize>()
        + input.current_input.chars().count();

    ContextSnapshot {
        id: new_id("ctx"),
        conversation_id: input.conversation_id,
        parent_node_id: input.parent_node_id,
        branch_type: input.branch_type,
        current_input: input.current_input,
        selected_messages,
        selected_import_refs: Vec::new(),
        explicit_constraints: Vec::new(),
        omitted_messages,
        system_contract_version: SYSTEM_CONTRACT_VERSION.to_string(),
        estimated_tokens: estimated_characters.div_ceil(4) as i64,
        created_at: now_timestamp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn() -> ContextTurn {
        ContextTurn {
            node_id: "node-1".into(),
            user_message_id: "message-user".into(),
            user_content_blocks: vec![ContentBlock::text("question")],
            assistant_message_id: Some("message-assistant".into()),
            assistant_content_blocks: Some(vec![ContentBlock::text("conclusion")]),
        }
    }

    #[test]
    fn deepening_keeps_the_current_answer() {
        let snapshot = compile_context(ContextCompileInput {
            conversation_id: "conversation-1".into(),
            parent_node_id: Some("node-1".into()),
            branch_type: BranchType::Deepens,
            current_input: "continue".into(),
            path: vec![turn()],
        });

        assert_eq!(snapshot.selected_messages.len(), 2);
        assert!(snapshot.omitted_messages.is_empty());
    }

    #[test]
    fn reframing_excludes_the_current_answer() {
        let snapshot = compile_context(ContextCompileInput {
            conversation_id: "conversation-1".into(),
            parent_node_id: Some("node-1".into()),
            branch_type: BranchType::Reframes,
            current_input: "try another premise".into(),
            path: vec![turn()],
        });

        assert_eq!(snapshot.selected_messages.len(), 1);
        assert_eq!(snapshot.omitted_messages[0].message_id, "message-assistant");
    }
}

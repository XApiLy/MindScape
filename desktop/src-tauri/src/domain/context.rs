use serde::{Deserialize, Serialize};

use super::{
    BranchType, ContentBlock, KernelError, KernelResult, MessageRole, blocks_plain_text,
    contracts::EvidenceRef, new_id, now_timestamp,
};

pub const SYSTEM_CONTRACT_VERSION: &str = "mindscape.context.v1";
const CHAT_MESSAGE_OVERHEAD_TOKENS: usize = 4;

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
    pub max_context_tokens: Option<i64>,
}

pub fn compile_context(input: ContextCompileInput) -> KernelResult<ContextSnapshot> {
    if matches!(input.max_context_tokens, Some(limit) if limit <= 0) {
        return Err(KernelError::Validation(
            "context token budget must be greater than zero".into(),
        ));
    }

    let current_input_tokens = estimate_single_message_tokens(&input.current_input);
    if matches!(input.max_context_tokens, Some(limit) if current_input_tokens > limit) {
        return Err(KernelError::Validation(format!(
            "current input requires an estimated {current_input_tokens} tokens, exceeding the context budget"
        )));
    }

    let mut selected_turns = Vec::new();
    let mut omitted_messages = Vec::new();
    let last_index = input.path.len().saturating_sub(1);

    for (index, turn) in input.path.iter().enumerate() {
        let mut selected_turn = vec![ContextMessageRef {
            message_id: turn.user_message_id.clone(),
            role: MessageRole::User,
            content_blocks: turn.user_content_blocks.clone(),
            source_node_id: turn.node_id.clone(),
        }];

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
                selected_turn.push(ContextMessageRef {
                    message_id: message_id.clone(),
                    role: MessageRole::Assistant,
                    content_blocks: content_blocks.clone(),
                    source_node_id: turn.node_id.clone(),
                });
            }
        }
        selected_turns.push(selected_turn);
    }

    if let Some(limit) = input.max_context_tokens {
        while estimate_context_tokens(&selected_turns, &input.current_input) > limit
            && !selected_turns.is_empty()
        {
            for message in selected_turns.remove(0) {
                omitted_messages.push(OmittedContextRef {
                    message_id: message.message_id,
                    reason: "older turn omitted to satisfy the context token budget".into(),
                });
            }
        }
    }

    let estimated_tokens = estimate_context_tokens(&selected_turns, &input.current_input);
    let selected_messages = selected_turns.into_iter().flatten().collect();

    Ok(ContextSnapshot {
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
        estimated_tokens,
        created_at: now_timestamp(),
    })
}

fn estimate_context_tokens(selected_turns: &[Vec<ContextMessageRef>], current_input: &str) -> i64 {
    let mut estimate = TokenEstimate::default();
    estimate.add_message(current_input);
    for message in selected_turns.iter().flatten() {
        estimate.add_message(&blocks_plain_text(&message.content_blocks));
    }
    estimate.tokens()
}

fn estimate_single_message_tokens(text: &str) -> i64 {
    let mut estimate = TokenEstimate::default();
    estimate.add_message(text);
    estimate.tokens()
}

#[derive(Debug, Default)]
struct TokenEstimate {
    ascii_characters: usize,
    non_ascii_utf8_bytes: usize,
    messages: usize,
}

impl TokenEstimate {
    fn add_message(&mut self, text: &str) {
        for character in text.chars() {
            if character.is_ascii() {
                self.ascii_characters = self.ascii_characters.saturating_add(1);
            } else {
                self.non_ascii_utf8_bytes = self
                    .non_ascii_utf8_bytes
                    .saturating_add(character.len_utf8());
            }
        }
        self.messages = self.messages.saturating_add(1);
    }

    fn tokens(&self) -> i64 {
        let ascii_tokens = self.ascii_characters.div_ceil(4);
        let message_overhead = self.messages.saturating_mul(CHAT_MESSAGE_OVERHEAD_TOKENS);
        let total = ascii_tokens
            .saturating_add(self.non_ascii_utf8_bytes)
            .saturating_add(message_overhead);
        i64::try_from(total).unwrap_or(i64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(id: &str, question: &str, answer: &str) -> ContextTurn {
        ContextTurn {
            node_id: format!("node-{id}"),
            user_message_id: format!("message-{id}-user"),
            user_content_blocks: vec![ContentBlock::text(question)],
            assistant_message_id: Some(format!("message-{id}-assistant")),
            assistant_content_blocks: Some(vec![ContentBlock::text(answer)]),
        }
    }

    fn compile(branch_type: BranchType, path: Vec<ContextTurn>) -> ContextSnapshot {
        compile_context(ContextCompileInput {
            conversation_id: "conversation-1".into(),
            parent_node_id: path.last().map(|turn| turn.node_id.clone()),
            branch_type,
            current_input: "next".into(),
            path,
            max_context_tokens: None,
        })
        .expect("compile context")
    }

    #[test]
    fn root_context_contains_only_the_current_input() {
        let snapshot = compile(BranchType::Continues, vec![]);

        assert_eq!(snapshot.parent_node_id, None);
        assert_eq!(snapshot.current_input, "next");
        assert!(snapshot.selected_messages.is_empty());
        assert!(snapshot.omitted_messages.is_empty());
    }

    #[test]
    fn continuing_keeps_the_complete_root_to_parent_path() {
        let snapshot = compile(
            BranchType::Continues,
            vec![
                turn("root", "root question", "root answer"),
                turn("parent", "parent question", "parent answer"),
            ],
        );

        assert_eq!(
            snapshot
                .selected_messages
                .iter()
                .map(|message| message.message_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "message-root-user",
                "message-root-assistant",
                "message-parent-user",
                "message-parent-assistant",
            ]
        );
        assert!(snapshot.omitted_messages.is_empty());
    }

    #[test]
    fn deepening_keeps_the_current_answer() {
        let snapshot = compile(
            BranchType::Deepens,
            vec![turn("parent", "question", "conclusion")],
        );

        assert_eq!(snapshot.selected_messages.len(), 2);
        assert!(snapshot.omitted_messages.is_empty());
    }

    #[test]
    fn diverging_keeps_background_but_excludes_only_the_current_answer() {
        let snapshot = compile(
            BranchType::Diverges,
            vec![
                turn("root", "root question", "root answer"),
                turn("parent", "parent question", "parent conclusion"),
            ],
        );

        assert_eq!(snapshot.selected_messages.len(), 3);
        assert_eq!(
            snapshot.omitted_messages[0].message_id,
            "message-parent-assistant"
        );
    }

    #[test]
    fn reframing_keeps_the_upstream_question_but_excludes_the_current_answer() {
        let snapshot = compile(
            BranchType::Reframes,
            vec![
                turn("root", "root question", "root answer"),
                turn("parent", "parent question", "parent conclusion"),
            ],
        );

        assert_eq!(snapshot.selected_messages.len(), 3);
        assert_eq!(
            snapshot.omitted_messages[0].message_id,
            "message-parent-assistant"
        );
    }

    #[test]
    fn budget_trims_the_oldest_complete_turn_and_records_each_omission() {
        let snapshot = compile_context(ContextCompileInput {
            conversation_id: "conversation-1".into(),
            parent_node_id: Some("node-parent".into()),
            branch_type: BranchType::Continues,
            current_input: "next".into(),
            path: vec![
                turn("root", "12345678", "abcdefgh"),
                turn("parent", "u2", "a2"),
            ],
            max_context_tokens: Some(14),
        })
        .expect("compile budgeted context");

        assert_eq!(snapshot.estimated_tokens, 14);
        assert_eq!(snapshot.selected_messages.len(), 2);
        assert_eq!(snapshot.omitted_messages.len(), 2);
        assert!(
            snapshot
                .omitted_messages
                .iter()
                .all(|message| message.reason.contains("context token budget"))
        );
    }

    #[test]
    fn input_larger_than_the_budget_fails_instead_of_silently_truncating_user_text() {
        let error = compile_context(ContextCompileInput {
            conversation_id: "conversation-1".into(),
            parent_node_id: None,
            branch_type: BranchType::Continues,
            current_input: "this input cannot fit".into(),
            path: vec![],
            max_context_tokens: Some(1),
        })
        .expect_err("reject oversized current input");

        assert!(error.to_string().contains("exceeding the context budget"));
    }

    #[test]
    fn multilingual_text_and_chat_envelopes_are_estimated_conservatively() {
        assert_eq!(estimate_single_message_tokens("abcdefghijklmnop"), 8);
        assert_eq!(estimate_single_message_tokens("上下文预算"), 19);
        assert_eq!(estimate_single_message_tokens("🧠"), 8);

        let error = compile_context(ContextCompileInput {
            conversation_id: "conversation-1".into(),
            parent_node_id: None,
            branch_type: BranchType::Continues,
            current_input: "你好世界".into(),
            path: vec![],
            max_context_tokens: Some(4),
        })
        .expect_err("a character-count heuristic must not under-estimate CJK input");
        assert!(error.to_string().contains("estimated 16 tokens"));
    }

    #[test]
    fn every_budget_keeps_a_complete_turn_suffix_within_the_limit() {
        let path = vec![
            turn("root", "1111", "aaaa"),
            turn("middle", "22222222", "bbbbbbbb"),
            turn("parent", "333333333333", "cccccccccccc"),
        ];

        for limit in 5..=41 {
            let snapshot = compile_context(ContextCompileInput {
                conversation_id: "conversation-1".into(),
                parent_node_id: Some("node-parent".into()),
                branch_type: BranchType::Continues,
                current_input: "next".into(),
                path: path.clone(),
                max_context_tokens: Some(limit),
            })
            .expect("compile context for every valid budget");

            assert!(snapshot.estimated_tokens <= limit);
            assert_eq!(snapshot.selected_messages.len() % 2, 0);
            for pair in snapshot.selected_messages.chunks_exact(2) {
                assert_eq!(pair[0].role, MessageRole::User);
                assert_eq!(pair[1].role, MessageRole::Assistant);
                assert_eq!(pair[0].source_node_id, pair[1].source_node_id);
            }

            let selected_node_ids = snapshot
                .selected_messages
                .chunks_exact(2)
                .map(|pair| pair[0].source_node_id.as_str())
                .collect::<Vec<_>>();
            let path_node_ids = path
                .iter()
                .map(|turn| turn.node_id.as_str())
                .collect::<Vec<_>>();
            assert!(path_node_ids.ends_with(&selected_node_ids));
        }
    }
}

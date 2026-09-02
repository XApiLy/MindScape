use thiserror::Error;

use crate::domain::{
    ImportKnowledgeSuggestionDraft, blocks_plain_text,
    contracts::{GeneratorKind, GeneratorRef, ImportedMessage, KnowledgeEntityKind},
};

const MAX_SUGGESTION_NAME_CHARS: usize = 240;

/// Produces untrusted, source-referencing drafts. Implementations cannot
/// author proposal identity, evidence metadata, scope, status, or entities.
pub trait ImportKnowledgeSuggestionProducer: Send + Sync {
    fn generator(&self) -> GeneratorRef;

    fn produce(
        &self,
        messages: &[ImportedMessage],
    ) -> Result<Vec<ImportKnowledgeSuggestionDraft>, ImportKnowledgeSuggestionError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicImportKnowledgeSuggestionProducer;

impl ImportKnowledgeSuggestionProducer for DeterministicImportKnowledgeSuggestionProducer {
    fn generator(&self) -> GeneratorRef {
        GeneratorRef {
            kind: GeneratorKind::DeterministicRule,
            generator_id: "mindscape-import-knowledge-rule".into(),
            generator_version: "v1".into(),
        }
    }

    fn produce(
        &self,
        messages: &[ImportedMessage],
    ) -> Result<Vec<ImportKnowledgeSuggestionDraft>, ImportKnowledgeSuggestionError> {
        messages
            .iter()
            .enumerate()
            .map(|(ordinal, message)| {
                let ordinal = u32::try_from(ordinal)
                    .map_err(|_| ImportKnowledgeSuggestionError::TooManyMessages)?;
                let text = normalize_text(&blocks_plain_text(&message.content_blocks));
                if text.is_empty() {
                    return Err(ImportKnowledgeSuggestionError::EmptyMessage {
                        message_id: message.id.clone(),
                    });
                }
                Ok(ImportKnowledgeSuggestionDraft {
                    ordinal,
                    kind: classify(&text),
                    name: truncate_chars(&text, MAX_SUGGESTION_NAME_CHARS),
                    aliases: Vec::new(),
                    evidence_message_ids: vec![message.id.clone()],
                })
            })
            .collect()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportKnowledgeSuggestionError {
    #[error("selected import message {message_id} has no usable text")]
    EmptyMessage { message_id: String },
    #[error("too many selected import messages")]
    TooManyMessages,
}

fn normalize_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn classify(value: &str) -> KnowledgeEntityKind {
    let lowercase = value.to_lowercase();
    if value.ends_with(['?', '？']) {
        KnowledgeEntityKind::Question
    } else if contains_any(&lowercase, &["决定", "决策", "decided", "decision"]) {
        KnowledgeEntityKind::Decision
    } else if contains_any(
        &lowercase,
        &[
            "必须",
            "不得",
            "不能",
            "约束",
            "must",
            "cannot",
            "constraint",
        ],
    ) {
        KnowledgeEntityKind::Constraint
    } else if contains_any(&lowercase, &["目标", "目的", "goal", "objective"]) {
        KnowledgeEntityKind::Goal
    } else {
        KnowledgeEntityKind::Topic
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(test)]
mod tests {
    use crate::domain::{ContentBlock, MessageRole};

    use super::*;

    fn message(id: &str, text: &str) -> ImportedMessage {
        ImportedMessage {
            id: id.into(),
            import_revision_id: "revision-1".into(),
            role: MessageRole::Imported,
            content_blocks: vec![ContentBlock::text(text)],
            occurred_at: None,
            source_locator: format!("$.messages[{id}]"),
            parent_imported_message_id: None,
            platform_extension: serde_json::Value::Null,
        }
    }

    #[test]
    fn deterministic_producer_only_returns_allowed_source_references() {
        let drafts = DeterministicImportKnowledgeSuggestionProducer
            .produce(&[
                message("message-1", "团队决定采用本地 SQLite。"),
                message("message-2", "离线模式必须保留来源。"),
            ])
            .expect("produce drafts");

        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].kind, KnowledgeEntityKind::Decision);
        assert_eq!(drafts[0].evidence_message_ids, ["message-1"]);
        assert_eq!(drafts[1].kind, KnowledgeEntityKind::Constraint);
        assert_eq!(drafts[1].evidence_message_ids, ["message-2"]);
        assert!(drafts.iter().all(|draft| draft.aliases.is_empty()));
    }

    #[test]
    fn deterministic_producer_rejects_empty_selected_content() {
        let error = DeterministicImportKnowledgeSuggestionProducer
            .produce(&[message("message-empty", "  \n ")])
            .expect_err("empty source must fail");

        assert_eq!(
            error,
            ImportKnowledgeSuggestionError::EmptyMessage {
                message_id: "message-empty".into()
            }
        );
    }
}

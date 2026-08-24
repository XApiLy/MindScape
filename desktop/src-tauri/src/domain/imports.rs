use std::collections::{HashMap, HashSet};

use thiserror::Error;

use super::contracts::{ImportRevision, ImportSource, ImportedMessage, ParseReport};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportBundleValidationError {
    #[error("import bundle field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("import revision does not belong to the import source")]
    RevisionSourceMismatch,
    #[error("parse report does not belong to the import revision")]
    ReportRevisionMismatch,
    #[error("imported message {message_id} does not belong to the import revision")]
    MessageRevisionMismatch { message_id: String },
    #[error("imported message {message_id} appears more than once")]
    DuplicateMessageId { message_id: String },
    #[error("import source locator {source_locator} appears more than once")]
    DuplicateSourceLocator { source_locator: String },
    #[error("imported message {message_id} refers to missing parent {parent_id}")]
    MissingParent {
        message_id: String,
        parent_id: String,
    },
    #[error("imported message {message_id} cannot be its own parent")]
    SelfParent { message_id: String },
    #[error("imported message parent graph contains a cycle at {message_id}")]
    ParentCycle { message_id: String },
    #[error("parse report message count {reported} does not match persisted messages {actual}")]
    MessageCountMismatch { reported: u64, actual: u64 },
}

pub fn validate_import_bundle(
    source: &ImportSource,
    revision: &ImportRevision,
    messages: &[ImportedMessage],
    report: &ParseReport,
) -> Result<(), ImportBundleValidationError> {
    require_non_empty("source.id", &source.id)?;
    require_non_empty("source.conversation_id", &source.conversation_id)?;
    require_non_empty("source.content_hash", &source.content_hash)?;
    require_non_empty("source.storage_ref", &source.storage_ref)?;
    require_non_empty("source.created_at", &source.created_at)?;
    require_non_empty("revision.id", &revision.id)?;
    require_non_empty("revision.adapter_id", &revision.adapter_id)?;
    require_non_empty("revision.adapter_version", &revision.adapter_version)?;
    require_non_empty("revision.created_at", &revision.created_at)?;
    require_non_empty("report.import_revision_id", &report.import_revision_id)?;

    if revision.import_source_id != source.id {
        return Err(ImportBundleValidationError::RevisionSourceMismatch);
    }
    if report.import_revision_id != revision.id {
        return Err(ImportBundleValidationError::ReportRevisionMismatch);
    }

    let actual_message_count = u64::try_from(messages.len()).map_err(|_| {
        ImportBundleValidationError::MessageCountMismatch {
            reported: report.message_count,
            actual: u64::MAX,
        }
    })?;
    if report.message_count != actual_message_count {
        return Err(ImportBundleValidationError::MessageCountMismatch {
            reported: report.message_count,
            actual: actual_message_count,
        });
    }

    let mut message_ids = HashSet::with_capacity(messages.len());
    let mut source_locators = HashSet::with_capacity(messages.len());
    let mut parents = HashMap::with_capacity(messages.len());
    for message in messages {
        require_non_empty("message.id", &message.id)?;
        require_non_empty("message.source_locator", &message.source_locator)?;
        if message.import_revision_id != revision.id {
            return Err(ImportBundleValidationError::MessageRevisionMismatch {
                message_id: message.id.clone(),
            });
        }
        if !message_ids.insert(message.id.clone()) {
            return Err(ImportBundleValidationError::DuplicateMessageId {
                message_id: message.id.clone(),
            });
        }
        if !source_locators.insert(message.source_locator.clone()) {
            return Err(ImportBundleValidationError::DuplicateSourceLocator {
                source_locator: message.source_locator.clone(),
            });
        }
        if let Some(parent_id) = &message.parent_imported_message_id {
            if parent_id == &message.id {
                return Err(ImportBundleValidationError::SelfParent {
                    message_id: message.id.clone(),
                });
            }
            parents.insert(message.id.clone(), Some(parent_id.clone()));
        } else {
            parents.insert(message.id.clone(), None);
        }
    }

    for (message_id, parent_id) in &parents {
        if let Some(parent_id) = parent_id
            && !message_ids.contains(parent_id)
        {
            return Err(ImportBundleValidationError::MissingParent {
                message_id: message_id.clone(),
                parent_id: parent_id.clone(),
            });
        }
    }

    for message_id in message_ids {
        let mut visited = HashSet::new();
        let mut current = Some(message_id.clone());
        while let Some(current_id) = current {
            if !visited.insert(current_id.clone()) {
                return Err(ImportBundleValidationError::ParentCycle {
                    message_id: current_id,
                });
            }
            current = parents.get(&current_id).cloned().flatten();
        }
    }

    Ok(())
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ImportBundleValidationError> {
    if value.trim().is_empty() {
        return Err(ImportBundleValidationError::EmptyField { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::contracts::{ImportPlatform, ImportRevisionStatus};
    use crate::domain::{ContentBlock, MessageRole};

    fn source() -> ImportSource {
        ImportSource {
            id: "source-1".into(),
            conversation_id: "conversation-1".into(),
            platform: ImportPlatform::Generic,
            original_file_name: Some("notes.md".into()),
            content_hash: "sha256:source".into(),
            storage_ref: "sha256/source".into(),
            created_at: "2026-08-24T00:00:00Z".into(),
        }
    }

    fn revision() -> ImportRevision {
        ImportRevision {
            id: "revision-1".into(),
            import_source_id: "source-1".into(),
            adapter_id: "generic-markdown".into(),
            adapter_version: "v1".into(),
            status: ImportRevisionStatus::Parsed,
            created_at: "2026-08-24T00:00:00Z".into(),
        }
    }

    fn message(id: &str, locator: &str, parent: Option<&str>) -> ImportedMessage {
        ImportedMessage {
            id: id.into(),
            import_revision_id: "revision-1".into(),
            role: MessageRole::Imported,
            content_blocks: vec![ContentBlock::text("raw")],
            occurred_at: None,
            source_locator: locator.into(),
            parent_imported_message_id: parent.map(str::to_owned),
            platform_extension: json!({}),
        }
    }

    fn report(message_count: u64) -> ParseReport {
        ParseReport {
            import_revision_id: "revision-1".into(),
            conversation_count: 1,
            message_count,
            attachment_count: 0,
            tool_record_count: 0,
            field_recovery: vec![],
            warnings: vec![],
            errors: vec![],
        }
    }

    #[test]
    fn accepts_a_valid_bundle_even_when_children_precede_parents() {
        let messages = vec![
            message("message-child", "$.messages[1]", Some("message-parent")),
            message("message-parent", "$.messages[0]", None),
        ];
        validate_import_bundle(&source(), &revision(), &messages, &report(2))
            .expect("valid import bundle");
    }

    #[test]
    fn rejects_duplicate_locators_and_missing_parents() {
        let duplicate_locator = vec![
            message("message-1", "$.messages[0]", None),
            message("message-2", "$.messages[0]", None),
        ];
        assert!(matches!(
            validate_import_bundle(&source(), &revision(), &duplicate_locator, &report(2)),
            Err(ImportBundleValidationError::DuplicateSourceLocator { .. })
        ));

        let missing_parent = vec![message("message-1", "$.messages[0]", Some("missing"))];
        assert!(matches!(
            validate_import_bundle(&source(), &revision(), &missing_parent, &report(1)),
            Err(ImportBundleValidationError::MissingParent { .. })
        ));
    }

    #[test]
    fn rejects_parent_cycles_and_count_mismatches() {
        let cycle = vec![
            message("message-1", "$.messages[0]", Some("message-2")),
            message("message-2", "$.messages[1]", Some("message-1")),
        ];
        assert!(matches!(
            validate_import_bundle(&source(), &revision(), &cycle, &report(2)),
            Err(ImportBundleValidationError::ParentCycle { .. })
        ));

        let messages = vec![message("message-1", "$.messages[0]", None)];
        assert!(matches!(
            validate_import_bundle(&source(), &revision(), &messages, &report(2)),
            Err(ImportBundleValidationError::MessageCountMismatch { .. })
        ));
    }
}

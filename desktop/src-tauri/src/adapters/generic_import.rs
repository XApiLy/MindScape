use serde_json::Value;
use thiserror::Error;

use crate::domain::contracts::{
    FieldRecovery, ImportIssue, ImportRevision, ImportRevisionStatus, ImportSource,
    ImportedMessage, ParseReport, RecoveryStatus,
};
use crate::domain::{ContentBlock, MessageRole, new_id, now_timestamp};

use super::ImportPayloadFormat;

#[derive(Debug, Error)]
pub enum GenericImportError {
    #[error("import payload is not valid UTF-8")]
    InvalidUtf8,
    #[error("JSONL line {line} is invalid JSON")]
    InvalidJsonLine { line: u64 },
}

#[derive(Debug, Clone)]
pub struct ParsedImportBundle {
    pub source: ImportSource,
    pub revision: ImportRevision,
    pub messages: Vec<ImportedMessage>,
    pub report: ParseReport,
}

pub fn parse_generic_import(
    source: ImportSource,
    revision_id: String,
    format: ImportPayloadFormat,
    payload: &[u8],
) -> Result<ParsedImportBundle, GenericImportError> {
    let text = std::str::from_utf8(payload).map_err(|_| GenericImportError::InvalidUtf8)?;
    let mut messages = Vec::new();
    let mut warnings = Vec::new();
    let mut recovery = vec![FieldRecovery {
        field: "rawText".into(),
        status: RecoveryStatus::Recovered,
        detail: None,
    }];

    match format {
        ImportPayloadFormat::Markdown | ImportPayloadFormat::Text => {
            messages.push(message(&revision_id, "document", "$.text", text.to_owned()));
        }
        ImportPayloadFormat::JsonLines => {
            for (index, line) in text.lines().enumerate() {
                let line_number = index as u64 + 1;
                if line.trim().is_empty() {
                    warnings.push(ImportIssue {
                        code: "empty_line".into(),
                        message: "Empty JSONL line was skipped.".into(),
                        source_locator: Some(format!("$[{index}]")),
                        recoverable: true,
                    });
                    continue;
                }
                let value: Value = serde_json::from_str(line)
                    .map_err(|_| GenericImportError::InvalidJsonLine { line: line_number })?;
                let content = value
                    .get("content")
                    .and_then(Value::as_str)
                    .or_else(|| value.get("text").and_then(Value::as_str))
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| value.to_string());
                messages.push(message(
                    &revision_id,
                    &format!("message-{line_number}"),
                    &format!("$[{index}]"),
                    content,
                ));
            }
            if messages.is_empty() {
                recovery.push(FieldRecovery {
                    field: "messages".into(),
                    status: RecoveryStatus::Partial,
                    detail: Some("No non-empty JSONL records were found.".into()),
                });
            }
        }
    }

    let report = ParseReport {
        import_revision_id: revision_id.clone(),
        conversation_count: if messages.is_empty() { 0 } else { 1 },
        message_count: messages.len() as u64,
        attachment_count: 0,
        tool_record_count: 0,
        field_recovery: recovery,
        warnings,
        errors: vec![],
    };
    let revision = ImportRevision {
        id: revision_id,
        import_source_id: source.id.clone(),
        adapter_id: "generic-import".into(),
        adapter_version: "1".into(),
        status: ImportRevisionStatus::Parsed,
        created_at: now_timestamp(),
    };
    Ok(ParsedImportBundle {
        source,
        revision,
        messages,
        report,
    })
}

fn message(revision_id: &str, label: &str, locator: &str, content: String) -> ImportedMessage {
    ImportedMessage {
        id: new_id(&format!("import-message-{label}")),
        import_revision_id: revision_id.into(),
        role: MessageRole::Imported,
        content_blocks: vec![ContentBlock::text(content)],
        occurred_at: None,
        source_locator: locator.into(),
        parent_imported_message_id: None,
        platform_extension: Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contracts::ImportPlatform;

    fn source() -> ImportSource {
        ImportSource {
            id: "source-parse".into(),
            conversation_id: "conversation-parse".into(),
            platform: ImportPlatform::Generic,
            original_file_name: Some("sample.md".into()),
            content_hash: "hash".into(),
            storage_ref: "aa/hash".into(),
            created_at: now_timestamp(),
        }
    }

    #[test]
    fn parses_markdown_as_one_lossless_imported_message() {
        let bundle = parse_generic_import(
            source(),
            "revision-md".into(),
            ImportPayloadFormat::Markdown,
            "# Title\n\nbody".as_bytes(),
        )
        .expect("parse markdown");
        assert_eq!(bundle.report.message_count, 1);
        assert_eq!(bundle.messages[0].source_locator, "$.text");
        assert_eq!(bundle.revision.status, ImportRevisionStatus::Parsed);
    }

    #[test]
    fn parses_jsonl_records_and_reports_empty_lines() {
        let bundle = parse_generic_import(
            source(),
            "revision-jsonl".into(),
            ImportPayloadFormat::JsonLines,
            b"{\"role\":\"user\",\"content\":\"hello\"}\n\n{\"text\":\"world\"}",
        )
        .expect("parse jsonl");
        assert_eq!(bundle.report.message_count, 2);
        assert_eq!(bundle.report.warnings.len(), 1);
    }

    #[test]
    fn rejects_invalid_jsonl_without_creating_a_partial_bundle() {
        let result = parse_generic_import(
            source(),
            "revision-invalid".into(),
            ImportPayloadFormat::JsonLines,
            b"{invalid}",
        );
        assert!(matches!(
            result,
            Err(GenericImportError::InvalidJsonLine { line: 1 })
        ));
    }
}

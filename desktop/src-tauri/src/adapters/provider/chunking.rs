use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::embedding::IndexInput;

pub const MARKDOWN_CHUNK_VERSION: &str = "mindscape-markdown-chunk-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChunkKind {
    Heading,
    Paragraph,
    CodeBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextChunk {
    pub id: String,
    pub source_id: String,
    pub source_hash: String,
    pub branch_id: Option<String>,
    pub ordinal: usize,
    pub kind: ChunkKind,
    pub heading_path: Vec<String>,
    pub text: String,
    pub chunk_hash: String,
    pub chunk_version: String,
}

impl TextChunk {
    pub fn as_index_input(&self) -> IndexInput {
        IndexInput {
            id: self.id.clone(),
            text: self.text.clone(),
            source_hash: self.source_hash.clone(),
            chunk_version: self.chunk_version.clone(),
        }
    }
}

pub fn chunk_markdown(
    source_id: impl Into<String>,
    source_hash: impl Into<String>,
    branch_id: Option<String>,
    markdown: &str,
) -> Vec<TextChunk> {
    let source_id = source_id.into();
    let source_hash = source_hash.into();
    let mut chunks = Vec::new();
    let mut heading_path = Vec::<String>::new();
    let mut paragraph = Vec::<String>::new();
    let mut code = Vec::<String>::new();
    let mut in_code = false;
    let mut code_fence = String::new();

    let flush = |kind: ChunkKind,
                 lines: &mut Vec<String>,
                 chunks: &mut Vec<TextChunk>,
                 heading_path: &[String],
                 source_id: &str,
                 source_hash: &str,
                 branch_id: &Option<String>| {
        let text = lines.join("\n").trim().to_string();
        lines.clear();
        if text.is_empty() {
            return;
        }
        let ordinal = chunks.len();
        let chunk_hash = hex_hash(text.as_bytes());
        chunks.push(TextChunk {
            id: format!("{source_id}:chunk:{ordinal}"),
            source_id: source_id.into(),
            source_hash: source_hash.into(),
            branch_id: branch_id.clone(),
            ordinal,
            kind,
            heading_path: heading_path.to_vec(),
            text,
            chunk_hash,
            chunk_version: MARKDOWN_CHUNK_VERSION.into(),
        });
    };

    for line in markdown.lines() {
        if in_code {
            code.push(line.into());
            if line.trim_start().starts_with(&code_fence) {
                in_code = false;
                flush(
                    ChunkKind::CodeBlock,
                    &mut code,
                    &mut chunks,
                    &heading_path,
                    &source_id,
                    &source_hash,
                    &branch_id,
                );
                code_fence.clear();
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(
                ChunkKind::Paragraph,
                &mut paragraph,
                &mut chunks,
                &heading_path,
                &source_id,
                &source_hash,
                &branch_id,
            );
            in_code = true;
            code_fence = trimmed[..3].into();
            code.push(line.into());
        } else if let Some(heading) = parse_heading(trimmed) {
            flush(
                ChunkKind::Paragraph,
                &mut paragraph,
                &mut chunks,
                &heading_path,
                &source_id,
                &source_hash,
                &branch_id,
            );
            let (level, title) = heading;
            heading_path.truncate(level.saturating_sub(1));
            heading_path.push(title.to_string());
            flush(
                ChunkKind::Heading,
                &mut vec![trimmed.into()],
                &mut chunks,
                &heading_path,
                &source_id,
                &source_hash,
                &branch_id,
            );
        } else if trimmed.is_empty() {
            flush(
                ChunkKind::Paragraph,
                &mut paragraph,
                &mut chunks,
                &heading_path,
                &source_id,
                &source_hash,
                &branch_id,
            );
        } else {
            paragraph.push(line.into());
        }
    }
    if in_code {
        flush(
            ChunkKind::CodeBlock,
            &mut code,
            &mut chunks,
            &heading_path,
            &source_id,
            &source_hash,
            &branch_id,
        );
    } else {
        flush(
            ChunkKind::Paragraph,
            &mut paragraph,
            &mut chunks,
            &heading_path,
            &source_id,
            &source_hash,
            &branch_id,
        );
    }
    chunks
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6)
        .contains(&level)
        .then(|| (level, line[level..].trim()))
        .filter(|(_, title)| !title.is_empty())
}

fn hex_hash(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_heading_path_paragraph_code_and_provenance() {
        let chunks = chunk_markdown(
            "source-1",
            "source-hash",
            Some("branch-1".into()),
            "# Project\n\nIntro paragraph.\n\n## Design\n\n```rust\nlet x = 1;\n```\n",
        );
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].kind, ChunkKind::Heading);
        assert_eq!(chunks[1].kind, ChunkKind::Paragraph);
        assert_eq!(chunks[2].heading_path, vec!["Project", "Design"]);
        assert_eq!(chunks[3].kind, ChunkKind::CodeBlock);
        assert_eq!(chunks[3].branch_id.as_deref(), Some("branch-1"));
        assert_eq!(chunks[3].source_hash, "source-hash");
        assert_eq!(chunks[3].chunk_version, MARKDOWN_CHUNK_VERSION);
    }

    #[test]
    fn chunk_ids_and_hashes_are_deterministic() {
        let first = chunk_markdown("source", "hash", None, "## Same\n\ntext");
        let second = chunk_markdown("source", "hash", None, "## Same\n\ntext");
        assert_eq!(first, second);
        assert_ne!(first[0].chunk_hash, first[1].chunk_hash);
    }

    #[test]
    fn unfinished_code_fence_is_preserved_as_code_data() {
        let chunks = chunk_markdown("source", "hash", None, "```\nnot finished");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, ChunkKind::CodeBlock);
        assert!(chunks[0].text.contains("not finished"));
    }

    #[test]
    fn chunk_projects_to_embedding_input_without_losing_stable_identity() {
        let chunks = chunk_markdown("source", "hash", Some("branch".into()), "# Heading\n\nbody");
        let input = chunks[1].as_index_input();
        assert_eq!(input.id, chunks[1].id);
        assert_eq!(input.text, "body");
        assert_eq!(input.source_hash, "hash");
        assert_eq!(input.chunk_version, MARKDOWN_CHUNK_VERSION);
    }
}

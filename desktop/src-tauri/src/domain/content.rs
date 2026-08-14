use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Code {
        language: Option<String>,
        code: String,
    },
    Link {
        url: String,
        label: Option<String>,
    },
    AttachmentRef {
        attachment_id: String,
        media_type: Option<String>,
        display_name: String,
    },
    ToolCallRef {
        tool_run_id: String,
    },
    ToolResultRef {
        tool_run_id: String,
    },
    Unsupported {
        original_type: String,
        raw_json: serde_json::Value,
    },
}

impl ContentBlock {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text { text: value.into() }
    }

    pub fn plain_text(&self) -> String {
        match self {
            Self::Text { text } => text.clone(),
            Self::Code { language, code } => match language {
                Some(language) => format!("```{language}\n{code}\n```"),
                None => format!("```\n{code}\n```"),
            },
            Self::Link { url, label } => label
                .as_ref()
                .map(|label| format!("{label} ({url})"))
                .unwrap_or_else(|| url.clone()),
            Self::AttachmentRef { display_name, .. } => {
                format!("[attachment: {display_name}]")
            }
            Self::ToolCallRef { tool_run_id } => format!("[tool call: {tool_run_id}]"),
            Self::ToolResultRef { tool_run_id } => format!("[tool result: {tool_run_id}]"),
            Self::Unsupported { original_type, .. } => {
                format!("[unsupported content: {original_type}]")
            }
        }
    }
}

pub fn blocks_plain_text(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .map(ContentBlock::plain_text)
        .collect::<Vec<_>>()
        .join("\n")
}

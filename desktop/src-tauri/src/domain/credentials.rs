use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type CredentialResult<T> = Result<T, CredentialError>;

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRef {
    pub provider_id: String,
    pub account_id: String,
}

impl CredentialRef {
    pub fn validate(&self) -> CredentialResult<()> {
        validate_segment("provider ID", &self.provider_id)?;
        validate_segment("account ID", &self.account_id)
    }
}

fn validate_segment(label: &str, value: &str) -> CredentialResult<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(CredentialError::InvalidReference(format!(
            "{label} must contain only letters, numbers, '-' or '_'"
        )));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("invalid credential reference: {0}")]
    InvalidReference(String),
    #[error("credential not found")]
    NotFound,
    #[error("operating system credential store is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetCredentialInput {
    #[serde(flatten)]
    pub reference: CredentialRef,
    pub secret: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_reference_rejects_path_and_service_injection() {
        for value in ["../openai", "openai/key", "openai key", ""] {
            let reference = CredentialRef {
                provider_id: value.into(),
                account_id: "default".into(),
            };
            assert!(reference.validate().is_err());
        }
    }
}

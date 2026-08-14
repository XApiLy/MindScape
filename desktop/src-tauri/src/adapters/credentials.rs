use std::sync::Arc;

use keyring::v1::{Entry, Error as KeyringError};
use zeroize::Zeroizing;

use crate::domain::{CredentialError, CredentialRef, CredentialResult};

const SERVICE_PREFIX: &str = "com.mindscape.desktop.provider";

pub trait CredentialStore: Send + Sync + std::fmt::Debug {
    fn set(&self, reference: &CredentialRef, secret: &str) -> CredentialResult<()>;
    fn get(&self, reference: &CredentialRef) -> CredentialResult<Zeroizing<String>>;
    fn delete(&self, reference: &CredentialRef) -> CredentialResult<()>;
}

#[derive(Debug, Default)]
pub struct OsCredentialStore;

impl OsCredentialStore {
    fn entry(reference: &CredentialRef) -> CredentialResult<Entry> {
        reference.validate()?;
        Entry::new(
            &format!("{SERVICE_PREFIX}.{}", reference.provider_id),
            &reference.account_id,
        )
        .map_err(map_keyring_error)
    }
}

impl CredentialStore for OsCredentialStore {
    fn set(&self, reference: &CredentialRef, secret: &str) -> CredentialResult<()> {
        if secret.trim().is_empty() {
            return Err(CredentialError::InvalidReference(
                "credential secret cannot be empty".into(),
            ));
        }
        Self::entry(reference)?
            .set_password(secret)
            .map_err(map_keyring_error)
    }

    fn get(&self, reference: &CredentialRef) -> CredentialResult<Zeroizing<String>> {
        Self::entry(reference)?
            .get_password()
            .map(Zeroizing::new)
            .map_err(map_keyring_error)
    }

    fn delete(&self, reference: &CredentialRef) -> CredentialResult<()> {
        match Self::entry(reference)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(map_keyring_error(error)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CredentialService {
    store: Arc<dyn CredentialStore>,
}

impl CredentialService {
    pub fn os_default() -> Self {
        Self {
            store: Arc::new(OsCredentialStore),
        }
    }

    #[cfg(test)]
    pub fn new(store: Arc<dyn CredentialStore>) -> Self {
        Self { store }
    }

    pub fn set(&self, reference: &CredentialRef, secret: &str) -> CredentialResult<()> {
        self.store.set(reference, secret)
    }

    pub fn exists(&self, reference: &CredentialRef) -> CredentialResult<bool> {
        match self.store.get(reference) {
            Ok(_) => Ok(true),
            Err(CredentialError::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn resolve(&self, reference: &CredentialRef) -> CredentialResult<Zeroizing<String>> {
        self.store.get(reference)
    }

    pub fn delete(&self, reference: &CredentialRef) -> CredentialResult<()> {
        self.store.delete(reference)
    }
}

fn map_keyring_error(error: KeyringError) -> CredentialError {
    match error {
        KeyringError::NoEntry => CredentialError::NotFound,
        _ => CredentialError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use super::*;

    #[derive(Debug, Default)]
    struct MemoryCredentialStore {
        secrets: Mutex<HashMap<CredentialRef, String>>,
    }

    impl CredentialStore for MemoryCredentialStore {
        fn set(&self, reference: &CredentialRef, secret: &str) -> CredentialResult<()> {
            self.secrets
                .lock()
                .expect("lock credential store")
                .insert(reference.clone(), secret.into());
            Ok(())
        }

        fn get(&self, reference: &CredentialRef) -> CredentialResult<Zeroizing<String>> {
            self.secrets
                .lock()
                .expect("lock credential store")
                .get(reference)
                .cloned()
                .map(Zeroizing::new)
                .ok_or(CredentialError::NotFound)
        }

        fn delete(&self, reference: &CredentialRef) -> CredentialResult<()> {
            self.secrets
                .lock()
                .expect("lock credential store")
                .remove(reference);
            Ok(())
        }
    }

    fn reference() -> CredentialRef {
        CredentialRef {
            provider_id: "openai".into(),
            account_id: "default".into(),
        }
    }

    #[test]
    fn frontend_safe_lifecycle_never_requires_returning_the_secret() {
        let service = CredentialService::new(Arc::new(MemoryCredentialStore::default()));
        let reference = reference();

        assert!(!service.exists(&reference).unwrap());
        service.set(&reference, "top-secret").unwrap();
        assert!(service.exists(&reference).unwrap());
        assert_eq!(service.resolve(&reference).unwrap().as_str(), "top-secret");
        service.delete(&reference).unwrap();
        assert!(!service.exists(&reference).unwrap());
    }
}

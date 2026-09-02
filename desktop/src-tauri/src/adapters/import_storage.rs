use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

const DEFAULT_MAX_IMPORT_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportPayloadFormat {
    Markdown,
    JsonLines,
    Text,
}

impl ImportPayloadFormat {
    fn from_file_name(file_name: &Path) -> Result<Self, ImportStorageError> {
        let extension = file_name
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase);

        match extension.as_deref() {
            Some("md" | "markdown") => Ok(Self::Markdown),
            Some("jsonl") => Ok(Self::JsonLines),
            Some("txt") => Ok(Self::Text),
            _ => Err(ImportStorageError::UnsupportedFormat),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredImportPayload {
    pub content_hash: String,
    pub storage_ref: String,
    pub format: ImportPayloadFormat,
    pub byte_length: u64,
    pub duplicate: bool,
}

#[derive(Debug, Error)]
pub enum ImportStorageError {
    #[error("only Markdown, JSONL, and TXT imports are supported")]
    UnsupportedFormat,
    #[error("import payload is empty")]
    EmptyPayload,
    #[error("import payload exceeds the {max_bytes} byte limit")]
    PayloadTooLarge { max_bytes: u64 },
    #[error("stored import payload does not match its content hash")]
    HashCollision,
    #[error("stored import reference is invalid")]
    InvalidStorageRef,
    #[error("stored import payload is not valid UTF-8 text")]
    InvalidUtf8,
    #[error("failed to access import storage: {0}")]
    Io(#[from] io::Error),
}

/// Immutable, content-addressed storage for raw imported payloads.
#[derive(Debug, Clone)]
pub struct ImportStorage {
    root: PathBuf,
    max_bytes: u64,
}

impl ImportStorage {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self::with_max_bytes(root, DEFAULT_MAX_IMPORT_BYTES)
    }

    pub fn with_max_bytes(root: impl AsRef<Path>, max_bytes: u64) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            max_bytes,
        }
    }

    pub fn store(
        &self,
        original_file_name: impl AsRef<Path>,
        payload: &[u8],
    ) -> Result<StoredImportPayload, ImportStorageError> {
        if payload.is_empty() {
            return Err(ImportStorageError::EmptyPayload);
        }
        if payload.len() as u64 > self.max_bytes {
            return Err(ImportStorageError::PayloadTooLarge {
                max_bytes: self.max_bytes,
            });
        }

        let format = ImportPayloadFormat::from_file_name(original_file_name.as_ref())?;
        let content_hash = format!("{:x}", Sha256::digest(payload));
        let relative_path = PathBuf::from(&content_hash[0..2]).join(&content_hash);
        let destination = self.root.join(&relative_path);

        if destination.exists() {
            return self.existing_result(destination, relative_path, content_hash, format, payload);
        }

        let parent = self.root.join(&content_hash[0..2]);
        fs::create_dir_all(&parent)?;
        let temporary = parent.join(format!(".{}.tmp", Uuid::new_v4()));
        let write_result = (|| -> io::Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(payload)?;
            file.sync_all()?;
            fs::rename(&temporary, &destination)
        })();

        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary);
            if destination.exists() {
                return self.existing_result(
                    destination,
                    relative_path,
                    content_hash,
                    format,
                    payload,
                );
            }
            return Err(error.into());
        }

        Ok(Self::result(
            relative_path,
            content_hash,
            format,
            payload,
            false,
        ))
    }

    pub fn discard_if_new(&self, stored: &StoredImportPayload) -> io::Result<()> {
        if !stored.duplicate {
            let path = self.root.join(&stored.storage_ref);
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    pub fn read_verified_text(
        &self,
        storage_ref: &str,
        expected_hash: &str,
        preview_bytes: usize,
    ) -> Result<(String, u64, bool), ImportStorageError> {
        if !valid_storage_ref(storage_ref, expected_hash) || preview_bytes == 0 {
            return Err(ImportStorageError::InvalidStorageRef);
        }
        let path = self.root.join(storage_ref);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(ImportStorageError::InvalidStorageRef);
        }
        if metadata.len() > self.max_bytes {
            return Err(ImportStorageError::PayloadTooLarge {
                max_bytes: self.max_bytes,
            });
        }
        let payload = fs::read(path)?;
        if format!("{:x}", Sha256::digest(&payload)) != expected_hash {
            return Err(ImportStorageError::HashCollision);
        }
        let text = std::str::from_utf8(&payload).map_err(|_| ImportStorageError::InvalidUtf8)?;
        let truncated = payload.len() > preview_bytes;
        let mut end = preview_bytes.min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Ok((text[..end].into(), payload.len() as u64, truncated))
    }

    pub fn recover_interrupted_writes(&self) -> io::Result<u64> {
        if !self.root.exists() {
            return Ok(0);
        }
        let mut removed = 0;
        for prefix in fs::read_dir(&self.root)? {
            let prefix = prefix?;
            if !prefix.file_type()?.is_dir() {
                continue;
            }
            for entry in fs::read_dir(prefix.path())? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if entry.file_type()?.is_file() && name.starts_with('.') && name.ends_with(".tmp") {
                    fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    pub fn reconcile_unreferenced(&self, referenced: &HashSet<String>) -> io::Result<u64> {
        if !self.root.exists() {
            return Ok(0);
        }
        let mut removed = 0;
        for prefix in fs::read_dir(&self.root)? {
            let prefix = prefix?;
            if !prefix.file_type()?.is_dir() {
                continue;
            }
            for entry in fs::read_dir(prefix.path())? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let reference = format!("{}/{}", prefix.file_name().to_string_lossy(), name);
                if entry.file_type()?.is_file()
                    && name.len() == 64
                    && name.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && !referenced.contains(&reference)
                {
                    fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    fn existing_result(
        &self,
        destination: PathBuf,
        relative_path: PathBuf,
        content_hash: String,
        format: ImportPayloadFormat,
        payload: &[u8],
    ) -> Result<StoredImportPayload, ImportStorageError> {
        if fs::read(destination)? != payload {
            return Err(ImportStorageError::HashCollision);
        }
        Ok(Self::result(
            relative_path,
            content_hash,
            format,
            payload,
            true,
        ))
    }

    fn result(
        relative_path: PathBuf,
        content_hash: String,
        format: ImportPayloadFormat,
        payload: &[u8],
        duplicate: bool,
    ) -> StoredImportPayload {
        StoredImportPayload {
            content_hash,
            storage_ref: relative_path.to_string_lossy().replace('\\', "/"),
            format,
            byte_length: payload.len() as u64,
            duplicate,
        }
    }
}

fn valid_storage_ref(storage_ref: &str, expected_hash: &str) -> bool {
    expected_hash.len() == 64
        && expected_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && storage_ref == format!("{}/{}", &expected_hash[..2], expected_hash)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn stores_payload_by_sha256_without_using_untrusted_file_name() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());

        let stored = storage
            .store("../../private/conversation.md", b"# Safe import")
            .expect("store payload");

        assert_eq!(stored.content_hash.len(), 64);
        assert!(!stored.storage_ref.contains("private"));
        assert!(!stored.storage_ref.contains(".."));
        assert_eq!(
            fs::read(directory.path().join(&stored.storage_ref)).expect("read stored payload"),
            b"# Safe import"
        );
    }

    #[test]
    fn reports_identical_content_as_duplicate_without_overwriting_it() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());

        let first = storage.store("first.txt", b"same").expect("first import");
        let second = storage
            .store("renamed.txt", b"same")
            .expect("duplicate import");

        assert!(!first.duplicate);
        assert!(second.duplicate);
        assert_eq!(first.storage_ref, second.storage_ref);
        assert_eq!(first.content_hash, second.content_hash);
    }

    #[test]
    fn rejects_unsupported_empty_and_oversized_payloads() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::with_max_bytes(directory.path(), 4);

        assert!(matches!(
            storage.store("conversation.json", b"{}"),
            Err(ImportStorageError::UnsupportedFormat)
        ));
        assert!(matches!(
            storage.store("conversation.jsonl", b""),
            Err(ImportStorageError::EmptyPayload)
        ));
        assert!(matches!(
            storage.store("conversation.jsonl", b"12345"),
            Err(ImportStorageError::PayloadTooLarge { max_bytes: 4 })
        ));
    }

    #[test]
    fn deduplicates_identical_bytes_across_supported_file_names() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());

        let markdown = storage.store("source.md", b"same").expect("markdown");
        let text = storage.store("source.txt", b"same").expect("text");

        assert_eq!(markdown.content_hash, text.content_hash);
        assert_eq!(markdown.storage_ref, text.storage_ref);
        assert!(text.duplicate);
        assert_eq!(text.format, ImportPayloadFormat::Text);
    }

    #[test]
    fn startup_recovery_removes_only_interrupted_temporary_files() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());
        let stored = storage.store("source.md", b"committed").expect("stored");
        let prefix = directory.path().join("aa");
        fs::create_dir_all(&prefix).expect("prefix");
        fs::write(prefix.join(".interrupted.tmp"), b"partial").expect("temporary");
        fs::write(prefix.join("unrelated.txt"), b"keep").expect("unrelated");
        assert_eq!(storage.recover_interrupted_writes().expect("recover"), 1);
        assert!(directory.path().join(stored.storage_ref).exists());
        assert!(prefix.join("unrelated.txt").exists());
    }

    #[test]
    fn reconciliation_removes_only_unreferenced_content_addressed_files() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());
        let committed = storage.store("source.md", b"committed").expect("stored");
        let orphan_dir = directory.path().join("bb");
        fs::create_dir_all(&orphan_dir).expect("orphan dir");
        let orphan = orphan_dir.join("b".repeat(64));
        fs::write(&orphan, b"orphan").expect("orphan");
        let mut refs = HashSet::new();
        refs.insert(committed.storage_ref.clone());
        assert_eq!(storage.reconcile_unreferenced(&refs).expect("reconcile"), 1);
        assert!(directory.path().join(committed.storage_ref).exists());
        assert!(!orphan.exists());
    }

    #[test]
    fn read_verified_text_truncates_on_utf8_boundary() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());
        let stored = storage
            .store("source.md", "你好世界".as_bytes())
            .expect("stored");
        let (content, length, truncated) = storage
            .read_verified_text(&stored.storage_ref, &stored.content_hash, 5)
            .expect("preview");
        assert_eq!((content.as_str(), length, truncated), ("你", 12, true));
    }

    #[test]
    fn read_verified_text_rejects_unregistered_path_shape() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());
        let error = storage
            .read_verified_text("../outside", &"a".repeat(64), 1024)
            .expect_err("unsafe reference");
        assert!(matches!(error, ImportStorageError::InvalidStorageRef));
    }

    #[test]
    fn read_verified_text_detects_tampered_content() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::new(directory.path());
        let stored = storage.store("source.txt", b"original").expect("stored");
        fs::write(directory.path().join(&stored.storage_ref), b"tampered").expect("tamper");
        let error = storage
            .read_verified_text(&stored.storage_ref, &stored.content_hash, 1024)
            .expect_err("hash mismatch");
        assert!(matches!(error, ImportStorageError::HashCollision));
    }

    #[test]
    fn read_verified_text_rejects_payload_that_grew_beyond_the_storage_limit() {
        let directory = TempDir::new().expect("temp directory");
        let storage = ImportStorage::with_max_bytes(directory.path(), 8);
        let stored = storage.store("source.txt", b"original").expect("stored");
        fs::write(directory.path().join(&stored.storage_ref), b"oversized").expect("grow file");
        let error = storage
            .read_verified_text(&stored.storage_ref, &stored.content_hash, 1024)
            .expect_err("oversized payload");
        assert!(matches!(
            error,
            ImportStorageError::PayloadTooLarge { max_bytes: 8 }
        ));
    }
}

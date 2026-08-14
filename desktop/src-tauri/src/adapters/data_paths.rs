use std::path::{Path, PathBuf};

use crate::domain::KernelResult;

/// Physical ownership boundaries for local MindScape data.
///
/// Structured domain state stays in SQLite, immutable imported payloads are
/// stored below `imports`, and recovery copies are isolated below `backups`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDataPaths {
    pub root: PathBuf,
    pub database: PathBuf,
    pub imports: PathBuf,
    pub backups: PathBuf,
}

impl LocalDataPaths {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            database: root.join("mindscape.sqlite3"),
            imports: root.join("imports"),
            backups: root.join("backups"),
            root,
        }
    }

    pub fn prepare(&self) -> KernelResult<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(&self.imports)?;
        std::fs::create_dir_all(&self.backups)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn prepares_separate_structured_import_and_backup_locations() {
        let directory = TempDir::new().expect("temp directory");
        let paths = LocalDataPaths::new(directory.path().join("mindscape"));

        paths.prepare().expect("prepare local data paths");

        assert!(paths.root.is_dir());
        assert!(paths.imports.is_dir());
        assert!(paths.backups.is_dir());
        assert_eq!(paths.database, paths.root.join("mindscape.sqlite3"));
    }
}

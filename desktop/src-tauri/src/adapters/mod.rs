mod credentials;
mod data_paths;
mod generic_import;
mod import_storage;
mod markdown_vault;
pub mod provider;
mod semantic_embedding;
mod sqlite;

pub use credentials::{CredentialService, CredentialStore};
pub use data_paths::LocalDataPaths;
pub use generic_import::{GenericImportError, ParsedImportBundle, parse_generic_import};
pub use import_storage::{ImportPayloadFormat, ImportStorage, StoredImportPayload};
pub use markdown_vault::{MarkdownEntityEdit, MarkdownVault};
pub use semantic_embedding::{
    SEMANTIC_MODEL_DIMENSIONS, SEMANTIC_MODEL_ID, SEMANTIC_MODEL_LICENSE, SEMANTIC_MODEL_REVISION,
    SEMANTIC_MODEL_VERSION, SemanticEmbedding, SemanticEmbeddingError, SemanticModelInstallError,
    SemanticModelPack, SemanticModelPackStatus,
};
pub use sqlite::{ImportKnowledgeProposalRequestReservation, SqliteStore};

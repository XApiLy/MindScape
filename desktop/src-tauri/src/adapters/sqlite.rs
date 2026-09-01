use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use rusqlite::{
    Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params, types::Type,
};

use crate::{
    adapters::provider::{
        DEFAULT_EMBEDDING_DIMENSIONS, EmbeddingRecord, KNOWLEDGE_ENTITY_INDEX_VERSION,
        KnowledgeFullTextMatch, KnowledgeVectorSnapshot, LOCAL_EMBEDDING_MODEL_VERSION,
        RetrievalAvailability, build_knowledge_embedding_record, knowledge_entity_source_hash,
        knowledge_search_text, normalize_retrieval_text,
    },
    domain::{
        AppendTurnInput, BranchType, CanvasNodePosition, CanvasViewportState, CompleteTurnInput,
        ContentBlock, ContextConstraint, ContextMessageRef, ContextSnapshot, ContextTurn,
        Conversation, ConversationEdge, ConversationGraph, ConversationNode, ConversationSummary,
        CreateConversationInput, FocusPromotionDecisionCommandInput,
        FocusPromotionDecisionProjection, FocusPromotionEntityMutation, FocusedContextSnapshot,
        KernelError, KernelResult, Message, MessageRole, OmittedContextRef, RunState,
        SCHEMA_VERSION, SaveCanvasViewportInput, UpdateNodePositionInput, Workspace,
        blocks_plain_text,
        contracts::{
            DiscussionLogProjection, DiscussionLogScope, EvidenceRef, KnowledgeEntity,
            KnowledgeRelation, KnowledgeStatus, ModelRunEvent, ModelRunEventEnvelope,
            ModelRunProjection, ModelRunRequest,
        },
        new_id, now_timestamp, plan_focus_promotion_decision, validate_focused_context_snapshot,
    },
};

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_conversations_workspace_updated
ON conversations(workspace_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS context_snapshots (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    parent_node_id TEXT,
    branch_type TEXT NOT NULL,
    current_input TEXT NOT NULL,
    selected_messages_json TEXT NOT NULL,
    omitted_messages_json TEXT NOT NULL,
    system_contract_version TEXT NOT NULL,
    estimated_tokens INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversation_nodes (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    parent_node_id TEXT REFERENCES conversation_nodes(id) ON DELETE RESTRICT,
    branch_type TEXT NOT NULL,
    title TEXT NOT NULL,
    user_message_id TEXT NOT NULL UNIQUE,
    assistant_message_id TEXT UNIQUE,
    provider_id TEXT,
    model_id TEXT,
    context_snapshot_id TEXT NOT NULL REFERENCES context_snapshots(id) ON DELETE RESTRICT,
    run_state TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_nodes_conversation_created
ON conversation_nodes(conversation_id, created_at);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL REFERENCES conversation_nodes(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_node ON messages(node_id, created_at);

CREATE TABLE IF NOT EXISTS conversation_edges (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    source_node_id TEXT NOT NULL REFERENCES conversation_nodes(id) ON DELETE CASCADE,
    target_node_id TEXT NOT NULL REFERENCES conversation_nodes(id) ON DELETE CASCADE,
    relation TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(source_node_id, target_node_id)
);

CREATE TABLE IF NOT EXISTS canvas_node_positions (
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL REFERENCES conversation_nodes(id) ON DELETE CASCADE,
    x REAL NOT NULL,
    y REAL NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(conversation_id, node_id)
);

CREATE TABLE IF NOT EXISTS domain_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_aggregate
ON domain_events(aggregate_type, aggregate_id, sequence);
"#;

const MIGRATION_V2: &str = r#"
ALTER TABLE messages ADD COLUMN content_blocks_json TEXT NOT NULL DEFAULT '[]';
"#;

const MIGRATION_V3: &str = r#"
ALTER TABLE context_snapshots
ADD COLUMN selected_import_refs_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE context_snapshots
ADD COLUMN explicit_constraints_json TEXT NOT NULL DEFAULT '[]';
"#;

const MIGRATION_V4: &str = r#"
CREATE TABLE model_runs (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL REFERENCES conversation_nodes(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    context_snapshot_id TEXT NOT NULL REFERENCES context_snapshots(id) ON DELETE RESTRICT,
    idempotency_key TEXT NOT NULL UNIQUE,
    state TEXT NOT NULL,
    request_json TEXT NOT NULL,
    last_sequence INTEGER NOT NULL DEFAULT 0,
    partial_content TEXT NOT NULL DEFAULT '',
    terminal_event_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_model_runs_node_created ON model_runs(node_id, created_at);
CREATE INDEX idx_model_runs_state ON model_runs(state, updated_at);

CREATE TABLE model_run_events (
    event_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES model_runs(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    occurred_at TEXT NOT NULL,
    event_json TEXT NOT NULL,
    UNIQUE(run_id, sequence)
);
"#;

const MIGRATION_V5: &str = r#"
CREATE TABLE canvas_viewports (
    conversation_id TEXT PRIMARY KEY REFERENCES conversations(id) ON DELETE CASCADE,
    x REAL NOT NULL,
    y REAL NOT NULL,
    zoom REAL NOT NULL CHECK(zoom > 0),
    updated_at TEXT NOT NULL
);
"#;

const MIGRATION_V6: &str = r#"
CREATE TABLE import_sources (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    original_file_name TEXT,
    content_hash TEXT NOT NULL UNIQUE,
    storage_ref TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_import_sources_conversation ON import_sources(conversation_id, created_at);
CREATE TABLE import_revisions (
    id TEXT PRIMARY KEY,
    import_source_id TEXT NOT NULL REFERENCES import_sources(id) ON DELETE CASCADE,
    adapter_id TEXT NOT NULL,
    adapter_version TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_import_revisions_source ON import_revisions(import_source_id, created_at);
CREATE TABLE imported_messages (
    id TEXT PRIMARY KEY,
    import_revision_id TEXT NOT NULL REFERENCES import_revisions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content_blocks_json TEXT NOT NULL,
    occurred_at TEXT,
    source_locator TEXT NOT NULL,
    parent_imported_message_id TEXT REFERENCES imported_messages(id) ON DELETE RESTRICT,
    platform_extension_json TEXT NOT NULL
);
CREATE INDEX idx_imported_messages_revision ON imported_messages(import_revision_id);
CREATE TABLE parse_reports (
    import_revision_id TEXT PRIMARY KEY REFERENCES import_revisions(id) ON DELETE CASCADE,
    conversation_count INTEGER NOT NULL,
    message_count INTEGER NOT NULL,
    attachment_count INTEGER NOT NULL,
    tool_record_count INTEGER NOT NULL,
    field_recovery_json TEXT NOT NULL,
    warnings_json TEXT NOT NULL,
    errors_json TEXT NOT NULL
);
"#;

const MIGRATION_V7: &str = r#"
CREATE TABLE focus_frame_lifecycle (
    focus_frame_id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    frame_json TEXT NOT NULL,
    status TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    updated_at TEXT NOT NULL,
    closed_at TEXT
);
CREATE INDEX idx_focus_frame_lifecycle_conversation
ON focus_frame_lifecycle(conversation_id, status, updated_at DESC);
"#;

const MIGRATION_V8: &str = r#"
CREATE TABLE focused_context_snapshots (
    focus_frame_id TEXT PRIMARY KEY REFERENCES focus_frame_lifecycle(focus_frame_id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    snapshot_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_focused_context_conversation ON focused_context_snapshots(conversation_id, updated_at DESC);
"#;

const MIGRATION_V9: &str = r#"
CREATE TABLE knowledge_entities (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, entity_json TEXT NOT NULL, revision INTEGER NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE knowledge_relations (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, relation_json TEXT NOT NULL, revision INTEGER NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE evidence_refs (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, evidence_json TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE INDEX idx_knowledge_entities_conversation ON knowledge_entities(conversation_id, updated_at DESC);
CREATE INDEX idx_knowledge_relations_conversation ON knowledge_relations(conversation_id, updated_at DESC);
"#;

const MIGRATION_V10: &str = r#"
CREATE VIRTUAL TABLE knowledge_entities_fts USING fts5(
    entity_id UNINDEXED,
    conversation_id UNINDEXED,
    search_text,
    tokenize = 'unicode61'
);
"#;

const MIGRATION_V11: &str = r#"
CREATE TABLE markdown_projections (
    id TEXT NOT NULL,
    target_entity_id TEXT NOT NULL REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    projection_revision INTEGER NOT NULL CHECK(projection_revision > 0),
    projection_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY(id, projection_revision),
    UNIQUE(target_entity_id, projection_revision)
);
CREATE INDEX idx_markdown_projection_target ON markdown_projections(target_entity_id, projection_revision DESC);
"#;

const MIGRATION_V12: &str = r#"
CREATE TABLE knowledge_vector_records (
    entity_id TEXT PRIMARY KEY REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    record_json TEXT NOT NULL,
    source_revision INTEGER NOT NULL CHECK(source_revision > 0),
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_knowledge_vector_conversation
ON knowledge_vector_records(conversation_id, updated_at DESC);
"#;

const MIGRATION_V13: &str = r#"
ALTER TABLE markdown_projections RENAME TO markdown_projections_v12;
CREATE TABLE markdown_projections (
    id TEXT NOT NULL,
    target_entity_id TEXT NOT NULL REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    projection_revision INTEGER NOT NULL CHECK(projection_revision > 0),
    projection_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY(id, projection_revision),
    UNIQUE(target_entity_id, projection_revision)
);
INSERT INTO markdown_projections SELECT id, target_entity_id, projection_revision, projection_json, created_at FROM markdown_projections_v12;
DROP TABLE markdown_projections_v12;
CREATE INDEX idx_markdown_projection_target ON markdown_projections(target_entity_id, projection_revision DESC);
"#;

const MIGRATION_V14: &str = r#"
CREATE TABLE IF NOT EXISTS discussion_log_revisions (
    id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    conversation_id TEXT REFERENCES conversations(id) ON DELETE CASCADE,
    project_id TEXT,
    focus_frame_id TEXT,
    projection_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(id, revision),
    CHECK(
        (conversation_id IS NOT NULL AND project_id IS NULL) OR
        (conversation_id IS NULL AND project_id IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS idx_discussion_logs_conversation
ON discussion_log_revisions(conversation_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_discussion_logs_project
ON discussion_log_revisions(project_id, updated_at DESC);
"#;

const MIGRATION_V15: &str = r#"
CREATE TABLE IF NOT EXISTS knowledge_vector_index_states (
    conversation_id TEXT PRIMARY KEY REFERENCES conversations(id) ON DELETE CASCADE,
    model_version TEXT NOT NULL,
    dimensions INTEGER NOT NULL CHECK(dimensions > 0),
    status TEXT NOT NULL CHECK(status IN ('ready', 'stale')),
    updated_at TEXT NOT NULL
);
"#;

const MIGRATION_V16: &str = r#"
CREATE TABLE IF NOT EXISTS focus_promotion_decisions (
    decision_id TEXT PRIMARY KEY,
    focus_frame_id TEXT NOT NULL REFERENCES focus_frame_lifecycle(focus_frame_id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    candidate_ref TEXT NOT NULL,
    decision_revision INTEGER NOT NULL CHECK(decision_revision = 1),
    request_json TEXT NOT NULL,
    projection_json TEXT NOT NULL,
    decided_at TEXT NOT NULL,
    UNIQUE(focus_frame_id, candidate_ref)
);
CREATE INDEX IF NOT EXISTS idx_focus_promotion_decisions_frame
ON focus_promotion_decisions(focus_frame_id, decided_at, decision_id);
"#;

const VECTOR_INDEX_READY: &str = "ready";
const VECTOR_INDEX_STALE: &str = "stale";

#[derive(Debug, Clone)]
pub struct SqliteStore {
    database_path: PathBuf,
    backup_dir: PathBuf,
}

impl SqliteStore {
    #[cfg(test)]
    pub fn open(database_path: impl AsRef<Path>) -> KernelResult<Self> {
        let database_path = database_path.as_ref();
        let backup_dir = database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        Self::open_with_backup_dir(database_path, backup_dir)
    }

    pub fn open_with_backup_dir(
        database_path: impl AsRef<Path>,
        backup_dir: impl AsRef<Path>,
    ) -> KernelResult<Self> {
        let database_path = database_path.as_ref().to_path_buf();
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let backup_dir = backup_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&backup_dir)?;

        let store = Self {
            database_path,
            backup_dir,
        };
        let mut connection = store.connection()?;
        let current_version = database_schema_version(&connection)?;
        if current_version > 0 && current_version < SCHEMA_VERSION {
            store.create_pre_migration_backup(&connection, current_version)?;
        }
        store.migrate(&mut connection)?;
        Ok(store)
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Commits one complete import revision as a single durable unit.
    ///
    /// The raw bytes must already have been committed by `ImportStorage`; this
    /// transaction only records its immutable reference and parsed projections.
    pub fn persist_import_bundle(
        &self,
        source: &crate::domain::contracts::ImportSource,
        revision: &crate::domain::contracts::ImportRevision,
        messages: &[crate::domain::contracts::ImportedMessage],
        report: &crate::domain::contracts::ParseReport,
    ) -> KernelResult<()> {
        crate::domain::validate_import_bundle(source, revision, messages, report)
            .map_err(|error| KernelError::Integrity(error.to_string()))?;
        let ordered_messages = parent_first_import_messages(messages);
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO import_sources
             (id, conversation_id, platform, original_file_name, content_hash, storage_ref, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source.id,
                source.conversation_id,
                serde_json::to_string(&source.platform)?,
                source.original_file_name,
                source.content_hash,
                source.storage_ref,
                source.created_at,
            ],
        )?;
        transaction.execute(
            "INSERT INTO import_revisions
             (id, import_source_id, adapter_id, adapter_version, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                revision.id,
                revision.import_source_id,
                revision.adapter_id,
                revision.adapter_version,
                serde_json::to_string(&revision.status)?,
                revision.created_at,
            ],
        )?;
        for message in ordered_messages {
            transaction.execute(
                "INSERT INTO imported_messages
                 (id, import_revision_id, role, content_blocks_json, occurred_at, source_locator,
                  parent_imported_message_id, platform_extension_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    message.id,
                    message.import_revision_id,
                    serde_json::to_string(&message.role)?,
                    serde_json::to_string(&message.content_blocks)?,
                    message.occurred_at,
                    message.source_locator,
                    message.parent_imported_message_id,
                    serde_json::to_string(&message.platform_extension)?,
                ],
            )?;
        }
        transaction.execute(
            "INSERT INTO parse_reports
             (import_revision_id, conversation_count, message_count, attachment_count,
              tool_record_count, field_recovery_json, warnings_json, errors_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                report.import_revision_id,
                report.conversation_count,
                report.message_count,
                report.attachment_count,
                report.tool_record_count,
                serde_json::to_string(&report.field_recovery)?,
                serde_json::to_string(&report.warnings)?,
                serde_json::to_string(&report.errors)?,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_import_sources(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<crate::domain::contracts::ImportSource>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, conversation_id, platform, original_file_name, content_hash, storage_ref, created_at
             FROM import_sources WHERE conversation_id = ?1 ORDER BY created_at DESC, id ASC",
        )?;
        let rows = statement.query_map([conversation_id], |row| {
            Ok(crate::domain::contracts::ImportSource {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                platform: parse_json(&row.get::<_, String>(2)?, 2)?,
                original_file_name: row.get(3)?,
                content_hash: row.get(4)?,
                storage_ref: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_import_storage_refs(&self) -> KernelResult<HashSet<String>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT storage_ref FROM import_sources")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<Result<HashSet<_>, _>>().map_err(Into::into)
    }

    pub fn get_import_bundle(
        &self,
        source_id: &str,
    ) -> KernelResult<crate::domain::contracts::ImportBundleQueryProjection> {
        use crate::domain::contracts::{
            ImportBundleQueryProjection, ImportRevision, ImportedMessage, ParseReport,
        };
        let connection = self.connection()?;
        let source = self
            .list_import_sources_for_id(&connection, source_id)?
            .ok_or_else(|| KernelError::NotFound {
                entity: "ImportSource",
                id: source_id.into(),
            })?;
        let revision = connection.query_row(
            "SELECT id, import_source_id, adapter_id, adapter_version, status, created_at
             FROM import_revisions WHERE import_source_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
            [source_id],
            |row| Ok(ImportRevision { id: row.get(0)?, import_source_id: row.get(1)?, adapter_id: row.get(2)?, adapter_version: row.get(3)?, status: parse_json(&row.get::<_, String>(4)?, 4)?, created_at: row.get(5)? }),
        )?;
        let mut statement = connection.prepare(
            "SELECT id, import_revision_id, role, content_blocks_json, occurred_at, source_locator,
                    parent_imported_message_id, platform_extension_json
             FROM imported_messages WHERE import_revision_id = ?1 ORDER BY source_locator, id",
        )?;
        let messages = statement
            .query_map([&revision.id], |row| {
                Ok(ImportedMessage {
                    id: row.get(0)?,
                    import_revision_id: row.get(1)?,
                    role: parse_json(&row.get::<_, String>(2)?, 2)?,
                    content_blocks: parse_json(&row.get::<_, String>(3)?, 3)?,
                    occurred_at: row.get(4)?,
                    source_locator: row.get(5)?,
                    parent_imported_message_id: row.get(6)?,
                    platform_extension: parse_json(&row.get::<_, String>(7)?, 7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let report = connection.query_row(
            "SELECT import_revision_id, conversation_count, message_count, attachment_count,
                    tool_record_count, field_recovery_json, warnings_json, errors_json
             FROM parse_reports WHERE import_revision_id = ?1",
            [&revision.id],
            |row| {
                Ok(ParseReport {
                    import_revision_id: row.get(0)?,
                    conversation_count: row.get(1)?,
                    message_count: row.get(2)?,
                    attachment_count: row.get(3)?,
                    tool_record_count: row.get(4)?,
                    field_recovery: parse_json(&row.get::<_, String>(5)?, 5)?,
                    warnings: parse_json(&row.get::<_, String>(6)?, 6)?,
                    errors: parse_json(&row.get::<_, String>(7)?, 7)?,
                })
            },
        )?;
        Ok(ImportBundleQueryProjection {
            source,
            revision,
            messages,
            report,
        })
    }

    fn list_import_sources_for_id(
        &self,
        connection: &Connection,
        source_id: &str,
    ) -> KernelResult<Option<crate::domain::contracts::ImportSource>> {
        connection.query_row(
            "SELECT id, conversation_id, platform, original_file_name, content_hash, storage_ref, created_at FROM import_sources WHERE id = ?1",
            [source_id],
            |row| Ok(crate::domain::contracts::ImportSource { id: row.get(0)?, conversation_id: row.get(1)?, platform: parse_json(&row.get::<_, String>(2)?, 2)?, original_file_name: row.get(3)?, content_hash: row.get(4)?, storage_ref: row.get(5)?, created_at: row.get(6)? }),
        ).optional().map_err(Into::into)
    }

    pub fn insert_focus_frame_lifecycle(
        &self,
        snapshot: &crate::domain::FocusFrameLifecycleSnapshot,
    ) -> KernelResult<()> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO focus_frame_lifecycle
             (focus_frame_id, conversation_id, frame_json, status, revision, updated_at, closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                snapshot.frame.id,
                snapshot.frame.conversation_id,
                serde_json::to_string(&snapshot.frame)?,
                serde_json::to_string(&snapshot.status)?,
                snapshot.revision,
                snapshot.updated_at,
                snapshot.closed_at,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_focused_context_snapshot(
        &self,
        snapshot: &FocusedContextSnapshot,
    ) -> KernelResult<()> {
        validate_focused_context_snapshot(snapshot)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO focused_context_snapshots (focus_frame_id, conversation_id, snapshot_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(focus_frame_id) DO UPDATE SET conversation_id=excluded.conversation_id,
               snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
            params![snapshot.focus_frame.id, snapshot.focus_frame.conversation_id,
                serde_json::to_string(snapshot)?, snapshot.context_snapshot.created_at],
        )?;
        Ok(())
    }

    pub fn get_focused_context_snapshot(
        &self,
        focus_frame_id: &str,
    ) -> KernelResult<Option<FocusedContextSnapshot>> {
        let connection = self.connection()?;
        let value = connection
            .query_row(
                "SELECT snapshot_json FROM focused_context_snapshots WHERE focus_frame_id = ?1",
                [focus_frame_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        value
            .map(|json| {
                let snapshot: FocusedContextSnapshot = serde_json::from_str(&json)?;
                validate_focused_context_snapshot(&snapshot)?;
                Ok(snapshot)
            })
            .transpose()
    }

    pub fn upsert_knowledge_entity(
        &self,
        conversation_id: &str,
        entity: &KnowledgeEntity,
    ) -> KernelResult<()> {
        entity.validate_for_conversation(conversation_id)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute("INSERT INTO knowledge_entities (id,conversation_id,entity_json,revision,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET conversation_id=excluded.conversation_id,entity_json=excluded.entity_json,revision=excluded.revision,updated_at=excluded.updated_at", params![entity.id, conversation_id, serde_json::to_string(entity)?, entity.revision, entity.updated_at])?;
        sync_knowledge_entity_fts(&transaction, conversation_id, entity)?;
        sync_knowledge_entity_vector(&transaction, conversation_id, entity)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_knowledge_entities(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<KnowledgeEntity>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT entity_json FROM knowledge_entities WHERE conversation_id=?1 ORDER BY updated_at DESC, id ASC")?;
        let rows = statement.query_map([conversation_id], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    pub fn get_knowledge_entity(
        &self,
        conversation_id: &str,
        entity_id: &str,
    ) -> KernelResult<KnowledgeEntity> {
        let connection = self.connection()?;
        let json = connection
            .query_row(
                "SELECT entity_json FROM knowledge_entities WHERE conversation_id=?1 AND id=?2",
                params![conversation_id, entity_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "KnowledgeEntity",
                id: entity_id.into(),
            })?;
        let entity: KnowledgeEntity = serde_json::from_str(&json)?;
        entity.validate_for_conversation(conversation_id)?;
        Ok(entity)
    }

    /// Commits the immutable decision and every SQLite-derived mutation as one unit.
    ///
    /// An IMMEDIATE transaction keeps lifecycle/entity version checks and the
    /// resulting entity, FTS, vector and relation writes in the same writer
    /// boundary. The stable decision ID is checked before reading mutable
    /// source state so an exact retry remains idempotent even after delete.
    pub fn persist_focus_promotion_decision(
        &self,
        input: &FocusPromotionDecisionCommandInput,
        actor: &crate::domain::contracts::GeneratorRef,
    ) -> KernelResult<FocusPromotionDecisionProjection> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some((persisted_input, persisted)) =
            load_focus_promotion_decision(&transaction, &input.decision_id)?
        {
            if persisted_input == *input {
                return Ok(persisted);
            }
            return Err(KernelError::Integrity(format!(
                "focus promotion decision {} idempotency key was reused with different input",
                input.decision_id
            )));
        }

        if let Some(existing_id) = transaction
            .query_row(
                "SELECT decision_id FROM focus_promotion_decisions
                 WHERE focus_frame_id = ?1 AND candidate_ref = ?2",
                params![input.focus_frame_id, input.candidate_ref],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Err(KernelError::Integrity(format!(
                "focus promotion candidate {} was already decided by {}",
                input.candidate_ref, existing_id
            )));
        }

        let lifecycle = load_focus_frame_lifecycle(&transaction, &input.focus_frame_id)?;
        let candidates = lifecycle.promotion_candidates()?.ok_or_else(|| {
            KernelError::Validation("FocusFrame has no promotion candidates".into())
        })?;
        let conversation_id = lifecycle.frame.conversation_id.clone();
        let entity = load_knowledge_entity(&transaction, &conversation_id, &input.candidate_ref)?;
        let plan = plan_focus_promotion_decision(input, &candidates, &lifecycle, &entity, actor)
            .map_err(map_focus_promotion_error)?;

        match &plan.entity_mutation {
            FocusPromotionEntityMutation::UpsertSource(source) => {
                update_knowledge_entity_revision(
                    &transaction,
                    &conversation_id,
                    source,
                    input.expected_entity_revision,
                )?;
            }
            FocusPromotionEntityMutation::Promote { source, promoted } => {
                promoted.validate_for_conversation(&conversation_id)?;
                transaction.execute(
                    "INSERT INTO knowledge_entities
                     (id, conversation_id, entity_json, revision, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        promoted.id,
                        conversation_id,
                        serde_json::to_string(promoted)?,
                        promoted.revision,
                        promoted.updated_at,
                    ],
                )?;
                sync_knowledge_entity_fts(&transaction, &conversation_id, promoted)?;
                sync_knowledge_entity_vector(&transaction, &conversation_id, promoted)?;
                update_knowledge_entity_revision(
                    &transaction,
                    &conversation_id,
                    source,
                    input.expected_entity_revision,
                )?;
            }
            FocusPromotionEntityMutation::DeleteSource {
                entity_id,
                expected_revision,
            } => {
                delete_knowledge_entity_revision(
                    &transaction,
                    &conversation_id,
                    entity_id,
                    *expected_revision,
                )?;
            }
        }

        transaction.execute(
            "INSERT INTO focus_promotion_decisions
             (decision_id, focus_frame_id, conversation_id, candidate_ref,
              decision_revision, request_json, projection_json, decided_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                plan.decision.decision_id,
                plan.decision.focus_frame_id,
                plan.decision.conversation_id,
                plan.decision.candidate_ref,
                plan.decision.decision_revision,
                serde_json::to_string(input)?,
                serde_json::to_string(&plan.decision)?,
                plan.decision.decided_at,
            ],
        )?;
        transaction.commit()?;
        Ok(plan.decision)
    }

    pub fn get_focus_promotion_decision(
        &self,
        decision_id: &str,
    ) -> KernelResult<FocusPromotionDecisionProjection> {
        if decision_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "focus promotion decision id must not be empty".into(),
            ));
        }
        let connection = self.connection()?;
        load_focus_promotion_decision(&connection, decision_id)?
            .map(|(_, projection)| projection)
            .ok_or_else(|| KernelError::NotFound {
                entity: "FocusPromotionDecision",
                id: decision_id.into(),
            })
    }

    pub fn replay_focus_promotion_decision(
        &self,
        input: &FocusPromotionDecisionCommandInput,
    ) -> KernelResult<Option<FocusPromotionDecisionProjection>> {
        let connection = self.connection()?;
        let Some((persisted_input, projection)) =
            load_focus_promotion_decision(&connection, &input.decision_id)?
        else {
            return Ok(None);
        };
        if persisted_input != *input {
            return Err(KernelError::Integrity(format!(
                "focus promotion decision {} idempotency key was reused with different input",
                input.decision_id
            )));
        }
        Ok(Some(projection))
    }

    pub fn list_focus_promotion_decisions(
        &self,
        focus_frame_id: &str,
    ) -> KernelResult<Vec<FocusPromotionDecisionProjection>> {
        if focus_frame_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "FocusFrame id must not be empty".into(),
            ));
        }
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT projection_json FROM focus_promotion_decisions
             WHERE focus_frame_id = ?1 ORDER BY decided_at, decision_id",
        )?;
        let rows = statement.query_map([focus_frame_id], |row| {
            parse_json::<FocusPromotionDecisionProjection>(&row.get::<_, String>(0)?, 0)
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_all_focus_promotion_decisions(
        &self,
    ) -> KernelResult<Vec<FocusPromotionDecisionProjection>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT projection_json FROM focus_promotion_decisions
             ORDER BY decided_at, decision_id",
        )?;
        let rows = statement.query_map([], |row| {
            parse_json::<FocusPromotionDecisionProjection>(&row.get::<_, String>(0)?, 0)
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_all_knowledge_entity_ids(&self) -> KernelResult<HashSet<String>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT id FROM knowledge_entities")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<HashSet<_>, _>>()?)
    }

    pub fn list_all_knowledge_entities(&self) -> KernelResult<Vec<KnowledgeEntity>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT entity_json FROM knowledge_entities ORDER BY updated_at DESC, id ASC",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| {
            let entity: KnowledgeEntity = serde_json::from_str(&row?)?;
            entity.validate()?;
            Ok(entity)
        })
        .collect()
    }

    pub fn list_all_knowledge_relations(&self) -> KernelResult<Vec<KnowledgeRelation>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT relation_json FROM knowledge_relations ORDER BY updated_at DESC, id ASC",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| {
            let relation: KnowledgeRelation = serde_json::from_str(&row?)?;
            relation.validate()?;
            Ok(relation)
        })
        .collect()
    }

    pub fn next_markdown_projection_revision(&self, entity_id: &str) -> KernelResult<u64> {
        let connection = self.connection()?;
        let current: u64 = connection.query_row(
            "SELECT COALESCE(MAX(projection_revision), 0) FROM markdown_projections WHERE target_entity_id=?1",
            [entity_id], |row| row.get(0))?;
        current
            .checked_add(1)
            .ok_or_else(|| KernelError::Integrity("MarkdownProjection revision overflowed".into()))
    }

    pub fn persist_markdown_projection(
        &self,
        projection: &crate::domain::contracts::MarkdownProjection,
    ) -> KernelResult<()> {
        projection.validate()?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO markdown_projections (id,target_entity_id,projection_revision,projection_json,created_at) VALUES (?1,?2,?3,?4,?5)",
            params![projection.id, projection.target_entity_id, projection.projection_revision, serde_json::to_string(projection)?, projection.created_at],
        )?;
        Ok(())
    }

    pub fn persist_markdown_entity_revision(
        &self,
        conversation_id: &str,
        entity: &KnowledgeEntity,
        projection: &crate::domain::contracts::MarkdownProjection,
    ) -> KernelResult<()> {
        entity.validate_for_conversation(conversation_id)?;
        projection.validate()?;
        if projection.target_entity_id != entity.id || projection.entity_revision != entity.revision
        {
            return Err(KernelError::Integrity(
                "MarkdownProjection does not match the Entity revision".into(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO knowledge_entities (id,conversation_id,entity_json,revision,updated_at)
             VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET conversation_id=excluded.conversation_id,
               entity_json=excluded.entity_json,revision=excluded.revision,updated_at=excluded.updated_at",
            params![entity.id, conversation_id, serde_json::to_string(entity)?, entity.revision, entity.updated_at],
        )?;
        sync_knowledge_entity_fts(&transaction, conversation_id, entity)?;
        sync_knowledge_entity_vector(&transaction, conversation_id, entity)?;
        transaction.execute(
            "INSERT INTO markdown_projections (id,target_entity_id,projection_revision,projection_json,created_at)
             VALUES (?1,?2,?3,?4,?5)",
            params![projection.id, projection.target_entity_id, projection.projection_revision, serde_json::to_string(projection)?, projection.created_at],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_markdown_projections(
        &self,
        entity_id: &str,
    ) -> KernelResult<Vec<crate::domain::contracts::MarkdownProjection>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT projection_json FROM markdown_projections WHERE target_entity_id=?1 ORDER BY projection_revision DESC")?;
        let rows = statement.query_map([entity_id], |row| row.get::<_, String>(0))?;
        rows.map(|row| {
            let projection: crate::domain::contracts::MarkdownProjection =
                serde_json::from_str(&row?)?;
            projection.validate()?;
            Ok(projection)
        })
        .collect()
    }

    pub fn persist_discussion_log_projection(
        &self,
        projection: &DiscussionLogProjection,
    ) -> KernelResult<()> {
        projection.validate()?;
        let (workspace_id, conversation_id, project_id, focus_frame_id) =
            discussion_scope_columns(&projection.log.scope);
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current_revision: u64 = transaction.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM discussion_log_revisions WHERE id=?1",
            [&projection.log.id],
            |row| row.get(0),
        )?;
        let expected_revision = current_revision
            .checked_add(1)
            .ok_or_else(|| KernelError::Integrity("DiscussionLog revision overflowed".into()))?;
        if projection.log.revision != expected_revision {
            return Err(KernelError::Integrity(format!(
                "discussion log {} revision conflict",
                projection.log.id
            )));
        }
        transaction.execute(
            "INSERT INTO discussion_log_revisions
             (id,revision,workspace_id,conversation_id,project_id,focus_frame_id,projection_json,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                projection.log.id,
                projection.log.revision,
                workspace_id,
                conversation_id,
                project_id,
                focus_frame_id,
                serde_json::to_string(projection)?,
                projection.log.created_at,
                projection.log.updated_at,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn next_discussion_log_revision(&self, discussion_log_id: &str) -> KernelResult<u64> {
        if discussion_log_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "DiscussionLog id must not be empty".into(),
            ));
        }
        let connection = self.connection()?;
        let current: u64 = connection.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM discussion_log_revisions WHERE id=?1",
            [discussion_log_id],
            |row| row.get(0),
        )?;
        current
            .checked_add(1)
            .ok_or_else(|| KernelError::Integrity("DiscussionLog revision overflowed".into()))
    }

    pub fn get_discussion_log_projection(
        &self,
        discussion_log_id: &str,
    ) -> KernelResult<DiscussionLogProjection> {
        if discussion_log_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "DiscussionLog id must not be empty".into(),
            ));
        }
        let connection = self.connection()?;
        let json = connection
            .query_row(
                "SELECT projection_json FROM discussion_log_revisions
                 WHERE id=?1 ORDER BY revision DESC LIMIT 1",
                [discussion_log_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "DiscussionLog",
                id: discussion_log_id.into(),
            })?;
        let projection: DiscussionLogProjection = serde_json::from_str(&json)?;
        projection.validate()?;
        Ok(projection)
    }

    pub fn list_conversation_discussion_logs(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<DiscussionLogProjection>> {
        list_discussion_logs(&self.connection()?, "conversation_id", conversation_id)
    }

    pub fn list_project_discussion_logs(
        &self,
        project_id: &str,
    ) -> KernelResult<Vec<DiscussionLogProjection>> {
        list_discussion_logs(&self.connection()?, "project_id", project_id)
    }

    pub fn list_all_discussion_logs(&self) -> KernelResult<Vec<DiscussionLogProjection>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT current.projection_json
             FROM discussion_log_revisions current
             JOIN (
               SELECT id, MAX(revision) AS revision
               FROM discussion_log_revisions
               GROUP BY id
             ) latest ON latest.id=current.id AND latest.revision=current.revision
             ORDER BY current.updated_at DESC, current.id ASC",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| {
            let projection: DiscussionLogProjection = serde_json::from_str(&row?)?;
            projection.validate()?;
            Ok(projection)
        })
        .collect()
    }

    pub fn search_knowledge_full_text(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
    ) -> KernelResult<Vec<KnowledgeFullTextMatch>> {
        let normalized = normalize_retrieval_text(query);
        if conversation_id.trim().is_empty() || normalized.is_empty() || limit == 0 {
            return Err(KernelError::Validation(
                "knowledge full-text query and limit must be valid".into(),
            ));
        }
        let fts_query = normalized
            .split_whitespace()
            .map(|term| format!("\"{term}\""))
            .collect::<Vec<_>>()
            .join(" OR ");
        let limit = i64::try_from(limit).map_err(|_| {
            KernelError::Validation("knowledge full-text limit is too large".into())
        })?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT entity_id, bm25(knowledge_entities_fts)
             FROM knowledge_entities_fts
             WHERE knowledge_entities_fts MATCH ?1 AND conversation_id = ?2
             ORDER BY bm25(knowledge_entities_fts), entity_id
             LIMIT ?3",
        )?;
        let rows = statement.query_map(params![fts_query, conversation_id, limit], |row| {
            let rank = row.get::<_, f64>(1)?;
            Ok(KnowledgeFullTextMatch {
                id: row.get(0)?,
                score: (1.0 / (1.0 + rank.abs())) as f32,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn load_knowledge_vector_snapshot(
        &self,
        conversation_id: &str,
    ) -> KernelResult<KnowledgeVectorSnapshot> {
        if conversation_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "knowledge vector scope must not be empty".into(),
            ));
        }
        let connection = self.connection()?;
        let index_state = read_vector_index_state(&connection, conversation_id)?;
        if index_state
            .as_ref()
            .is_some_and(|state| state.status != VECTOR_INDEX_READY)
        {
            return Ok(KnowledgeVectorSnapshot {
                availability: RetrievalAvailability::Unavailable,
                records: Vec::new(),
            });
        }
        let mut statement = connection.prepare(
            "SELECT record_json FROM knowledge_vector_records
             WHERE conversation_id = ?1 ORDER BY entity_id",
        )?;
        let rows = statement.query_map([conversation_id], |row| row.get::<_, String>(0))?;
        let serialized = rows.collect::<Result<Vec<_>, _>>()?;
        let mut records = Vec::with_capacity(serialized.len());
        for json in serialized {
            let Ok(record) = serde_json::from_str::<EmbeddingRecord>(&json) else {
                return Ok(KnowledgeVectorSnapshot {
                    availability: RetrievalAvailability::Unavailable,
                    records: Vec::new(),
                });
            };
            records.push(record);
        }
        if let Some(state) = index_state {
            let contract_matches = records.iter().all(|record| {
                record.metadata.model_version == state.model_version
                    && record.metadata.dimensions == state.dimensions
                    && record.vector.len() == state.dimensions
                    && record.vector.iter().all(|value| value.is_finite())
                    && record.vector.iter().any(|value| *value != 0.0)
            });
            if !contract_matches {
                return Ok(KnowledgeVectorSnapshot {
                    availability: RetrievalAvailability::Unavailable,
                    records: Vec::new(),
                });
            }
        }
        Ok(KnowledgeVectorSnapshot {
            availability: RetrievalAvailability::Available,
            records,
        })
    }

    pub fn rebuild_knowledge_vector_index(&self, conversation_id: &str) -> KernelResult<usize> {
        if conversation_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "knowledge vector scope must not be empty".into(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let conversation_exists = transaction
            .query_row(
                "SELECT 1 FROM conversations WHERE id = ?1",
                [conversation_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !conversation_exists {
            return Err(KernelError::NotFound {
                entity: "conversation",
                id: conversation_id.into(),
            });
        }
        let entities = {
            let mut statement = transaction.prepare(
                "SELECT entity_json FROM knowledge_entities
                 WHERE conversation_id = ?1 ORDER BY id",
            )?;
            let rows = statement.query_map([conversation_id], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|json| serde_json::from_str::<KnowledgeEntity>(&json).map_err(Into::into))
                .collect::<KernelResult<Vec<_>>>()?
        };
        transaction.execute(
            "DELETE FROM knowledge_vector_records WHERE conversation_id = ?1",
            [conversation_id],
        )?;
        transaction.execute(
            "DELETE FROM knowledge_vector_index_states WHERE conversation_id = ?1",
            [conversation_id],
        )?;
        let mut rebuilt = 0;
        for entity in &entities {
            entity.validate_for_conversation(conversation_id)?;
            if entity.status == KnowledgeStatus::Confirmed {
                sync_knowledge_entity_vector(&transaction, conversation_id, entity)?;
                rebuilt += 1;
            }
        }
        write_vector_index_state(
            &transaction,
            conversation_id,
            LOCAL_EMBEDDING_MODEL_VERSION,
            DEFAULT_EMBEDDING_DIMENSIONS,
            VECTOR_INDEX_READY,
        )?;
        transaction.commit()?;
        Ok(rebuilt)
    }

    pub fn replace_knowledge_vector_records(
        &self,
        conversation_id: &str,
        model_version: &str,
        dimensions: usize,
        records: &[EmbeddingRecord],
    ) -> KernelResult<usize> {
        if conversation_id.trim().is_empty() || model_version.trim().is_empty() || dimensions == 0 {
            return Err(KernelError::Validation(
                "knowledge vector scope and contract must not be empty".into(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut expected = HashMap::new();
        {
            let mut statement = transaction.prepare(
                "SELECT entity_json, revision, updated_at FROM knowledge_entities
                 WHERE conversation_id = ?1",
            )?;
            let rows = statement.query_map([conversation_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (json, revision, updated_at) = row?;
                let entity: KnowledgeEntity = serde_json::from_str(&json)?;
                entity.validate_for_conversation(conversation_id)?;
                if entity.status == KnowledgeStatus::Confirmed {
                    let source_hash = knowledge_entity_source_hash(&entity)
                        .map_err(|error| KernelError::Integrity(error.to_string()))?;
                    expected.insert(entity.id, (revision, updated_at, source_hash));
                }
            }
        }
        if records.len() != expected.len() {
            return Err(KernelError::Integrity(
                "semantic vector rebuild is incomplete for the confirmed knowledge scope".into(),
            ));
        }
        let mut seen = HashSet::new();
        for record in records {
            let Some((_, _, expected_source_hash)) = expected.get(&record.id) else {
                return Err(KernelError::Integrity(
                    "semantic vector records do not match the confirmed knowledge scope".into(),
                ));
            };
            if !seen.insert(record.id.as_str())
                || record.metadata.model_version != model_version
                || record.metadata.dimensions != dimensions
                || record.metadata.source_hash != *expected_source_hash
                || record.metadata.chunk_version != KNOWLEDGE_ENTITY_INDEX_VERSION
                || record.vector.len() != dimensions
                || record.vector.iter().any(|value| !value.is_finite())
                || record.vector.iter().all(|value| *value == 0.0)
            {
                return Err(KernelError::Integrity(
                    "semantic vector records do not match the active index contract".into(),
                ));
            }
        }

        transaction.execute(
            "DELETE FROM knowledge_vector_records WHERE conversation_id = ?1",
            [conversation_id],
        )?;
        for record in records {
            let (revision, updated_at, _) = &expected[&record.id];
            transaction.execute(
                "INSERT INTO knowledge_vector_records
                 (entity_id, conversation_id, record_json, source_revision, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    record.id,
                    conversation_id,
                    serde_json::to_string(record)?,
                    revision,
                    updated_at
                ],
            )?;
        }
        write_vector_index_state(
            &transaction,
            conversation_id,
            model_version,
            dimensions,
            VECTOR_INDEX_READY,
        )?;
        transaction.commit()?;
        Ok(records.len())
    }

    pub fn delete_knowledge_entity(
        &self,
        conversation_id: &str,
        entity_id: &str,
    ) -> KernelResult<bool> {
        if conversation_id.trim().is_empty() || entity_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "knowledge entity scope and id must not be empty".into(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let relation_ids = {
            let mut statement = transaction.prepare(
                "SELECT id, relation_json FROM knowledge_relations WHERE conversation_id = ?1",
            )?;
            let rows = statement.query_map([conversation_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|(id, json)| Ok((id, serde_json::from_str::<KnowledgeRelation>(&json)?)))
                .collect::<KernelResult<Vec<_>>>()?
                .into_iter()
                .filter(|(_, relation)| {
                    relation.source_entity_id == entity_id || relation.target_entity_id == entity_id
                })
                .map(|(id, _)| id)
                .collect::<Vec<_>>()
        };
        for relation_id in relation_ids {
            transaction.execute(
                "DELETE FROM knowledge_relations WHERE id = ?1 AND conversation_id = ?2",
                params![relation_id, conversation_id],
            )?;
        }
        transaction.execute(
            "DELETE FROM knowledge_entities_fts WHERE entity_id = ?1 AND conversation_id = ?2",
            params![entity_id, conversation_id],
        )?;
        transaction.execute(
            "DELETE FROM knowledge_vector_records WHERE entity_id = ?1 AND conversation_id = ?2",
            params![entity_id, conversation_id],
        )?;
        let deleted = transaction.execute(
            "DELETE FROM knowledge_entities WHERE id = ?1 AND conversation_id = ?2",
            params![entity_id, conversation_id],
        )?;
        transaction.commit()?;
        Ok(deleted == 1)
    }

    pub fn upsert_knowledge_relation(
        &self,
        conversation_id: &str,
        relation: &KnowledgeRelation,
    ) -> KernelResult<()> {
        relation.validate_for_conversation(conversation_id)?;
        let connection = self.connection()?;
        connection.execute("INSERT INTO knowledge_relations (id,conversation_id,relation_json,revision,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET conversation_id=excluded.conversation_id,relation_json=excluded.relation_json,revision=excluded.revision,updated_at=excluded.updated_at", params![relation.id, conversation_id, serde_json::to_string(relation)?, relation.revision, relation.updated_at])?;
        Ok(())
    }

    pub fn list_knowledge_relations(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<KnowledgeRelation>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT relation_json FROM knowledge_relations WHERE conversation_id=?1 ORDER BY updated_at DESC, id ASC")?;
        let rows = statement.query_map([conversation_id], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    pub fn upsert_evidence_ref(
        &self,
        conversation_id: &str,
        evidence: &crate::domain::contracts::EvidenceRef,
    ) -> KernelResult<()> {
        if conversation_id.trim().is_empty() || evidence.id.trim().is_empty() {
            return Err(KernelError::Validation(
                "evidence scope and id must not be empty".into(),
            ));
        }
        let connection = self.connection()?;
        connection.execute("INSERT INTO evidence_refs (id,conversation_id,evidence_json,created_at) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET conversation_id=excluded.conversation_id,evidence_json=excluded.evidence_json,created_at=excluded.created_at", params![evidence.id, conversation_id, serde_json::to_string(evidence)?, evidence.created_at])?;
        Ok(())
    }

    pub fn update_focus_frame_lifecycle(
        &self,
        snapshot: &crate::domain::FocusFrameLifecycleSnapshot,
        expected_revision: u64,
    ) -> KernelResult<()> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE focus_frame_lifecycle
             SET frame_json = ?1, status = ?2, revision = ?3, updated_at = ?4, closed_at = ?5
             WHERE focus_frame_id = ?6 AND conversation_id = ?7 AND revision = ?8",
            params![
                serde_json::to_string(&snapshot.frame)?,
                serde_json::to_string(&snapshot.status)?,
                snapshot.revision,
                snapshot.updated_at,
                snapshot.closed_at,
                snapshot.frame.id,
                snapshot.frame.conversation_id,
                expected_revision,
            ],
        )?;
        if changed != 1 {
            return Err(KernelError::Integrity(format!(
                "focus frame {} revision conflict",
                snapshot.frame.id
            )));
        }
        Ok(())
    }

    pub fn get_focus_frame_lifecycle(
        &self,
        focus_frame_id: &str,
    ) -> KernelResult<crate::domain::FocusFrameLifecycleSnapshot> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT frame_json, status, revision, updated_at, closed_at
                 FROM focus_frame_lifecycle WHERE focus_frame_id = ?1",
                [focus_frame_id],
                |row| {
                    Ok(crate::domain::FocusFrameLifecycleSnapshot {
                        contract_version: crate::domain::FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
                        frame: parse_json(&row.get::<_, String>(0)?, 0)?,
                        status: parse_json(&row.get::<_, String>(1)?, 1)?,
                        revision: row.get(2)?,
                        updated_at: row.get(3)?,
                        closed_at: row.get(4)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "FocusFrameLifecycle",
                id: focus_frame_id.into(),
            })
    }

    pub fn list_focus_frame_lifecycles(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<crate::domain::FocusFrameLifecycleSnapshot>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT frame_json, status, revision, updated_at, closed_at
             FROM focus_frame_lifecycle
             WHERE conversation_id = ?1
             ORDER BY updated_at DESC, focus_frame_id ASC",
        )?;
        let rows = statement.query_map([conversation_id], |row| {
            Ok(crate::domain::FocusFrameLifecycleSnapshot {
                contract_version: crate::domain::FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
                frame: parse_json(&row.get::<_, String>(0)?, 0)?,
                status: parse_json(&row.get::<_, String>(1)?, 1)?,
                revision: row.get(2)?,
                updated_at: row.get(3)?,
                closed_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    #[cfg(test)]
    fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    pub fn schema_version(&self) -> KernelResult<i64> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn ensure_default_workspace(&self) -> KernelResult<Workspace> {
        let mut connection = self.connection()?;
        let existing = connection
            .query_row(
                "SELECT id, name, created_at, updated_at FROM workspaces ORDER BY created_at LIMIT 1",
                [],
                row_to_workspace,
            )
            .optional()?;

        if let Some(workspace) = existing {
            return Ok(workspace);
        }

        let transaction = connection.transaction()?;
        let now = now_timestamp();
        let workspace = Workspace {
            id: new_id("workspace"),
            name: "本地工作区".into(),
            created_at: now.clone(),
            updated_at: now,
        };
        transaction.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                workspace.id,
                workspace.name,
                workspace.created_at,
                workspace.updated_at
            ],
        )?;
        append_event(
            &transaction,
            "workspace",
            &workspace.id,
            "workspace.created",
            &workspace,
        )?;
        transaction.commit()?;
        Ok(workspace)
    }

    pub fn list_conversations(&self, workspace_id: &str) -> KernelResult<Vec<ConversationSummary>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT c.id, c.workspace_id, c.title, c.created_at, c.updated_at, c.revision,
                    COUNT(n.id) AS node_count
             FROM conversations c
             LEFT JOIN conversation_nodes n ON n.conversation_id = c.id
             WHERE c.workspace_id = ?1
             GROUP BY c.id
             ORDER BY c.updated_at DESC",
        )?;
        let rows = statement.query_map([workspace_id], |row| {
            Ok(ConversationSummary {
                conversation: row_to_conversation(row)?,
                node_count: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn create_conversation(
        &self,
        input: &CreateConversationInput,
    ) -> KernelResult<Conversation> {
        let mut connection = self.connection()?;
        let workspace_exists = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = ?1)",
            [&input.workspace_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !workspace_exists {
            return Err(KernelError::NotFound {
                entity: "workspace",
                id: input.workspace_id.clone(),
            });
        }

        let transaction = connection.transaction()?;
        let now = now_timestamp();
        let conversation = Conversation {
            id: new_id("conversation"),
            workspace_id: input.workspace_id.clone(),
            title: input.title.trim().to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: 1,
        };
        transaction.execute(
            "INSERT INTO conversations
             (id, workspace_id, title, created_at, updated_at, revision)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                conversation.id,
                conversation.workspace_id,
                conversation.title,
                conversation.created_at,
                conversation.updated_at,
                conversation.revision,
            ],
        )?;
        transaction.execute(
            "UPDATE workspaces SET updated_at = ?1 WHERE id = ?2",
            params![now, input.workspace_id],
        )?;
        append_event(
            &transaction,
            "conversation",
            &conversation.id,
            "conversation.created",
            &conversation,
        )?;
        transaction.commit()?;
        Ok(conversation)
    }

    pub fn load_conversation_graph(
        &self,
        conversation_id: &str,
    ) -> KernelResult<ConversationGraph> {
        let connection = self.connection()?;
        let conversation = connection
            .query_row(
                "SELECT id, workspace_id, title, created_at, updated_at, revision
                 FROM conversations WHERE id = ?1",
                [conversation_id],
                row_to_conversation,
            )
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "conversation",
                id: conversation_id.to_string(),
            })?;

        let nodes = query_nodes(&connection, conversation_id)?;
        let edges = query_edges(&connection, conversation_id)?;
        let positions = query_positions(&connection, conversation_id)?;

        Ok(ConversationGraph {
            conversation,
            nodes,
            edges,
            positions,
        })
    }

    pub fn path_to_node(
        &self,
        conversation_id: &str,
        node_id: Option<&str>,
    ) -> KernelResult<Vec<ContextTurn>> {
        let Some(mut current_id) = node_id.map(str::to_string) else {
            return Ok(Vec::new());
        };
        let connection = self.connection()?;
        let mut path = Vec::new();
        let mut visited = HashSet::new();

        loop {
            if !visited.insert(current_id.clone()) {
                return Err(KernelError::Integrity(format!(
                    "cycle detected while reading node path at {current_id}"
                )));
            }

            let record = connection
                .query_row(
                    "SELECT n.parent_node_id, n.user_message_id, um.content_blocks_json,
                            n.assistant_message_id, am.content_blocks_json
                     FROM conversation_nodes n
                     JOIN messages um ON um.id = n.user_message_id
                     LEFT JOIN messages am ON am.id = n.assistant_message_id
                     WHERE n.id = ?1 AND n.conversation_id = ?2",
                    params![current_id, conversation_id],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| KernelError::NotFound {
                    entity: "conversation node",
                    id: current_id.clone(),
                })?;

            path.push(ContextTurn {
                node_id: current_id.clone(),
                user_message_id: record.1,
                user_content_blocks: parse_json_value(&record.2)?,
                assistant_message_id: record.3,
                assistant_content_blocks: record.4.as_deref().map(parse_json_value).transpose()?,
            });

            match record.0 {
                Some(parent_id) => current_id = parent_id,
                None => break,
            }
        }

        path.reverse();
        Ok(path)
    }

    pub fn insert_turn(
        &self,
        input: &AppendTurnInput,
        snapshot: &ContextSnapshot,
    ) -> KernelResult<ConversationNode> {
        let mut connection = self.connection()?;
        let conversation_exists = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = ?1)",
            [&input.conversation_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !conversation_exists {
            return Err(KernelError::NotFound {
                entity: "conversation",
                id: input.conversation_id.clone(),
            });
        }

        let transaction = connection.transaction()?;
        let now = now_timestamp();
        let node_id = new_id("node");
        let user_message = Message {
            id: new_id("message"),
            conversation_id: input.conversation_id.clone(),
            node_id: node_id.clone(),
            role: MessageRole::User,
            content_blocks: vec![ContentBlock::text(input.prompt.trim())],
            created_at: now.clone(),
        };

        insert_snapshot(&transaction, snapshot)?;
        transaction.execute(
            "INSERT INTO conversation_nodes
             (id, conversation_id, parent_node_id, branch_type, title, user_message_id,
              assistant_message_id, provider_id, model_id, context_snapshot_id, run_state,
              created_at, updated_at, revision)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10, ?11, ?12, 1)",
            params![
                node_id,
                input.conversation_id,
                input.parent_node_id,
                input.branch_type.as_db(),
                input.title.trim(),
                user_message.id,
                input.provider_id,
                input.model_id,
                snapshot.id,
                RunState::Pending.as_db(),
                now,
                now,
            ],
        )?;
        insert_message(&transaction, &user_message)?;

        if let Some(parent_id) = &input.parent_node_id {
            let edge = ConversationEdge {
                id: new_id("edge"),
                conversation_id: input.conversation_id.clone(),
                source_node_id: parent_id.clone(),
                target_node_id: node_id.clone(),
                relation: input.branch_type,
                created_at: now.clone(),
            };
            transaction.execute(
                "INSERT INTO conversation_edges
                 (id, conversation_id, source_node_id, target_node_id, relation, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    edge.id,
                    edge.conversation_id,
                    edge.source_node_id,
                    edge.target_node_id,
                    edge.relation.as_db(),
                    edge.created_at,
                ],
            )?;
        }

        transaction.execute(
            "UPDATE conversations
             SET updated_at = ?1, revision = revision + 1
             WHERE id = ?2",
            params![now, input.conversation_id],
        )?;
        append_event(
            &transaction,
            "conversation",
            &input.conversation_id,
            "conversation.turn_appended",
            &serde_json::json!({
                "nodeId": node_id,
                "parentNodeId": input.parent_node_id,
                "branchType": input.branch_type,
                "contextSnapshotId": snapshot.id,
                "userMessageId": user_message.id,
            }),
        )?;
        transaction.commit()?;
        self.load_node(&node_id)
    }

    pub fn complete_turn(&self, input: &CompleteTurnInput) -> KernelResult<ConversationNode> {
        let mut connection = self.connection()?;
        let node_record = connection
            .query_row(
                "SELECT conversation_id, assistant_message_id, run_state
                 FROM conversation_nodes WHERE id = ?1",
                [&input.node_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "conversation node",
                id: input.node_id.clone(),
            })?;

        if node_record.1.is_some() || node_record.2 == RunState::Completed.as_db() {
            return Err(KernelError::Validation(
                "a completed turn is immutable; create a retry node instead".into(),
            ));
        }

        let transaction = connection.transaction()?;
        let now = now_timestamp();
        let assistant_message = Message {
            id: new_id("message"),
            conversation_id: node_record.0.clone(),
            node_id: input.node_id.clone(),
            role: MessageRole::Assistant,
            content_blocks: vec![ContentBlock::text(input.content.trim())],
            created_at: now.clone(),
        };
        insert_message(&transaction, &assistant_message)?;
        transaction.execute(
            "UPDATE conversation_nodes
             SET assistant_message_id = ?1, provider_id = ?2, model_id = ?3,
                 run_state = ?4, updated_at = ?5, revision = revision + 1
             WHERE id = ?6",
            params![
                assistant_message.id,
                input.provider_id,
                input.model_id,
                RunState::Completed.as_db(),
                now,
                input.node_id,
            ],
        )?;
        transaction.execute(
            "UPDATE conversations
             SET updated_at = ?1, revision = revision + 1 WHERE id = ?2",
            params![now, node_record.0],
        )?;
        append_event(
            &transaction,
            "conversation",
            &node_record.0,
            "conversation.turn_completed",
            &serde_json::json!({
                "nodeId": input.node_id,
                "assistantMessageId": assistant_message.id,
                "providerId": input.provider_id,
                "modelId": input.model_id,
            }),
        )?;
        transaction.commit()?;
        self.load_node(&input.node_id)
    }

    pub fn get_context_snapshot(&self, snapshot_id: &str) -> KernelResult<ContextSnapshot> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, conversation_id, parent_node_id, branch_type, current_input,
                        selected_messages_json, selected_import_refs_json,
                        explicit_constraints_json, omitted_messages_json,
                        system_contract_version, estimated_tokens, created_at
                 FROM context_snapshots WHERE id = ?1",
                [snapshot_id],
                row_to_snapshot,
            )
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "context snapshot",
                id: snapshot_id.to_string(),
            })
    }

    pub fn update_node_position(&self, input: &UpdateNodePositionInput) -> KernelResult<()> {
        let mut connection = self.connection()?;
        let node_exists = connection.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM conversation_nodes WHERE id = ?1 AND conversation_id = ?2
             )",
            params![input.node_id, input.conversation_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !node_exists {
            return Err(KernelError::NotFound {
                entity: "conversation node",
                id: input.node_id.clone(),
            });
        }

        let transaction = connection.transaction()?;
        let now = now_timestamp();
        transaction.execute(
            "INSERT INTO canvas_node_positions (conversation_id, node_id, x, y, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(conversation_id, node_id)
             DO UPDATE SET x = excluded.x, y = excluded.y, updated_at = excluded.updated_at",
            params![input.conversation_id, input.node_id, input.x, input.y, now],
        )?;
        append_event(
            &transaction,
            "conversation",
            &input.conversation_id,
            "conversation.node_position_updated",
            input,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn save_canvas_viewport(
        &self,
        input: &SaveCanvasViewportInput,
    ) -> KernelResult<CanvasViewportState> {
        input.validate()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let conversation_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = ?1)",
            [&input.conversation_id],
            |row| row.get(0),
        )?;
        if !conversation_exists {
            return Err(KernelError::NotFound {
                entity: "conversation",
                id: input.conversation_id.clone(),
            });
        }
        let updated_at = now_timestamp();
        transaction.execute(
            "INSERT INTO canvas_viewports (conversation_id, x, y, zoom, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(conversation_id) DO UPDATE SET
                x = excluded.x, y = excluded.y, zoom = excluded.zoom,
                updated_at = excluded.updated_at",
            params![
                input.conversation_id,
                input.x,
                input.y,
                input.zoom,
                updated_at
            ],
        )?;
        transaction.commit()?;
        Ok(CanvasViewportState {
            conversation_id: input.conversation_id.clone(),
            x: input.x,
            y: input.y,
            zoom: input.zoom,
            updated_at,
        })
    }

    pub fn get_canvas_viewport(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Option<CanvasViewportState>> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT conversation_id, x, y, zoom, updated_at
                 FROM canvas_viewports WHERE conversation_id = ?1",
                [conversation_id],
                |row| {
                    Ok(CanvasViewportState {
                        conversation_id: row.get(0)?,
                        x: row.get(1)?,
                        y: row.get(2)?,
                        zoom: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn recover_interrupted_runs(&self) -> KernelResult<usize> {
        let incomplete = self
            .list_model_runs(None)?
            .into_iter()
            .filter(|run| matches!(run.state, RunState::Pending | RunState::Streaming))
            .collect::<Vec<_>>();
        for run in &incomplete {
            self.record_model_run_event(&ModelRunEventEnvelope {
                contract_version: crate::domain::contracts::RUNTIME_CONTRACT_VERSION.into(),
                event_id: new_id("run-event"),
                run_id: run.run_id.clone(),
                node_id: run.node_id.clone(),
                sequence: run.last_sequence + 1,
                occurred_at: now_timestamp(),
                event: ModelRunEvent::application_interrupted(!run.partial_content.is_empty()),
            })?;
        }

        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let now = now_timestamp();
        let orphaned = transaction.execute(
            "UPDATE conversation_nodes
             SET run_state = 'failed', updated_at = ?1, revision = revision + 1
             WHERE run_state IN ('pending', 'streaming')
               AND NOT EXISTS (SELECT 1 FROM model_runs r WHERE r.node_id = conversation_nodes.id)",
            [&now],
        )?;
        transaction.commit()?;
        Ok(incomplete.len() + orphaned)
    }

    pub fn list_model_runs(
        &self,
        conversation_id: Option<&str>,
    ) -> KernelResult<Vec<ModelRunProjection>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, conversation_id, node_id, provider_id, model_id, state,
                    last_sequence, partial_content, terminal_event_json, updated_at
             FROM model_runs
             WHERE (?1 IS NULL OR conversation_id = ?1)
             ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([conversation_id], |row| {
            let state = parse_run_state(row.get(5)?, 5)?;
            let terminal_json: Option<String> = row.get(8)?;
            let terminal_event = terminal_json
                .map(|json| parse_json::<ModelRunEvent>(&json, 8))
                .transpose()?;
            Ok(ModelRunProjection {
                run_id: row.get(0)?,
                conversation_id: row.get(1)?,
                node_id: row.get(2)?,
                provider_id: row.get(3)?,
                model_id: row.get(4)?,
                state,
                last_sequence: row.get(6)?,
                partial_content: row.get(7)?,
                terminal_event,
                updated_at: row.get(9)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    #[cfg(test)]
    pub fn event_count(&self, aggregate_id: &str) -> KernelResult<i64> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT COUNT(*) FROM domain_events WHERE aggregate_id = ?1",
                [aggregate_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn create_model_run(&self, request: &ModelRunRequest) -> KernelResult<()> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO model_runs
             (id, conversation_id, node_id, provider_id, model_id, context_snapshot_id,
              idempotency_key, state, request_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9, ?9)
             ON CONFLICT(idempotency_key) DO NOTHING",
            params![
                request.run_id,
                request.conversation_id,
                request.node_id,
                request.provider_id,
                request.model_id,
                request.context_snapshot.id,
                request.idempotency_key,
                serde_json::to_string(request)?,
                request.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn model_run_request_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> KernelResult<Option<ModelRunRequest>> {
        let connection = self.connection()?;
        let json: Option<String> = connection
            .query_row(
                "SELECT request_json FROM model_runs WHERE idempotency_key = ?1",
                [idempotency_key],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| parse_json_value(&value)).transpose()
    }

    pub fn load_model_run_node(&self, node_id: &str) -> KernelResult<ConversationNode> {
        self.load_node(node_id)
    }

    pub fn record_model_run_event(&self, envelope: &ModelRunEventEnvelope) -> KernelResult<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let current_sequence: i64 = transaction
            .query_row(
                "SELECT last_sequence FROM model_runs WHERE id = ?1",
                [&envelope.run_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "model run",
                id: envelope.run_id.clone(),
            })?;

        if current_sequence >= envelope.sequence as i64 {
            let existing: Option<String> = transaction
                .query_row(
                    "SELECT event_id FROM model_run_events WHERE run_id = ?1 AND sequence = ?2",
                    params![envelope.run_id, envelope.sequence],
                    |row| row.get(0),
                )
                .optional()?;
            return if existing.as_deref() == Some(&envelope.event_id) {
                Ok(())
            } else {
                Err(KernelError::Integrity(format!(
                    "model run {} received duplicate or stale sequence {}",
                    envelope.run_id, envelope.sequence
                )))
            };
        }
        if envelope.sequence != current_sequence as u64 + 1 {
            return Err(KernelError::Integrity(format!(
                "model run {} expected sequence {}, received {}",
                envelope.run_id,
                current_sequence + 1,
                envelope.sequence
            )));
        }

        let state = envelope.event.resulting_state().as_db();
        let delta = match &envelope.event {
            ModelRunEvent::TextDelta { delta } => Some(delta.as_str()),
            _ => None,
        };
        let terminal = envelope.event.is_terminal().then_some(&envelope.event);
        transaction.execute(
            "INSERT INTO model_run_events (event_id, run_id, sequence, occurred_at, event_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                envelope.event_id,
                envelope.run_id,
                envelope.sequence,
                envelope.occurred_at,
                serde_json::to_string(&envelope.event)?,
            ],
        )?;
        transaction.execute(
            "UPDATE model_runs
             SET state = ?1, last_sequence = ?2,
                 partial_content = partial_content || COALESCE(?3, ''),
                 terminal_event_json = COALESCE(?4, terminal_event_json), updated_at = ?5
             WHERE id = ?6",
            params![
                state,
                envelope.sequence,
                delta,
                terminal.map(serde_json::to_string).transpose()?,
                envelope.occurred_at,
                envelope.run_id,
            ],
        )?;
        apply_run_event_to_node(&transaction, envelope, state)?;
        transaction.commit()?;
        Ok(())
    }

    fn load_node(&self, node_id: &str) -> KernelResult<ConversationNode> {
        let connection = self.connection()?;
        connection
            .query_row(NODE_SELECT_BY_ID, [node_id], row_to_node)
            .optional()?
            .ok_or_else(|| KernelError::NotFound {
                entity: "conversation node",
                id: node_id.to_string(),
            })
    }

    fn connection(&self) -> KernelResult<Connection> {
        let connection = Connection::open(&self.database_path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(connection)
    }

    fn create_pre_migration_backup(
        &self,
        connection: &Connection,
        current_version: i64,
    ) -> KernelResult<PathBuf> {
        connection.execute_batch("PRAGMA wal_checkpoint(FULL);")?;
        let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%6fZ");
        let backup_path = self.backup_dir.join(format!(
            "mindscape-schema-v{current_version}-{timestamp}.sqlite3"
        ));
        std::fs::copy(&self.database_path, &backup_path)?;
        Ok(backup_path)
    }

    fn migrate(&self, connection: &mut Connection) -> KernelResult<()> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
             );",
        )?;
        let mut current_version: i64 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;

        if current_version > SCHEMA_VERSION {
            return Err(KernelError::Integrity(format!(
                "database schema v{current_version} is newer than supported v{SCHEMA_VERSION}"
            )));
        }

        if current_version < 1 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V1)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 1;
        }

        if current_version < 2 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V2)?;
            let legacy_messages = {
                let mut statement = transaction.prepare("SELECT id, content FROM messages")?;
                let rows = statement.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            };
            for (message_id, content) in legacy_messages {
                let blocks = vec![ContentBlock::text(content)];
                transaction.execute(
                    "UPDATE messages SET content_blocks_json = ?1 WHERE id = ?2",
                    params![serde_json::to_string(&blocks)?, message_id],
                )?;
            }
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (2, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 2;
        }

        if current_version < 3 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V3)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (3, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 3;
        }

        if current_version < 4 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V4)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (4, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 4;
        }

        if current_version < 5 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V5)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (5, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
        }

        if current_version < 6 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V6)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (6, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
        }

        if current_version < 7 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V7)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (7, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 7;
        }

        if current_version < 8 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V8)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (8, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 8;
        }
        if current_version < 9 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V9)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (9, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 9;
        }
        if current_version < 10 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V10)?;
            let entities = {
                let mut statement = transaction.prepare(
                    "SELECT conversation_id, entity_json FROM knowledge_entities ORDER BY id",
                )?;
                let rows = statement.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            };
            for (conversation_id, json) in entities {
                let entity: KnowledgeEntity = serde_json::from_str(&json)?;
                entity.validate_for_conversation(&conversation_id)?;
                sync_knowledge_entity_fts(&transaction, &conversation_id, &entity)?;
            }
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (10, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 10;
        }

        if current_version < 11 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V11)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (11, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 11;
        }

        if current_version < 12 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V12)?;
            let entities = {
                let mut statement = transaction.prepare(
                    "SELECT conversation_id, entity_json FROM knowledge_entities ORDER BY id",
                )?;
                let rows = statement.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                rows.collect::<Result<Vec<_>, _>>()?
            };
            for (conversation_id, json) in entities {
                let entity: KnowledgeEntity = serde_json::from_str(&json)?;
                entity.validate_for_conversation(&conversation_id)?;
                sync_knowledge_entity_vector(&transaction, &conversation_id, &entity)?;
            }
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (12, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 12;
        }

        if current_version < 13 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V13)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (13, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 13;
        }

        if current_version < 14 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V14)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (14, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 14;
        }

        if current_version < 15 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V15)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (15, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
            current_version = 15;
        }

        if current_version < 16 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MIGRATION_V16)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (16, ?1)",
                [now_timestamp()],
            )?;
            transaction.commit()?;
        }

        debug_assert_eq!(SCHEMA_VERSION, 16);
        Ok(())
    }
}

fn load_focus_promotion_decision(
    connection: &Connection,
    decision_id: &str,
) -> KernelResult<
    Option<(
        FocusPromotionDecisionCommandInput,
        FocusPromotionDecisionProjection,
    )>,
> {
    let stored = connection
        .query_row(
            "SELECT request_json, projection_json FROM focus_promotion_decisions
             WHERE decision_id = ?1",
            [decision_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    stored
        .map(|(request_json, projection_json)| {
            let input: FocusPromotionDecisionCommandInput = serde_json::from_str(&request_json)?;
            let projection: FocusPromotionDecisionProjection =
                serde_json::from_str(&projection_json)?;
            validate_persisted_focus_promotion_decision(&input, &projection)?;
            Ok((input, projection))
        })
        .transpose()
}

fn validate_persisted_focus_promotion_decision(
    input: &FocusPromotionDecisionCommandInput,
    projection: &FocusPromotionDecisionProjection,
) -> KernelResult<()> {
    let expected_source_revision = match input.action {
        crate::domain::FocusPromotionDecisionAction::Delete => None,
        _ => Some(
            input
                .expected_entity_revision
                .checked_add(1)
                .ok_or_else(|| {
                    KernelError::Integrity("focus promotion entity revision overflowed".into())
                })?,
        ),
    };
    let consistent = projection.contract_version
        == crate::domain::FOCUS_PROMOTION_DECISION_CONTRACT_VERSION
        && projection.decision_id == input.decision_id
        && projection.focus_frame_id == input.focus_frame_id
        && !projection.conversation_id.trim().is_empty()
        && projection.candidate_ref == input.candidate_ref
        && projection.action == input.action
        && projection.target_scope == input.target_scope
        && projection.promoted_entity_id == input.promoted_entity_id
        && projection.source_entity_revision == expected_source_revision
        && projection.decision_revision == 1
        && projection.memory_version == input.expected_memory_version
        && projection.lifecycle_revision == input.expected_lifecycle_revision
        && projection.decided_at == input.decided_at;
    if !consistent {
        return Err(KernelError::Integrity(format!(
            "focus promotion decision {} projection is inconsistent with its immutable request",
            input.decision_id
        )));
    }
    Ok(())
}

fn load_focus_frame_lifecycle(
    connection: &Connection,
    focus_frame_id: &str,
) -> KernelResult<crate::domain::FocusFrameLifecycleSnapshot> {
    connection
        .query_row(
            "SELECT frame_json, status, revision, updated_at, closed_at
             FROM focus_frame_lifecycle WHERE focus_frame_id = ?1",
            [focus_frame_id],
            |row| {
                Ok(crate::domain::FocusFrameLifecycleSnapshot {
                    contract_version: crate::domain::FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
                    frame: parse_json(&row.get::<_, String>(0)?, 0)?,
                    status: parse_json(&row.get::<_, String>(1)?, 1)?,
                    revision: row.get(2)?,
                    updated_at: row.get(3)?,
                    closed_at: row.get(4)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| KernelError::NotFound {
            entity: "FocusFrameLifecycle",
            id: focus_frame_id.into(),
        })
}

fn load_knowledge_entity(
    connection: &Connection,
    conversation_id: &str,
    entity_id: &str,
) -> KernelResult<KnowledgeEntity> {
    let json = connection
        .query_row(
            "SELECT entity_json FROM knowledge_entities
             WHERE conversation_id = ?1 AND id = ?2",
            params![conversation_id, entity_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| KernelError::NotFound {
            entity: "KnowledgeEntity",
            id: entity_id.into(),
        })?;
    let entity: KnowledgeEntity = serde_json::from_str(&json)?;
    entity.validate_for_conversation(conversation_id)?;
    Ok(entity)
}

fn update_knowledge_entity_revision(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    entity: &KnowledgeEntity,
    expected_revision: u64,
) -> KernelResult<()> {
    entity.validate_for_conversation(conversation_id)?;
    let changed = transaction.execute(
        "UPDATE knowledge_entities
         SET entity_json = ?1, revision = ?2, updated_at = ?3
         WHERE id = ?4 AND conversation_id = ?5 AND revision = ?6",
        params![
            serde_json::to_string(entity)?,
            entity.revision,
            entity.updated_at,
            entity.id,
            conversation_id,
            expected_revision,
        ],
    )?;
    if changed != 1 {
        return Err(KernelError::Integrity(format!(
            "knowledge entity {} revision conflict",
            entity.id
        )));
    }
    sync_knowledge_entity_fts(transaction, conversation_id, entity)?;
    sync_knowledge_entity_vector(transaction, conversation_id, entity)?;
    Ok(())
}

fn delete_knowledge_entity_revision(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    entity_id: &str,
    expected_revision: u64,
) -> KernelResult<()> {
    let relation_ids = {
        let mut statement = transaction.prepare(
            "SELECT id, relation_json FROM knowledge_relations WHERE conversation_id = ?1",
        )?;
        let rows = statement.query_map([conversation_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|(id, json)| Ok((id, serde_json::from_str::<KnowledgeRelation>(&json)?)))
            .collect::<KernelResult<Vec<_>>>()?
            .into_iter()
            .filter(|(_, relation)| {
                relation.source_entity_id == entity_id || relation.target_entity_id == entity_id
            })
            .map(|(id, _)| id)
            .collect::<Vec<_>>()
    };
    for relation_id in relation_ids {
        transaction.execute(
            "DELETE FROM knowledge_relations WHERE id = ?1 AND conversation_id = ?2",
            params![relation_id, conversation_id],
        )?;
    }
    transaction.execute(
        "DELETE FROM knowledge_entities_fts WHERE entity_id = ?1 AND conversation_id = ?2",
        params![entity_id, conversation_id],
    )?;
    transaction.execute(
        "DELETE FROM knowledge_vector_records WHERE entity_id = ?1 AND conversation_id = ?2",
        params![entity_id, conversation_id],
    )?;
    let deleted = transaction.execute(
        "DELETE FROM knowledge_entities
         WHERE id = ?1 AND conversation_id = ?2 AND revision = ?3",
        params![entity_id, conversation_id, expected_revision],
    )?;
    if deleted != 1 {
        return Err(KernelError::Integrity(format!(
            "knowledge entity {entity_id} revision conflict"
        )));
    }
    Ok(())
}

fn map_focus_promotion_error(error: crate::domain::FocusPromotionDecisionError) -> KernelError {
    use crate::domain::FocusPromotionDecisionError::{
        StaleEntityRevision, StaleLifecycleRevision, StaleMemoryVersion,
    };
    match error {
        StaleMemoryVersion { .. } | StaleLifecycleRevision { .. } | StaleEntityRevision { .. } => {
            KernelError::Integrity(error.to_string())
        }
        _ => KernelError::Validation(error.to_string()),
    }
}

fn discussion_scope_columns(
    scope: &DiscussionLogScope,
) -> (&str, Option<&str>, Option<&str>, Option<&str>) {
    match scope {
        DiscussionLogScope::Project {
            workspace_id,
            project_id,
        } => (workspace_id, None, Some(project_id), None),
        DiscussionLogScope::Conversation {
            workspace_id,
            conversation_id,
            focus_frame_id,
        } => (
            workspace_id,
            Some(conversation_id),
            None,
            focus_frame_id.as_deref(),
        ),
    }
}

fn list_discussion_logs(
    connection: &Connection,
    scope_column: &str,
    scope_id: &str,
) -> KernelResult<Vec<DiscussionLogProjection>> {
    if scope_id.trim().is_empty() {
        return Err(KernelError::Validation(
            "DiscussionLog scope id must not be empty".into(),
        ));
    }
    let sql = match scope_column {
        "conversation_id" => {
            "SELECT current.projection_json
             FROM discussion_log_revisions current
             JOIN (
               SELECT id, MAX(revision) AS revision
               FROM discussion_log_revisions
               WHERE conversation_id=?1
               GROUP BY id
             ) latest ON latest.id=current.id AND latest.revision=current.revision
             ORDER BY current.updated_at DESC, current.id ASC"
        }
        "project_id" => {
            "SELECT current.projection_json
             FROM discussion_log_revisions current
             JOIN (
               SELECT id, MAX(revision) AS revision
               FROM discussion_log_revisions
               WHERE project_id=?1
               GROUP BY id
             ) latest ON latest.id=current.id AND latest.revision=current.revision
             ORDER BY current.updated_at DESC, current.id ASC"
        }
        _ => {
            return Err(KernelError::Integrity(
                "unsupported DiscussionLog scope column".into(),
            ));
        }
    };
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([scope_id], |row| row.get::<_, String>(0))?;
    rows.map(|row| {
        let projection: DiscussionLogProjection = serde_json::from_str(&row?)?;
        projection.validate()?;
        Ok(projection)
    })
    .collect()
}

fn sync_knowledge_entity_fts(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    entity: &KnowledgeEntity,
) -> KernelResult<()> {
    transaction.execute(
        "DELETE FROM knowledge_entities_fts WHERE entity_id = ?1",
        [&entity.id],
    )?;
    if entity.status == KnowledgeStatus::Confirmed {
        transaction.execute(
            "INSERT INTO knowledge_entities_fts (entity_id, conversation_id, search_text)
             VALUES (?1, ?2, ?3)",
            params![entity.id, conversation_id, knowledge_search_text(entity)],
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct VectorIndexState {
    model_version: String,
    dimensions: usize,
    status: String,
}

fn read_vector_index_state(
    connection: &Connection,
    conversation_id: &str,
) -> KernelResult<Option<VectorIndexState>> {
    let stored = connection
        .query_row(
            "SELECT model_version, dimensions, status
             FROM knowledge_vector_index_states WHERE conversation_id = ?1",
            [conversation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    stored
        .map(|(model_version, dimensions, status)| {
            let dimensions = usize::try_from(dimensions).map_err(|_| {
                KernelError::Integrity("knowledge vector dimensions are invalid".into())
            })?;
            Ok(VectorIndexState {
                model_version,
                dimensions,
                status,
            })
        })
        .transpose()
}

fn write_vector_index_state(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    model_version: &str,
    dimensions: usize,
    status: &str,
) -> KernelResult<()> {
    let dimensions = i64::try_from(dimensions)
        .map_err(|_| KernelError::Integrity("knowledge vector dimensions are too large".into()))?;
    transaction.execute(
        "INSERT INTO knowledge_vector_index_states
         (conversation_id, model_version, dimensions, status, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(conversation_id) DO UPDATE SET
             model_version = excluded.model_version,
             dimensions = excluded.dimensions,
             status = excluded.status,
             updated_at = excluded.updated_at",
        params![
            conversation_id,
            model_version,
            dimensions,
            status,
            now_timestamp()
        ],
    )?;
    Ok(())
}

fn sync_knowledge_entity_vector(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    entity: &KnowledgeEntity,
) -> KernelResult<()> {
    let index_state = match read_vector_index_state(transaction, conversation_id)? {
        Some(state) => Some(state),
        None => transaction
            .query_row(
                "SELECT record_json FROM knowledge_vector_records
                 WHERE conversation_id = ?1 ORDER BY entity_id LIMIT 1",
                [conversation_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .and_then(|json| serde_json::from_str::<EmbeddingRecord>(&json).ok())
            .map(|record| VectorIndexState {
                model_version: record.metadata.model_version,
                dimensions: record.metadata.dimensions,
                status: VECTOR_INDEX_READY.into(),
            }),
    };
    transaction.execute(
        "DELETE FROM knowledge_vector_records WHERE entity_id = ?1",
        [&entity.id],
    )?;
    if let Some(state) = index_state.as_ref().filter(|state| {
        state.model_version != LOCAL_EMBEDDING_MODEL_VERSION
            || state.dimensions != DEFAULT_EMBEDDING_DIMENSIONS
    }) {
        let status = if entity.status == KnowledgeStatus::Confirmed {
            VECTOR_INDEX_STALE
        } else {
            &state.status
        };
        write_vector_index_state(
            transaction,
            conversation_id,
            &state.model_version,
            state.dimensions,
            status,
        )?;
        return Ok(());
    }
    if entity.status == KnowledgeStatus::Confirmed {
        let record = build_knowledge_embedding_record(entity)
            .map_err(|error| KernelError::Integrity(error.to_string()))?;
        transaction.execute(
            "INSERT INTO knowledge_vector_records
             (entity_id, conversation_id, record_json, source_revision, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entity.id,
                conversation_id,
                serde_json::to_string(&record)?,
                entity.revision,
                entity.updated_at
            ],
        )?;
    }
    if index_state.is_some() || entity.status == KnowledgeStatus::Confirmed {
        write_vector_index_state(
            transaction,
            conversation_id,
            LOCAL_EMBEDDING_MODEL_VERSION,
            DEFAULT_EMBEDDING_DIMENSIONS,
            VECTOR_INDEX_READY,
        )?;
    }
    Ok(())
}

fn apply_run_event_to_node(
    transaction: &Transaction<'_>,
    envelope: &ModelRunEventEnvelope,
    run_state: &str,
) -> KernelResult<()> {
    if matches!(envelope.event, ModelRunEvent::UsageUpdated { .. }) {
        return Ok(());
    }
    let (conversation_id, provider_id, model_id, partial_content): (
        String,
        String,
        String,
        String,
    ) = transaction.query_row(
        "SELECT conversation_id, provider_id, model_id, partial_content
             FROM model_runs WHERE id = ?1",
        [&envelope.run_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let terminal = envelope.event.is_terminal();
    let now = &envelope.occurred_at;
    let mut assistant_message_id = None;
    if terminal && !partial_content.is_empty() {
        let existing: Option<String> = transaction.query_row(
            "SELECT assistant_message_id FROM conversation_nodes WHERE id = ?1",
            [&envelope.node_id],
            |row| row.get(0),
        )?;
        if existing.is_none() {
            let message = Message {
                id: new_id("message"),
                conversation_id: conversation_id.clone(),
                node_id: envelope.node_id.clone(),
                role: MessageRole::Assistant,
                content_blocks: vec![ContentBlock::text(partial_content.clone())],
                created_at: now.clone(),
            };
            insert_message(transaction, &message)?;
            assistant_message_id = Some(message.id);
        }
    }
    transaction.execute(
        "UPDATE conversation_nodes
         SET run_state = ?1, provider_id = ?2, model_id = ?3,
             assistant_message_id = COALESCE(?4, assistant_message_id),
             updated_at = ?5, revision = revision + 1
         WHERE id = ?6",
        params![
            run_state,
            provider_id,
            model_id,
            assistant_message_id,
            now,
            envelope.node_id,
        ],
    )?;
    if terminal {
        transaction.execute(
            "UPDATE conversations SET updated_at = ?1, revision = revision + 1 WHERE id = ?2",
            params![now, conversation_id],
        )?;
        append_event(
            transaction,
            "modelRun",
            &envelope.run_id,
            "model_run.terminal_state_applied",
            &serde_json::json!({
                "nodeId": envelope.node_id,
                "state": run_state,
                "partialContentRetained": !partial_content.is_empty(),
            }),
        )?;
    }
    Ok(())
}

fn database_schema_version(connection: &Connection) -> KernelResult<i64> {
    let migrations_exist: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
        [],
        |row| row.get(0),
    )?;
    if !migrations_exist {
        return Ok(0);
    }
    connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

const NODE_SELECT: &str =
    "SELECT n.id, n.conversation_id, n.parent_node_id, n.branch_type, n.title,
            n.provider_id, n.model_id, n.context_snapshot_id, n.run_state,
            n.created_at, n.updated_at, n.revision,
            um.id, um.role, um.content_blocks_json, um.created_at,
            am.id, am.role, am.content_blocks_json, am.created_at
     FROM conversation_nodes n
     JOIN messages um ON um.id = n.user_message_id
     LEFT JOIN messages am ON am.id = n.assistant_message_id";

const NODE_SELECT_BY_ID: &str =
    "SELECT n.id, n.conversation_id, n.parent_node_id, n.branch_type, n.title,
            n.provider_id, n.model_id, n.context_snapshot_id, n.run_state,
            n.created_at, n.updated_at, n.revision,
            um.id, um.role, um.content_blocks_json, um.created_at,
            am.id, am.role, am.content_blocks_json, am.created_at
     FROM conversation_nodes n
     JOIN messages um ON um.id = n.user_message_id
     LEFT JOIN messages am ON am.id = n.assistant_message_id
     WHERE n.id = ?1";

fn query_nodes(
    connection: &Connection,
    conversation_id: &str,
) -> KernelResult<Vec<ConversationNode>> {
    let sql = format!("{NODE_SELECT} WHERE n.conversation_id = ?1 ORDER BY n.created_at");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([conversation_id], row_to_node)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn query_edges(
    connection: &Connection,
    conversation_id: &str,
) -> KernelResult<Vec<ConversationEdge>> {
    let mut statement = connection.prepare(
        "SELECT id, conversation_id, source_node_id, target_node_id, relation, created_at
         FROM conversation_edges WHERE conversation_id = ?1 ORDER BY created_at",
    )?;
    let rows = statement.query_map([conversation_id], |row| {
        Ok(ConversationEdge {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            source_node_id: row.get(2)?,
            target_node_id: row.get(3)?,
            relation: parse_branch(row.get::<_, String>(4)?, 4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn query_positions(
    connection: &Connection,
    conversation_id: &str,
) -> KernelResult<Vec<CanvasNodePosition>> {
    let mut statement = connection.prepare(
        "SELECT node_id, x, y FROM canvas_node_positions
         WHERE conversation_id = ?1 ORDER BY node_id",
    )?;
    let rows = statement.query_map([conversation_id], |row| {
        Ok(CanvasNodePosition {
            node_id: row.get(0)?,
            x: row.get(1)?,
            y: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn insert_snapshot(transaction: &Transaction<'_>, snapshot: &ContextSnapshot) -> KernelResult<()> {
    transaction.execute(
        "INSERT INTO context_snapshots
         (id, conversation_id, parent_node_id, branch_type, current_input,
          selected_messages_json, selected_import_refs_json, explicit_constraints_json,
          omitted_messages_json, system_contract_version, estimated_tokens, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            snapshot.id,
            snapshot.conversation_id,
            snapshot.parent_node_id,
            snapshot.branch_type.as_db(),
            snapshot.current_input,
            serde_json::to_string(&snapshot.selected_messages)?,
            serde_json::to_string(&snapshot.selected_import_refs)?,
            serde_json::to_string(&snapshot.explicit_constraints)?,
            serde_json::to_string(&snapshot.omitted_messages)?,
            snapshot.system_contract_version,
            snapshot.estimated_tokens,
            snapshot.created_at,
        ],
    )?;
    Ok(())
}

fn insert_message(transaction: &Transaction<'_>, message: &Message) -> KernelResult<()> {
    let plain_text = blocks_plain_text(&message.content_blocks);
    let content_blocks_json = serde_json::to_string(&message.content_blocks)?;
    transaction.execute(
        "INSERT INTO messages
         (id, conversation_id, node_id, role, content, created_at, content_blocks_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            message.id,
            message.conversation_id,
            message.node_id,
            message.role.as_db(),
            plain_text,
            message.created_at,
            content_blocks_json,
        ],
    )?;
    Ok(())
}

fn append_event<T: serde::Serialize>(
    transaction: &Transaction<'_>,
    aggregate_type: &str,
    aggregate_id: &str,
    event_type: &str,
    payload: &T,
) -> KernelResult<()> {
    transaction.execute(
        "INSERT INTO domain_events
         (id, aggregate_type, aggregate_id, event_type, payload_json, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new_id("event"),
            aggregate_type,
            aggregate_id,
            event_type,
            serde_json::to_string(payload)?,
            now_timestamp(),
        ],
    )?;
    Ok(())
}

fn parent_first_import_messages(
    messages: &[crate::domain::contracts::ImportedMessage],
) -> Vec<&crate::domain::contracts::ImportedMessage> {
    let by_id: HashMap<&str, &crate::domain::contracts::ImportedMessage> = messages
        .iter()
        .map(|message| (message.id.as_str(), message))
        .collect();
    let mut ordered = Vec::with_capacity(messages.len());
    let mut emitted = HashSet::with_capacity(messages.len());

    fn emit<'a>(
        message: &'a crate::domain::contracts::ImportedMessage,
        by_id: &HashMap<&str, &'a crate::domain::contracts::ImportedMessage>,
        emitted: &mut HashSet<&'a str>,
        ordered: &mut Vec<&'a crate::domain::contracts::ImportedMessage>,
    ) {
        if emitted.contains(message.id.as_str()) {
            return;
        }
        if let Some(parent_id) = message.parent_imported_message_id.as_deref()
            && let Some(parent) = by_id.get(parent_id)
        {
            emit(parent, by_id, emitted, ordered);
        }
        emitted.insert(message.id.as_str());
        ordered.push(message);
    }

    for message in messages {
        emit(message, &by_id, &mut emitted, &mut ordered);
    }
    ordered
}

fn row_to_workspace(row: &Row<'_>) -> rusqlite::Result<Workspace> {
    Ok(Workspace {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn row_to_conversation(row: &Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        title: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        revision: row.get(5)?,
    })
}

fn row_to_node(row: &Row<'_>) -> rusqlite::Result<ConversationNode> {
    let node_id: String = row.get(0)?;
    let conversation_id: String = row.get(1)?;
    let assistant_id: Option<String> = row.get(16)?;
    let assistant_message = if let Some(id) = assistant_id {
        Some(Message {
            id,
            conversation_id: conversation_id.clone(),
            node_id: node_id.clone(),
            role: parse_role(row.get::<_, String>(17)?, 17)?,
            content_blocks: parse_json::<Vec<ContentBlock>>(&row.get::<_, String>(18)?, 18)?,
            created_at: row.get(19)?,
        })
    } else {
        None
    };

    Ok(ConversationNode {
        id: node_id.clone(),
        conversation_id: conversation_id.clone(),
        parent_node_id: row.get(2)?,
        branch_type: parse_branch(row.get::<_, String>(3)?, 3)?,
        title: row.get(4)?,
        provider_id: row.get(5)?,
        model_id: row.get(6)?,
        context_snapshot_id: row.get(7)?,
        run_state: parse_run_state(row.get::<_, String>(8)?, 8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        revision: row.get(11)?,
        user_message: Message {
            id: row.get(12)?,
            conversation_id,
            node_id,
            role: parse_role(row.get::<_, String>(13)?, 13)?,
            content_blocks: parse_json::<Vec<ContentBlock>>(&row.get::<_, String>(14)?, 14)?,
            created_at: row.get(15)?,
        },
        assistant_message,
    })
}

fn row_to_snapshot(row: &Row<'_>) -> rusqlite::Result<ContextSnapshot> {
    let selected_json: String = row.get(5)?;
    let selected_import_json: String = row.get(6)?;
    let constraints_json: String = row.get(7)?;
    let omitted_json: String = row.get(8)?;
    Ok(ContextSnapshot {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        parent_node_id: row.get(2)?,
        branch_type: parse_branch(row.get::<_, String>(3)?, 3)?,
        current_input: row.get(4)?,
        selected_messages: parse_json::<Vec<ContextMessageRef>>(&selected_json, 5)?,
        selected_import_refs: parse_json::<Vec<EvidenceRef>>(&selected_import_json, 6)?,
        explicit_constraints: parse_json::<Vec<ContextConstraint>>(&constraints_json, 7)?,
        omitted_messages: parse_json::<Vec<OmittedContextRef>>(&omitted_json, 8)?,
        system_contract_version: row.get(9)?,
        estimated_tokens: row.get(10)?,
        created_at: row.get(11)?,
    })
}

fn parse_branch(value: String, column: usize) -> rusqlite::Result<BranchType> {
    value.parse().map_err(|error: KernelError| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}

fn parse_run_state(value: String, column: usize) -> rusqlite::Result<RunState> {
    value.parse().map_err(|error: KernelError| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}

fn parse_role(value: String, column: usize) -> rusqlite::Result<MessageRole> {
    value.parse().map_err(|error: KernelError| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(value: &str, column: usize) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}

fn parse_json_value<T: serde::de::DeserializeOwned>(value: &str) -> KernelResult<T> {
    serde_json::from_str(value).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::domain::contracts::{
        FOCUS_CONTRACT_VERSION, FocusBranchKind, FocusContextPolicy, FocusFrame, FocusMemoryScope,
        GeneratorKind, GeneratorRef, ImportPlatform, ImportRevision, ImportRevisionStatus,
        ImportSource, ImportedMessage, KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntityKind,
        KnowledgeRelationKind, KnowledgeScope, ParseReport,
    };

    fn promotion_actor() -> GeneratorRef {
        GeneratorRef {
            kind: GeneratorKind::User,
            generator_id: "mindscape-local-user".into(),
            generator_version: "v1".into(),
        }
    }

    fn promotion_input(
        action: crate::domain::FocusPromotionDecisionAction,
    ) -> FocusPromotionDecisionCommandInput {
        FocusPromotionDecisionCommandInput {
            decision_id: "decision-persisted-1".into(),
            focus_frame_id: "focus-persisted-1".into(),
            candidate_ref: "entity-focus-candidate".into(),
            expected_memory_version: 1,
            expected_lifecycle_revision: 2,
            expected_entity_revision: 3,
            expected_decision_revision: 0,
            action,
            target_scope: None,
            promoted_entity_id: None,
            decided_at: "2026-08-31T02:00:00Z".into(),
        }
    }

    fn insert_closed_promotion_fixture(
        store: &SqliteStore,
    ) -> (Workspace, Conversation, KnowledgeEntity) {
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id.clone(),
                title: "Atomic promotion".into(),
            })
            .expect("conversation");
        let mut lifecycle = focus_lifecycle(&conversation.id);
        lifecycle.frame.memory_scope.promote_refs = vec!["entity-focus-candidate".into()];
        store
            .insert_focus_frame_lifecycle(&lifecycle)
            .expect("insert lifecycle");
        let closed = crate::domain::close_focus_frame(&lifecycle, "2026-08-31T01:00:00Z")
            .expect("close lifecycle");
        store
            .update_focus_frame_lifecycle(&closed, lifecycle.revision)
            .expect("persist closed lifecycle");
        let entity = KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "entity-focus-candidate".into(),
            kind: KnowledgeEntityKind::Decision,
            name: "Keep the verified branch result".into(),
            aliases: vec!["verified result".into()],
            scope: KnowledgeScope::FocusFrame {
                workspace_id: workspace.id.clone(),
                conversation_id: conversation.id.clone(),
                focus_frame_id: lifecycle.frame.id,
            },
            status: KnowledgeStatus::Candidate,
            revision: 3,
            evidence: vec![],
            generator: GeneratorRef {
                kind: GeneratorKind::Model,
                generator_id: "extractor".into(),
                generator_version: "v1".into(),
            },
            created_at: "2026-08-31T00:30:00Z".into(),
            updated_at: "2026-08-31T00:30:00Z".into(),
        };
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("insert candidate");
        (workspace, conversation, entity)
    }

    fn focus_lifecycle(conversation_id: &str) -> crate::domain::FocusFrameLifecycleSnapshot {
        crate::domain::FocusFrameLifecycleSnapshot {
            contract_version: crate::domain::FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
            frame: FocusFrame {
                contract_version: FOCUS_CONTRACT_VERSION.into(),
                id: "focus-persisted-1".into(),
                conversation_id: conversation_id.into(),
                parent_node_id: None,
                objective: "Persist focus".into(),
                active_work_item: Some("SQLite lifecycle".into()),
                context_policy: FocusContextPolicy::FocusNew,
                memory_scope: FocusMemoryScope {
                    branch_kind: FocusBranchKind::Task,
                    inherit_refs: vec![],
                    local_refs: vec![],
                    exclude_refs: vec![],
                    promote_refs: vec![],
                },
                include_refs: vec![],
                exclude_refs: vec![],
                memory_version: 1,
                created_at: now_timestamp(),
            },
            status: crate::domain::FocusFrameLifecycleStatus::Active,
            revision: 1,
            updated_at: now_timestamp(),
            closed_at: None,
        }
    }

    fn knowledge_entity(
        workspace_id: &str,
        conversation_id: &str,
        status: KnowledgeStatus,
    ) -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "entity-fts-backfill".into(),
            kind: KnowledgeEntityKind::Decision,
            name: "SQLite migration backfill".into(),
            aliases: vec!["本地索引迁移".into()],
            scope: KnowledgeScope::Conversation {
                workspace_id: workspace_id.into(),
                conversation_id: conversation_id.into(),
            },
            status,
            revision: 1,
            evidence: vec![],
            generator: GeneratorRef {
                kind: GeneratorKind::User,
                generator_id: "user".into(),
                generator_version: "v1".into(),
            },
            created_at: "2026-08-27T00:00:00Z".into(),
            updated_at: "2026-08-27T00:00:00Z".into(),
        }
    }

    fn semantic_record(entity: &KnowledgeEntity, vector: Vec<f32>) -> EmbeddingRecord {
        EmbeddingRecord {
            id: entity.id.clone(),
            metadata: crate::adapters::provider::EmbeddingMetadata {
                model_version: "semantic-test-v1".into(),
                dimensions: vector.len(),
                source_hash: knowledge_entity_source_hash(entity).expect("entity source hash"),
                chunk_version: KNOWLEDGE_ENTITY_INDEX_VERSION.into(),
            },
            vector,
        }
    }

    #[test]
    fn focus_promotion_confirm_is_atomic_idempotent_and_restart_durable() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("promotion-confirm.sqlite3");
        let store = SqliteStore::open(&database_path).expect("open store");
        let (_, conversation, _) = insert_closed_promotion_fixture(&store);
        let input = promotion_input(crate::domain::FocusPromotionDecisionAction::Confirm);

        let decision = store
            .persist_focus_promotion_decision(&input, &promotion_actor())
            .expect("persist decision");
        let replay = store
            .persist_focus_promotion_decision(&input, &promotion_actor())
            .expect("idempotent replay");
        assert_eq!(replay, decision);
        assert_eq!(
            store
                .list_focus_promotion_decisions(&input.focus_frame_id)
                .expect("list decisions")
                .len(),
            1
        );
        let entity = store
            .get_knowledge_entity(&conversation.id, &input.candidate_ref)
            .expect("confirmed source");
        assert_eq!(entity.status, KnowledgeStatus::Confirmed);
        assert_eq!(entity.revision, 4);
        assert_eq!(
            store
                .search_knowledge_full_text(&conversation.id, "verified", 5)
                .expect("fts")
                .len(),
            1
        );
        assert_eq!(
            store
                .load_knowledge_vector_snapshot(&conversation.id)
                .expect("vector snapshot")
                .records
                .len(),
            1
        );

        drop(store);
        let reopened = SqliteStore::open(&database_path).expect("restart store");
        assert_eq!(
            reopened
                .get_focus_promotion_decision(&input.decision_id)
                .expect("restart decision"),
            decision
        );
    }

    #[test]
    fn stale_focus_promotion_rolls_back_decision_and_derived_indexes() {
        let directory = TempDir::new().expect("temp directory");
        let store = SqliteStore::open(directory.path().join("promotion-stale.sqlite3"))
            .expect("open store");
        let (_, conversation, original) = insert_closed_promotion_fixture(&store);
        let mut stale = promotion_input(crate::domain::FocusPromotionDecisionAction::Confirm);
        stale.expected_entity_revision = 2;

        let error = store
            .persist_focus_promotion_decision(&stale, &promotion_actor())
            .expect_err("stale revision");
        assert!(error.to_string().contains("entity revision is stale"));
        assert_eq!(
            store
                .get_knowledge_entity(&conversation.id, &original.id)
                .expect("unchanged source"),
            original
        );
        assert!(
            store
                .list_focus_promotion_decisions(&stale.focus_frame_id)
                .expect("no decisions")
                .is_empty()
        );
        assert!(
            store
                .search_knowledge_full_text(&conversation.id, "verified", 5)
                .expect("empty fts")
                .is_empty()
        );
    }

    #[test]
    fn focus_promotion_delete_keeps_tombstone_and_removes_relations_and_indexes() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("promotion-delete.sqlite3");
        let store = SqliteStore::open(&database_path).expect("open store");
        let (workspace, conversation, candidate) = insert_closed_promotion_fixture(&store);
        let related = KnowledgeEntity {
            id: "entity-related".into(),
            scope: KnowledgeScope::Conversation {
                workspace_id: workspace.id.clone(),
                conversation_id: conversation.id.clone(),
            },
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: promotion_actor(),
            ..candidate.clone()
        };
        store
            .upsert_knowledge_entity(&conversation.id, &related)
            .expect("related entity");
        store
            .upsert_knowledge_relation(
                &conversation.id,
                &KnowledgeRelation {
                    contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
                    id: "relation-candidate-related".into(),
                    kind: KnowledgeRelationKind::Supports,
                    source_entity_id: candidate.id.clone(),
                    target_entity_id: related.id.clone(),
                    scope: candidate.scope.clone(),
                    status: KnowledgeStatus::Confirmed,
                    revision: 1,
                    evidence: vec![],
                    generator: promotion_actor(),
                    created_at: "2026-08-31T01:30:00Z".into(),
                    updated_at: "2026-08-31T01:30:00Z".into(),
                },
            )
            .expect("relation");
        let input = promotion_input(crate::domain::FocusPromotionDecisionAction::Delete);
        let decision = store
            .persist_focus_promotion_decision(&input, &promotion_actor())
            .expect("delete decision");
        assert_eq!(decision.source_entity_revision, None);
        assert!(matches!(
            store.get_knowledge_entity(&conversation.id, &candidate.id),
            Err(KernelError::NotFound { .. })
        ));
        assert!(
            store
                .list_knowledge_relations(&conversation.id)
                .expect("relations")
                .is_empty()
        );

        drop(store);
        let reopened = SqliteStore::open(&database_path).expect("restart store");
        assert_eq!(
            reopened
                .get_focus_promotion_decision(&input.decision_id)
                .expect("tombstone"),
            decision
        );
    }

    #[test]
    fn focus_promotion_promote_creates_a_distinct_confirmed_target_atomically() {
        let directory = TempDir::new().expect("temp directory");
        let store = SqliteStore::open(directory.path().join("promotion-promote.sqlite3"))
            .expect("open store");
        let (workspace, conversation, _) = insert_closed_promotion_fixture(&store);
        let mut input = promotion_input(crate::domain::FocusPromotionDecisionAction::Promote);
        input.target_scope = Some(crate::domain::FocusPromotionTargetScope::Project {
            workspace_id: workspace.id,
            project_id: "project-atomic".into(),
        });
        input.promoted_entity_id = Some("entity-promoted-result".into());

        let decision = store
            .persist_focus_promotion_decision(&input, &promotion_actor())
            .expect("promote decision");
        let source = store
            .get_knowledge_entity(&conversation.id, &input.candidate_ref)
            .expect("source");
        let promoted = store
            .get_knowledge_entity(&conversation.id, "entity-promoted-result")
            .expect("promoted target");
        assert_eq!(source.status, KnowledgeStatus::Confirmed);
        assert!(matches!(source.scope, KnowledgeScope::FocusFrame { .. }));
        assert_eq!(promoted.status, KnowledgeStatus::Confirmed);
        assert!(matches!(
            promoted.scope,
            KnowledgeScope::Project { ref project_id, .. } if project_id == "project-atomic"
        ));
        assert_eq!(promoted.evidence, source.evidence);
        assert_eq!(
            decision.promoted_entity_id.as_deref(),
            Some(promoted.id.as_str())
        );
        assert_eq!(
            store
                .search_knowledge_full_text(&conversation.id, "verified", 5)
                .expect("fts")
                .len(),
            2
        );
        assert_eq!(
            store
                .load_knowledge_vector_snapshot(&conversation.id)
                .expect("vectors")
                .records
                .len(),
            2
        );

        let mut duplicate = input;
        duplicate.decision_id = "decision-persisted-2".into();
        let error = store
            .persist_focus_promotion_decision(&duplicate, &promotion_actor())
            .expect_err("one decision per candidate");
        assert!(error.to_string().contains("already decided"));
    }

    #[test]
    fn focus_promotion_reject_removes_candidate_from_fts_and_vector() {
        let directory = TempDir::new().expect("temp directory");
        let store = SqliteStore::open(directory.path().join("promotion-reject.sqlite3"))
            .expect("open store");
        let (_, conversation, _) = insert_closed_promotion_fixture(&store);
        let input = promotion_input(crate::domain::FocusPromotionDecisionAction::Reject);

        store
            .persist_focus_promotion_decision(&input, &promotion_actor())
            .expect("reject decision");
        let source = store
            .get_knowledge_entity(&conversation.id, &input.candidate_ref)
            .expect("rejected source");
        assert_eq!(source.status, KnowledgeStatus::Rejected);
        assert!(
            store
                .search_knowledge_full_text(&conversation.id, "verified", 5)
                .expect("fts")
                .is_empty()
        );
        assert!(
            store
                .load_knowledge_vector_snapshot(&conversation.id)
                .expect("vectors")
                .records
                .is_empty()
        );
    }

    #[test]
    fn confirmed_entity_update_marks_semantic_index_stale_without_hash_downgrade() {
        let directory = TempDir::new().expect("temp directory");
        let store =
            SqliteStore::open(directory.path().join("semantic-stale.sqlite3")).expect("open store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id.clone(),
                title: "Semantic invalidation".into(),
            })
            .expect("conversation");
        let mut entity =
            knowledge_entity(&workspace.id, &conversation.id, KnowledgeStatus::Confirmed);
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("upsert entity");
        store
            .replace_knowledge_vector_records(
                &conversation.id,
                "semantic-test-v1",
                3,
                &[semantic_record(&entity, vec![1.0, 0.0, 0.0])],
            )
            .expect("persist semantic index");

        entity.revision = 2;
        entity.name = "Changed semantic source".into();
        entity.updated_at = "2026-08-30T18:00:00Z".into();
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("update entity");

        let snapshot = store
            .load_knowledge_vector_snapshot(&conversation.id)
            .expect("load invalidated snapshot");
        assert_eq!(snapshot.availability, RetrievalAvailability::Unavailable);
        assert!(snapshot.records.is_empty());
        let connection = store.connection().expect("connection");
        let state = read_vector_index_state(&connection, &conversation.id)
            .expect("read index state")
            .expect("persisted index state");
        assert_eq!(state.model_version, "semantic-test-v1");
        assert_eq!(state.status, VECTOR_INDEX_STALE);
    }

    #[test]
    fn semantic_rebuild_rejects_stale_source_and_survives_restart() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("semantic-rebuild.sqlite3");
        let store = SqliteStore::open(&database_path).expect("open store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id.clone(),
                title: "Semantic rebuild".into(),
            })
            .expect("conversation");
        let mut entity =
            knowledge_entity(&workspace.id, &conversation.id, KnowledgeStatus::Confirmed);
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("upsert entity");
        let stale_record = semantic_record(&entity, vec![1.0, 0.0, 0.0]);
        entity.revision = 2;
        entity.name = "Current semantic source".into();
        entity.updated_at = "2026-08-30T18:01:00Z".into();
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("update source");

        let error = store
            .replace_knowledge_vector_records(
                &conversation.id,
                "semantic-test-v1",
                3,
                &[stale_record],
            )
            .expect_err("reject stale semantic source");
        assert!(error.to_string().contains("active index contract"));
        let current_record = semantic_record(&entity, vec![0.0, 1.0, 0.0]);
        store
            .replace_knowledge_vector_records(
                &conversation.id,
                "semantic-test-v1",
                3,
                std::slice::from_ref(&current_record),
            )
            .expect("persist current semantic source");
        drop(store);

        let reopened = SqliteStore::open(&database_path).expect("reopen store");
        let snapshot = reopened
            .load_knowledge_vector_snapshot(&conversation.id)
            .expect("restore semantic snapshot");
        assert_eq!(snapshot.availability, RetrievalAvailability::Available);
        assert_eq!(snapshot.records, vec![current_record]);
        let projection = crate::adapters::provider::retrieve_validated_knowledge_with_semantic(
            &conversation.id,
            "non-literal semantic query",
            1,
            std::slice::from_ref(&entity),
            &[],
            vec![],
            snapshot,
            Some(&crate::adapters::provider::SemanticQueryEmbedding {
                model_version: "semantic-test-v1".into(),
                dimensions: 3,
                vector: vec![0.0, 1.0, 0.0],
            }),
        )
        .expect("query restored semantic index");
        assert_eq!(projection.candidates[0].entity.id, entity.id);
        assert_eq!(
            projection.candidates[0]
                .embedding
                .as_ref()
                .expect("semantic provenance")
                .model_version,
            "semantic-test-v1"
        );
    }

    #[test]
    fn rejecting_entity_removes_semantic_record_without_staling_remaining_index() {
        let directory = TempDir::new().expect("temp directory");
        let store = SqliteStore::open(directory.path().join("semantic-reject.sqlite3"))
            .expect("open store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id.clone(),
                title: "Semantic rejection".into(),
            })
            .expect("conversation");
        let mut entity =
            knowledge_entity(&workspace.id, &conversation.id, KnowledgeStatus::Confirmed);
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("upsert entity");
        store
            .replace_knowledge_vector_records(
                &conversation.id,
                "semantic-test-v1",
                3,
                &[semantic_record(&entity, vec![1.0, 0.0, 0.0])],
            )
            .expect("persist semantic index");

        entity.status = KnowledgeStatus::Rejected;
        entity.revision = 2;
        entity.updated_at = "2026-08-30T18:02:00Z".into();
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("reject entity");

        let snapshot = store
            .load_knowledge_vector_snapshot(&conversation.id)
            .expect("load index after rejection");
        assert_eq!(snapshot.availability, RetrievalAvailability::Available);
        assert!(snapshot.records.is_empty());
    }

    #[test]
    fn schema_v10_backfills_confirmed_entities_into_full_text_index() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("fts-backfill.sqlite3");
        let store = SqliteStore::open(&database_path).expect("open store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id.clone(),
                title: "FTS migration".into(),
            })
            .expect("conversation");
        store
            .upsert_knowledge_entity(
                &conversation.id,
                &knowledge_entity(&workspace.id, &conversation.id, KnowledgeStatus::Confirmed),
            )
            .expect("upsert entity");
        drop(store);

        let connection = Connection::open(&database_path).expect("open v10 database");
        connection
            .execute_batch(
                "DROP TABLE knowledge_vector_records;
                 DROP TABLE markdown_projections;
                 DROP TABLE knowledge_entities_fts;
                 DELETE FROM schema_migrations WHERE version >= 10;",
            )
            .expect("simulate v9 database");
        drop(connection);

        let migrated = SqliteStore::open(&database_path).expect("migrate to v10");
        let matches = migrated
            .search_knowledge_full_text(&conversation.id, "本地索引", 10)
            .expect("search backfilled index");
        assert_eq!(matches[0].id, "entity-fts-backfill");
    }

    #[test]
    fn markdown_revision_conflict_rolls_back_entity_and_derived_indexes() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("markdown-atomic.sqlite3");
        let store = SqliteStore::open(&database_path).expect("store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id.clone(),
                title: "Markdown atomic".into(),
            })
            .expect("conversation");
        let mut entity =
            knowledge_entity(&workspace.id, &conversation.id, KnowledgeStatus::Confirmed);
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("entity");
        let initial = crate::domain::contracts::MarkdownProjection {
            contract_version: crate::domain::contracts::MARKDOWN_PROJECTION_CONTRACT_VERSION.into(),
            id: "markdown-entity-fts-backfill".into(),
            target_entity_id: entity.id.clone(),
            relative_path: "entities/entity-fts-backfill.md".into(),
            entity_revision: 1,
            projection_revision: 1,
            content_hash: "hash-1".into(),
            frontmatter_version: "mindscape.frontmatter.v1".into(),
            created_at: "2026-08-29T00:00:00Z".into(),
        };
        store
            .persist_markdown_projection(&initial)
            .expect("initial projection");
        let initial_vectors = store
            .load_knowledge_vector_snapshot(&conversation.id)
            .expect("initial vector snapshot");
        entity.revision = 2;
        entity.name = "Changed but rolled back".into();
        let conflicting = crate::domain::contracts::MarkdownProjection {
            entity_revision: 2,
            content_hash: "hash-2".into(),
            ..initial.clone()
        };
        store
            .persist_markdown_entity_revision(&conversation.id, &entity, &conflicting)
            .expect_err("projection conflict");
        let restored = store
            .get_knowledge_entity(&conversation.id, &entity.id)
            .expect("restored entity");
        assert_eq!(restored.revision, 1);
        assert_eq!(restored.name, "SQLite migration backfill");
        assert!(
            store
                .search_knowledge_full_text(&conversation.id, "Changed", 10)
                .expect("search rolled-back name")
                .is_empty()
        );
        let vectors = store
            .load_knowledge_vector_snapshot(&conversation.id)
            .expect("load rolled-back vector");
        assert_eq!(vectors.records, initial_vectors.records);
        assert_eq!(
            store
                .list_markdown_projections(&entity.id)
                .expect("load rolled-back projections"),
            vec![initial.clone()]
        );

        drop(store);
        let reopened = SqliteStore::open(&database_path).expect("reopen store");
        assert_eq!(
            reopened
                .get_knowledge_entity(&conversation.id, &entity.id)
                .expect("entity after restart")
                .revision,
            1
        );
        assert_eq!(
            reopened
                .load_knowledge_vector_snapshot(&conversation.id)
                .expect("vector after restart")
                .records,
            initial_vectors.records
        );
        assert_eq!(
            reopened
                .list_markdown_projections(&entity.id)
                .expect("projections after restart"),
            vec![initial]
        );
    }

    #[test]
    fn schema_v12_backfills_and_status_changes_invalidate_vector_records() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("vector-backfill.sqlite3");
        let store = SqliteStore::open(&database_path).expect("open store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id.clone(),
                title: "Vector migration".into(),
            })
            .expect("conversation");
        let mut entity =
            knowledge_entity(&workspace.id, &conversation.id, KnowledgeStatus::Confirmed);
        store
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("upsert entity");
        drop(store);

        let connection = Connection::open(&database_path).expect("open v12 database");
        connection
            .execute_batch(
                "DROP TABLE knowledge_vector_records;
                 DELETE FROM schema_migrations WHERE version >= 12;",
            )
            .expect("simulate v11 database");
        drop(connection);

        let migrated = SqliteStore::open(&database_path).expect("migrate to v12");
        let snapshot = migrated
            .load_knowledge_vector_snapshot(&conversation.id)
            .expect("load backfilled vectors");
        assert_eq!(snapshot.availability, RetrievalAvailability::Available);
        assert_eq!(snapshot.records.len(), 1);
        assert_eq!(snapshot.records[0].id, entity.id);

        entity.status = KnowledgeStatus::Rejected;
        entity.revision = 2;
        entity.updated_at = "2026-08-27T00:01:00Z".into();
        migrated
            .upsert_knowledge_entity(&conversation.id, &entity)
            .expect("reject entity");
        assert!(
            migrated
                .load_knowledge_vector_snapshot(&conversation.id)
                .expect("load invalidated vectors")
                .records
                .is_empty()
        );
    }

    #[test]
    fn focus_lifecycle_survives_restart_and_rejects_stale_revision() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("focus.sqlite3");
        let store = SqliteStore::open(&database_path).expect("store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id,
                title: "Focus lifecycle".into(),
            })
            .expect("conversation");
        let active = focus_lifecycle(&conversation.id);
        store
            .insert_focus_frame_lifecycle(&active)
            .expect("insert lifecycle");
        let listed = store
            .list_focus_frame_lifecycles(&conversation.id)
            .expect("list lifecycle");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].frame.id, active.frame.id);
        assert!(
            store
                .list_focus_frame_lifecycles("conversation-without-focus")
                .expect("list empty lifecycle")
                .is_empty()
        );
        let closed =
            crate::domain::close_focus_frame(&active, "2026-08-25T04:00:00Z").expect("close focus");
        store
            .update_focus_frame_lifecycle(&closed, active.revision)
            .expect("persist close");
        assert!(
            store
                .update_focus_frame_lifecycle(&closed, active.revision)
                .is_err()
        );
        drop(store);

        let reopened_store = SqliteStore::open(&database_path).expect("reopen store");
        let restored = reopened_store
            .get_focus_frame_lifecycle(&active.frame.id)
            .expect("restore lifecycle");
        assert_eq!(restored, closed);
    }

    fn import_bundle(
        conversation_id: &str,
    ) -> (
        ImportSource,
        ImportRevision,
        Vec<ImportedMessage>,
        ParseReport,
    ) {
        let source = ImportSource {
            id: "import-source-1".into(),
            conversation_id: conversation_id.into(),
            platform: ImportPlatform::Generic,
            original_file_name: Some("conversation.md".into()),
            content_hash: "sha256:import-source-1".into(),
            storage_ref: "aa/import-source-1".into(),
            created_at: now_timestamp(),
        };
        let revision = ImportRevision {
            id: "import-revision-1".into(),
            import_source_id: source.id.clone(),
            adapter_id: "generic-markdown".into(),
            adapter_version: "1".into(),
            status: ImportRevisionStatus::Parsed,
            created_at: now_timestamp(),
        };
        let messages = vec![
            ImportedMessage {
                id: "import-message-child".into(),
                import_revision_id: revision.id.clone(),
                role: MessageRole::Imported,
                content_blocks: vec![ContentBlock::text("child")],
                occurred_at: None,
                source_locator: "$.messages[1]".into(),
                parent_imported_message_id: Some("import-message-parent".into()),
                platform_extension: json!({}),
            },
            ImportedMessage {
                id: "import-message-parent".into(),
                import_revision_id: revision.id.clone(),
                role: MessageRole::Imported,
                content_blocks: vec![ContentBlock::text("parent")],
                occurred_at: None,
                source_locator: "$.messages[0]".into(),
                parent_imported_message_id: None,
                platform_extension: json!({}),
            },
        ];
        let report = ParseReport {
            import_revision_id: revision.id.clone(),
            conversation_count: 1,
            message_count: 2,
            attachment_count: 0,
            tool_record_count: 0,
            field_recovery: vec![],
            warnings: vec![],
            errors: vec![],
        };
        (source, revision, messages, report)
    }

    #[test]
    fn import_bundle_persists_children_after_their_parents() {
        let directory = TempDir::new().expect("temp directory");
        let store = SqliteStore::open(directory.path().join("imports.sqlite3")).expect("store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id,
                title: "Imported conversation".into(),
            })
            .expect("conversation");
        let (source, revision, messages, report) = import_bundle(&conversation.id);

        store
            .persist_import_bundle(&source, &revision, &messages, &report)
            .expect("persist valid bundle");

        let connection = store.connection().expect("connection");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM imported_messages", [], |row| {
                row.get(0)
            })
            .expect("message count");
        assert_eq!(count, 2);
        let sources = store
            .list_import_sources(&conversation.id)
            .expect("list import sources");
        assert_eq!(sources, vec![source.clone()]);
        let restored = store
            .get_import_bundle(&source.id)
            .expect("restore import bundle");
        assert_eq!(restored.source, source);
        assert_eq!(restored.revision, revision);
        assert_eq!(restored.report, report);
        assert_eq!(restored.messages.len(), 2);
    }

    #[test]
    fn duplicate_content_hash_rolls_back_the_entire_second_bundle() {
        let directory = TempDir::new().expect("temp directory");
        let store = SqliteStore::open(directory.path().join("imports.sqlite3")).expect("store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id,
                title: "Imported conversation".into(),
            })
            .expect("conversation");
        let (source, revision, messages, report) = import_bundle(&conversation.id);
        store
            .persist_import_bundle(&source, &revision, &messages, &report)
            .expect("first bundle");

        let mut duplicate_source = source.clone();
        duplicate_source.id = "import-source-2".into();
        let mut duplicate_revision = revision.clone();
        duplicate_revision.id = "import-revision-2".into();
        duplicate_revision.import_source_id = duplicate_source.id.clone();
        let duplicate_messages = messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                let mut message = message.clone();
                message.id = format!("duplicate-message-{index}");
                message.import_revision_id = duplicate_revision.id.clone();
                message.parent_imported_message_id = None;
                message
            })
            .collect::<Vec<_>>();
        let mut duplicate_report = report.clone();
        duplicate_report.import_revision_id = duplicate_revision.id.clone();

        assert!(
            store
                .persist_import_bundle(
                    &duplicate_source,
                    &duplicate_revision,
                    &duplicate_messages,
                    &duplicate_report,
                )
                .is_err()
        );
        let connection = store.connection().expect("connection");
        let revision_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM import_revisions WHERE id = 'import-revision-2'",
                [],
                |row| row.get(0),
            )
            .expect("revision count");
        assert_eq!(revision_count, 0);
    }

    fn store_with_pending_node() -> (TempDir, SqliteStore, ModelRunRequest) {
        let directory = TempDir::new().expect("temp directory");
        let store = SqliteStore::open(directory.path().join("runs.sqlite3")).expect("open store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id,
                title: "Run persistence".into(),
            })
            .expect("conversation");
        let input = AppendTurnInput {
            conversation_id: conversation.id.clone(),
            parent_node_id: None,
            branch_type: BranchType::Continues,
            title: "Question".into(),
            prompt: "Persist this run".into(),
            provider_id: Some("mock".into()),
            model_id: Some("mock-stream-v1".into()),
        };
        let snapshot = crate::domain::compile_context(crate::domain::ContextCompileInput {
            conversation_id: conversation.id.clone(),
            parent_node_id: None,
            branch_type: BranchType::Continues,
            current_input: input.prompt.clone(),
            path: vec![],
            max_context_tokens: None,
        })
        .expect("compile context");
        let node = store.insert_turn(&input, &snapshot).expect("pending node");
        let request = ModelRunRequest {
            contract_version: "mindscape.runtime.v1".into(),
            run_id: "run-persistence-1".into(),
            conversation_id: conversation.id,
            node_id: node.id,
            context_snapshot: snapshot,
            provider_id: "mock".into(),
            model_id: "mock-stream-v1".into(),
            capabilities: vec![],
            budget: crate::domain::contracts::ModelRunBudget {
                max_output_tokens: None,
                max_cost_microunits: None,
                timeout_ms: 30_000,
            },
            effective_run_profile: None,
            idempotency_key: "run-persistence-key-1".into(),
            created_at: now_timestamp(),
        };
        (directory, store, request)
    }

    #[test]
    fn migrates_legacy_message_text_into_content_blocks() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("legacy.sqlite3");
        let connection = Connection::open(&database_path).expect("open legacy database");
        connection
            .execute_batch(MIGRATION_V1)
            .expect("apply legacy migration");
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
                [now_timestamp()],
            )
            .expect("record legacy version");
        let now = now_timestamp();
        connection
            .execute(
                "INSERT INTO workspaces (id, name, created_at, updated_at)
                 VALUES ('workspace-1', 'legacy workspace', ?1, ?1)",
                [&now],
            )
            .expect("insert legacy workspace");
        connection
            .execute(
                "INSERT INTO conversations
                 (id, workspace_id, title, created_at, updated_at, revision)
                 VALUES ('conversation-1', 'workspace-1', 'legacy conversation', ?1, ?1, 1)",
                [&now],
            )
            .expect("insert legacy conversation");
        connection
            .execute(
                "INSERT INTO context_snapshots
                 (id, conversation_id, parent_node_id, branch_type, current_input,
                  selected_messages_json, omitted_messages_json, system_contract_version,
                  estimated_tokens, created_at)
                 VALUES ('snapshot-1', 'conversation-1', NULL, 'reframing', 'legacy text',
                         '[]', '[]', 'legacy.v1', 3, ?1)",
                [&now],
            )
            .expect("insert legacy snapshot");
        connection
            .execute(
                "INSERT INTO conversation_nodes
                 (id, conversation_id, parent_node_id, branch_type, title,
                  user_message_id, assistant_message_id, provider_id, model_id,
                  context_snapshot_id, run_state, created_at, updated_at, revision)
                 VALUES ('node-1', 'conversation-1', NULL, 'reframing', 'legacy node',
                         'message-1', NULL, NULL, NULL, 'snapshot-1', 'pending', ?1, ?1, 1)",
                [&now],
            )
            .expect("insert legacy node");
        connection
            .execute(
                "INSERT INTO messages
                 (id, conversation_id, node_id, role, content, created_at)
                 VALUES ('message-1', 'conversation-1', 'node-1', 'user', 'legacy text', ?1)",
                [&now],
            )
            .expect("insert legacy message");
        drop(connection);

        let store = SqliteStore::open(&database_path).expect("migrate database");
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        let backups = std::fs::read_dir(store.backup_dir())
            .expect("read backup directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect backups");
        assert_eq!(backups.len(), 1);
        assert!(
            backups[0]
                .file_name()
                .to_string_lossy()
                .starts_with("mindscape-schema-v1-")
        );
        let backup = Connection::open(backups[0].path()).expect("open migration backup");
        assert_eq!(database_schema_version(&backup).unwrap(), 1);
        let connection = store.connection().expect("open migrated database");
        let json: String = connection
            .query_row(
                "SELECT content_blocks_json FROM messages WHERE id = 'message-1'",
                [],
                |row| row.get(0),
            )
            .expect("read migrated blocks");
        let blocks: Vec<ContentBlock> = serde_json::from_str(&json).expect("parse blocks");
        assert_eq!(blocks, vec![ContentBlock::text("legacy text")]);
    }

    #[test]
    fn new_database_does_not_create_a_migration_backup() {
        let directory = TempDir::new().expect("temp directory");
        let store =
            SqliteStore::open(directory.path().join("new.sqlite3")).expect("create new database");

        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(
            std::fs::read_dir(store.backup_dir())
                .expect("read backup directory")
                .count(),
            0
        );
    }

    #[test]
    fn rejects_database_from_a_newer_application_version() {
        let directory = TempDir::new().expect("temp directory");
        let database_path = directory.path().join("future.sqlite3");
        let connection = Connection::open(&database_path).expect("open future database");
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
                 INSERT INTO schema_migrations (version, applied_at) VALUES (999, 'future');",
            )
            .expect("create future schema marker");
        drop(connection);

        let error = SqliteStore::open(&database_path).expect_err("reject future database");
        assert!(error.to_string().contains("newer than supported"));
    }

    #[test]
    fn every_connection_enables_integrity_pragmas() {
        let directory = TempDir::new().expect("temp directory");
        let store =
            SqliteStore::open(directory.path().join("pragmas.sqlite3")).expect("create database");
        let connection = store.connection().expect("open configured connection");

        let foreign_keys: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("read foreign key setting");
        let busy_timeout: i64 = connection
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .expect("read busy timeout");
        assert_eq!(foreign_keys, 1);
        assert_eq!(busy_timeout, 5_000);
    }

    #[test]
    fn model_run_events_are_atomic_ordered_and_idempotent() {
        let (_directory, store, request) = store_with_pending_node();
        store.create_model_run(&request).expect("create model run");
        store
            .create_model_run(&request)
            .expect("idempotent create model run");
        assert_eq!(
            store
                .model_run_request_by_idempotency_key(&request.idempotency_key)
                .expect("load persisted request"),
            Some(request.clone()),
            "schema v4 must preserve the complete frozen ModelRunRequest"
        );
        let started = ModelRunEventEnvelope {
            contract_version: request.contract_version.clone(),
            event_id: "event-started".into(),
            run_id: request.run_id.clone(),
            node_id: request.node_id.clone(),
            sequence: 1,
            occurred_at: now_timestamp(),
            event: ModelRunEvent::Started,
        };
        let delta = ModelRunEventEnvelope {
            event_id: "event-delta".into(),
            sequence: 2,
            event: ModelRunEvent::TextDelta {
                delta: "partial answer".into(),
            },
            ..started.clone()
        };

        store
            .record_model_run_event(&started)
            .expect("record started");
        store
            .record_model_run_event(&started)
            .expect("idempotent event replay");
        store.record_model_run_event(&delta).expect("record delta");

        let connection = store.connection().expect("connection");
        let (state, sequence, partial, event_count): (String, i64, String, i64) = connection
            .query_row(
                "SELECT r.state, r.last_sequence, r.partial_content,
                        (SELECT COUNT(*) FROM model_run_events e WHERE e.run_id = r.id)
                 FROM model_runs r WHERE r.id = ?1",
                [&request.run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read persisted run");
        assert_eq!(state, "streaming");
        assert_eq!(sequence, 2);
        assert_eq!(partial, "partial answer");
        assert_eq!(event_count, 2);

        let completed = ModelRunEventEnvelope {
            event_id: "event-completed".into(),
            sequence: 3,
            event: ModelRunEvent::Completed {
                finish_reason: crate::domain::contracts::FinishReason::Stop,
                usage: crate::domain::contracts::ModelUsage::default(),
            },
            ..started.clone()
        };
        store
            .record_model_run_event(&completed)
            .expect("record completed");
        store
            .record_model_run_event(&completed)
            .expect("idempotent completed replay");
        let node = store.load_node(&request.node_id).expect("completed node");
        assert_eq!(node.run_state, RunState::Completed);
        assert_eq!(
            node.assistant_message
                .as_ref()
                .map(|message| blocks_plain_text(&message.content_blocks)),
            Some("partial answer".into())
        );

        let skipped = ModelRunEventEnvelope {
            event_id: "event-skipped".into(),
            sequence: 5,
            ..started
        };
        assert!(store.record_model_run_event(&skipped).is_err());
    }

    #[test]
    fn startup_recovery_fails_incomplete_run_and_retains_partial_output() {
        let (_directory, store, request) = store_with_pending_node();
        store.create_model_run(&request).expect("create model run");
        for envelope in [
            ModelRunEventEnvelope {
                contract_version: request.contract_version.clone(),
                event_id: "recovery-started".into(),
                run_id: request.run_id.clone(),
                node_id: request.node_id.clone(),
                sequence: 1,
                occurred_at: now_timestamp(),
                event: ModelRunEvent::Started,
            },
            ModelRunEventEnvelope {
                contract_version: request.contract_version.clone(),
                event_id: "recovery-delta".into(),
                run_id: request.run_id.clone(),
                node_id: request.node_id.clone(),
                sequence: 2,
                occurred_at: now_timestamp(),
                event: ModelRunEvent::TextDelta {
                    delta: "retained partial".into(),
                },
            },
        ] {
            store
                .record_model_run_event(&envelope)
                .expect("persist pre-crash event");
        }

        assert_eq!(store.recover_interrupted_runs().unwrap(), 1);
        assert_eq!(store.recover_interrupted_runs().unwrap(), 0);

        let run = store.list_model_runs(None).unwrap().remove(0);
        assert_eq!(run.state, RunState::Failed);
        assert_eq!(run.partial_content, "retained partial");
        assert!(matches!(
            run.terminal_event,
            Some(ModelRunEvent::Failed { .. })
        ));
        let node = store.load_node(&request.node_id).expect("recovered node");
        assert_eq!(node.run_state, RunState::Failed);
        assert_eq!(
            node.assistant_message
                .as_ref()
                .map(|message| blocks_plain_text(&message.content_blocks)),
            Some("retained partial".into())
        );
    }

    #[test]
    fn canvas_viewport_round_trips_and_updates_per_conversation() {
        let directory = TempDir::new().expect("temp directory");
        let store =
            SqliteStore::open(directory.path().join("viewport.sqlite3")).expect("open store");
        let workspace = store.ensure_default_workspace().expect("workspace");
        let conversation = store
            .create_conversation(&CreateConversationInput {
                workspace_id: workspace.id,
                title: "Viewport".into(),
            })
            .expect("conversation");
        assert!(
            store
                .get_canvas_viewport(&conversation.id)
                .unwrap()
                .is_none()
        );

        for (x, y, zoom) in [(10.0, 20.0, 0.8), (-42.0, 15.5, 1.2)] {
            store
                .save_canvas_viewport(&SaveCanvasViewportInput {
                    conversation_id: conversation.id.clone(),
                    x,
                    y,
                    zoom,
                })
                .expect("save viewport");
        }
        let viewport = store
            .get_canvas_viewport(&conversation.id)
            .unwrap()
            .expect("persisted viewport");
        assert_eq!((viewport.x, viewport.y, viewport.zoom), (-42.0, 15.5, 1.2));

        let invalid = store.save_canvas_viewport(&SaveCanvasViewportInput {
            conversation_id: conversation.id,
            x: 0.0,
            y: 0.0,
            zoom: 0.0,
        });
        assert!(invalid.is_err());
    }
}

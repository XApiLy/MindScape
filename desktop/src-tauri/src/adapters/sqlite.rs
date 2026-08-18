use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, types::Type};

use crate::domain::{
    AppendTurnInput, BranchType, CanvasNodePosition, CanvasViewportState, CompleteTurnInput,
    ContentBlock, ContextConstraint, ContextMessageRef, ContextSnapshot, ContextTurn, Conversation,
    ConversationEdge, ConversationGraph, ConversationNode, ConversationSummary,
    CreateConversationInput, KernelError, KernelResult, Message, MessageRole, OmittedContextRef,
    RunState, SCHEMA_VERSION, SaveCanvasViewportInput, UpdateNodePositionInput, Workspace,
    blocks_plain_text,
    contracts::{
        EvidenceRef, ModelRunEvent, ModelRunEventEnvelope, ModelRunProjection, ModelRunRequest,
    },
    new_id, now_timestamp,
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

        debug_assert_eq!(SCHEMA_VERSION, 5);
        Ok(())
    }
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
    use tempfile::TempDir;

    use super::*;

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

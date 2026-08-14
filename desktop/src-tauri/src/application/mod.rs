use crate::{
    adapters::SqliteStore,
    domain::{
        AppendTurnInput, CompleteTurnInput, ContextCompileInput, ContextSnapshot, Conversation,
        ConversationGraph, ConversationNode, CreateConversationInput, KernelBootstrap,
        KernelResult, UpdateNodePositionInput, compile_context,
        contracts::{ModelRunEventEnvelope, ModelRunRequest},
    },
};

#[derive(Debug, Clone)]
pub struct KernelService {
    store: SqliteStore,
}

impl KernelService {
    #[cfg(test)]
    pub fn open(database_path: impl AsRef<std::path::Path>) -> KernelResult<Self> {
        Ok(Self {
            store: SqliteStore::open(database_path)?,
        })
    }

    pub fn open_with_backup_dir(
        database_path: impl AsRef<std::path::Path>,
        backup_dir: impl AsRef<std::path::Path>,
    ) -> KernelResult<Self> {
        Ok(Self {
            store: SqliteStore::open_with_backup_dir(database_path, backup_dir)?,
        })
    }

    pub fn bootstrap(&self) -> KernelResult<KernelBootstrap> {
        let workspace = self.store.ensure_default_workspace()?;
        let conversations = self.store.list_conversations(&workspace.id)?;
        Ok(KernelBootstrap {
            schema_version: self.store.schema_version()?,
            database_path: self.store.database_path().display().to_string(),
            workspace,
            conversations,
        })
    }

    pub fn create_conversation(
        &self,
        input: CreateConversationInput,
    ) -> KernelResult<Conversation> {
        input.validate()?;
        self.store.create_conversation(&input)
    }

    pub fn load_conversation_graph(
        &self,
        conversation_id: &str,
    ) -> KernelResult<ConversationGraph> {
        self.store.load_conversation_graph(conversation_id)
    }

    pub fn append_turn(&self, input: AppendTurnInput) -> KernelResult<ConversationNode> {
        input.validate()?;
        let path = self
            .store
            .path_to_node(&input.conversation_id, input.parent_node_id.as_deref())?;
        let snapshot = compile_context(ContextCompileInput {
            conversation_id: input.conversation_id.clone(),
            parent_node_id: input.parent_node_id.clone(),
            branch_type: input.branch_type,
            current_input: input.prompt.clone(),
            path,
        });
        self.store.insert_turn(&input, &snapshot)
    }

    pub fn complete_turn(&self, input: CompleteTurnInput) -> KernelResult<ConversationNode> {
        input.validate()?;
        self.store.complete_turn(&input)
    }

    pub fn get_context_snapshot(&self, snapshot_id: &str) -> KernelResult<ContextSnapshot> {
        self.store.get_context_snapshot(snapshot_id)
    }

    pub fn update_node_position(&self, input: UpdateNodePositionInput) -> KernelResult<()> {
        input.validate()?;
        self.store.update_node_position(&input)
    }

    pub fn create_model_run(&self, request: &ModelRunRequest) -> KernelResult<()> {
        self.store.create_model_run(request)
    }

    pub fn record_model_run_event(&self, event: &ModelRunEventEnvelope) -> KernelResult<()> {
        self.store.record_model_run_event(event)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::domain::BranchType;

    fn service() -> (TempDir, KernelService) {
        let directory = TempDir::new().expect("temp directory");
        let service =
            KernelService::open(directory.path().join("mindscape.sqlite3")).expect("open kernel");
        (directory, service)
    }

    #[test]
    fn persists_a_conversation_graph_and_frozen_context() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Kernel test".into(),
            })
            .expect("create conversation");

        let root = service
            .append_turn(AppendTurnInput {
                conversation_id: conversation.id.clone(),
                parent_node_id: None,
                branch_type: BranchType::Continues,
                title: "Root".into(),
                prompt: "What should V1 contain?".into(),
                provider_id: Some("openai".into()),
                model_id: Some("test-model".into()),
            })
            .expect("append root");
        let root = service
            .complete_turn(CompleteTurnInput {
                node_id: root.id,
                content: "A traceable conversation graph.".into(),
                provider_id: "openai".into(),
                model_id: "test-model".into(),
            })
            .expect("complete root");

        let reframed = service
            .append_turn(AppendTurnInput {
                conversation_id: conversation.id.clone(),
                parent_node_id: Some(root.id.clone()),
                branch_type: BranchType::Reframes,
                title: "Reframe".into(),
                prompt: "Re-evaluate without accepting that answer.".into(),
                provider_id: Some("anthropic".into()),
                model_id: Some("test-model-2".into()),
            })
            .expect("append reframe");

        let snapshot = service
            .get_context_snapshot(&reframed.context_snapshot_id)
            .expect("load snapshot");
        assert_eq!(snapshot.selected_messages.len(), 1);
        assert_eq!(snapshot.omitted_messages.len(), 1);
        assert_eq!(
            snapshot.omitted_messages[0].message_id,
            root.assistant_message.unwrap().id
        );

        let graph = service
            .load_conversation_graph(&conversation.id)
            .expect("load graph");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].relation, BranchType::Reframes);
    }

    #[test]
    fn records_domain_events_for_committed_changes() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Events".into(),
            })
            .expect("create conversation");
        let node = service
            .append_turn(AppendTurnInput {
                conversation_id: conversation.id.clone(),
                parent_node_id: None,
                branch_type: BranchType::Continues,
                title: "Root".into(),
                prompt: "Test events".into(),
                provider_id: None,
                model_id: None,
            })
            .expect("append turn");
        service
            .update_node_position(UpdateNodePositionInput {
                conversation_id: conversation.id.clone(),
                node_id: node.id,
                x: 120.0,
                y: 240.0,
            })
            .expect("update position");

        assert_eq!(service.store.event_count(&conversation.id).unwrap(), 3);
    }
}

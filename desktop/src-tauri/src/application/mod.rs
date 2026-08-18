use std::sync::{Arc, Mutex};

use crate::{
    adapters::SqliteStore,
    domain::{
        AppendTurnInput, CanvasViewportState, CompleteTurnInput, ContextCompileInput,
        ContextSnapshot, Conversation, ConversationGraph, ConversationNode,
        CreateConversationInput, KernelBootstrap, KernelResult, SaveCanvasViewportInput,
        StartModelRunInput, UpdateNodePositionInput, compile_context,
        contracts::{
            ModelRunEventEnvelope, ModelRunProjection, ModelRunRequest, RUNTIME_CONTRACT_VERSION,
        },
        new_id, now_timestamp,
    },
};

#[derive(Debug, Clone)]
pub struct KernelService {
    store: SqliteStore,
    run_preparation: Arc<Mutex<()>>,
}

impl KernelService {
    #[cfg(test)]
    pub fn open(database_path: impl AsRef<std::path::Path>) -> KernelResult<Self> {
        Ok(Self {
            store: SqliteStore::open(database_path)?,
            run_preparation: Arc::new(Mutex::new(())),
        })
    }

    pub fn open_with_backup_dir(
        database_path: impl AsRef<std::path::Path>,
        backup_dir: impl AsRef<std::path::Path>,
    ) -> KernelResult<Self> {
        Ok(Self {
            store: SqliteStore::open_with_backup_dir(database_path, backup_dir)?,
            run_preparation: Arc::new(Mutex::new(())),
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
        self.append_turn_with_context_budget(input, None)
    }

    fn append_turn_with_context_budget(
        &self,
        input: AppendTurnInput,
        max_context_tokens: Option<i64>,
    ) -> KernelResult<ConversationNode> {
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
            max_context_tokens,
        })?;
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

    pub fn save_canvas_viewport(
        &self,
        input: SaveCanvasViewportInput,
    ) -> KernelResult<CanvasViewportState> {
        self.store.save_canvas_viewport(&input)
    }

    pub fn get_canvas_viewport(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Option<CanvasViewportState>> {
        self.store.get_canvas_viewport(conversation_id)
    }

    pub fn create_model_run(&self, request: &ModelRunRequest) -> KernelResult<()> {
        self.store.create_model_run(request)
    }

    pub fn record_model_run_event(&self, event: &ModelRunEventEnvelope) -> KernelResult<()> {
        self.store.record_model_run_event(event)
    }

    pub fn prepare_model_run(
        &self,
        input: StartModelRunInput,
        max_context_tokens: Option<i64>,
    ) -> KernelResult<(ConversationNode, ModelRunRequest)> {
        input.validate()?;
        let _preparation_guard = self.run_preparation.lock().map_err(|_| {
            crate::domain::KernelError::Integrity("model run preparation lock was poisoned".into())
        })?;
        if let Some(request) = self
            .store
            .model_run_request_by_idempotency_key(&input.idempotency_key)?
        {
            let node = self.store.load_model_run_node(&request.node_id)?;
            validate_idempotent_replay(&input, &node, &request)?;
            return Ok((node, request));
        }
        let node = self.append_turn_with_context_budget(
            AppendTurnInput {
                conversation_id: input.conversation_id.clone(),
                parent_node_id: input.parent_node_id,
                branch_type: input.branch_type,
                title: input.title,
                prompt: input.prompt,
                provider_id: Some(input.provider_id.clone()),
                model_id: Some(input.model_id.clone()),
            },
            max_context_tokens,
        )?;
        let snapshot = self.get_context_snapshot(&node.context_snapshot_id)?;
        let request = ModelRunRequest {
            contract_version: RUNTIME_CONTRACT_VERSION.into(),
            run_id: new_id("run"),
            conversation_id: input.conversation_id,
            node_id: node.id.clone(),
            context_snapshot: snapshot,
            provider_id: input.provider_id,
            model_id: input.model_id,
            capabilities: input.capabilities,
            budget: input.budget,
            idempotency_key: input.idempotency_key,
            created_at: now_timestamp(),
        };
        self.create_model_run(&request)?;
        Ok((node, request))
    }

    pub fn recover_interrupted_runs(&self) -> KernelResult<Vec<ModelRunProjection>> {
        self.store.recover_interrupted_runs()?;
        self.store.list_model_runs(None)
    }

    pub fn list_model_runs(
        &self,
        conversation_id: Option<&str>,
    ) -> KernelResult<Vec<ModelRunProjection>> {
        self.store.list_model_runs(conversation_id)
    }
}

fn validate_idempotent_replay(
    input: &StartModelRunInput,
    node: &ConversationNode,
    request: &ModelRunRequest,
) -> KernelResult<()> {
    let matches_original = input.conversation_id == request.conversation_id
        && input.parent_node_id == request.context_snapshot.parent_node_id
        && input.branch_type == request.context_snapshot.branch_type
        && input.title == node.title
        && input.prompt == request.context_snapshot.current_input
        && input.provider_id == request.provider_id
        && input.model_id == request.model_id
        && input.capabilities == request.capabilities
        && input.budget == request.budget;

    if matches_original {
        Ok(())
    } else {
        Err(crate::domain::KernelError::Validation(
            "idempotency key was already used for a different model run request".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Barrier;

    use tempfile::TempDir;

    use super::*;
    use crate::domain::{
        BranchType, RunState,
        contracts::{CapabilityRequirement, ModelRunBudget},
    };

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

    #[test]
    fn prepare_model_run_persists_a_budgeted_snapshot_without_mutating_history() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Budget invariant".into(),
            })
            .expect("create conversation");

        let root = service
            .append_turn(AppendTurnInput {
                conversation_id: conversation.id.clone(),
                parent_node_id: None,
                branch_type: BranchType::Continues,
                title: "Root".into(),
                prompt: "12345678".into(),
                provider_id: Some("mock".into()),
                model_id: Some("mock-chat".into()),
            })
            .expect("append root");
        let root = service
            .complete_turn(CompleteTurnInput {
                node_id: root.id,
                content: "abcdefgh".into(),
                provider_id: "mock".into(),
                model_id: "mock-chat".into(),
            })
            .expect("complete root");
        let parent = service
            .append_turn(AppendTurnInput {
                conversation_id: conversation.id.clone(),
                parent_node_id: Some(root.id.clone()),
                branch_type: BranchType::Continues,
                title: "Parent".into(),
                prompt: "u2".into(),
                provider_id: Some("mock".into()),
                model_id: Some("mock-chat".into()),
            })
            .expect("append parent");
        let parent = service
            .complete_turn(CompleteTurnInput {
                node_id: parent.id,
                content: "a2".into(),
                provider_id: "mock".into(),
                model_id: "mock-chat".into(),
            })
            .expect("complete parent");

        let (_node, request) = service
            .prepare_model_run(
                StartModelRunInput {
                    conversation_id: conversation.id.clone(),
                    parent_node_id: Some(parent.id.clone()),
                    branch_type: BranchType::Continues,
                    title: "Budgeted child".into(),
                    prompt: "next".into(),
                    provider_id: "mock".into(),
                    model_id: "mock-chat".into(),
                    capabilities: vec![CapabilityRequirement::TextInput],
                    budget: ModelRunBudget {
                        max_output_tokens: Some(16),
                        max_cost_microunits: None,
                        timeout_ms: 1_000,
                    },
                    idempotency_key: "budget-invariant-run".into(),
                },
                Some(14),
            )
            .expect("prepare budgeted model run");

        assert_eq!(request.context_snapshot.estimated_tokens, 14);
        assert_eq!(request.context_snapshot.selected_messages.len(), 2);
        assert_eq!(request.context_snapshot.omitted_messages.len(), 2);
        assert_eq!(
            service
                .get_context_snapshot(&request.context_snapshot.id)
                .expect("load frozen snapshot"),
            request.context_snapshot
        );

        let graph = service
            .load_conversation_graph(&conversation.id)
            .expect("reload graph");
        let persisted_root = graph
            .nodes
            .iter()
            .find(|node| node.id == root.id)
            .expect("root remains in graph");
        assert_eq!(persisted_root.user_message, root.user_message);
        assert_eq!(persisted_root.assistant_message, root.assistant_message);
        assert_eq!(persisted_root.run_state, RunState::Completed);
    }

    #[test]
    fn context_budget_failure_leaves_no_pending_node_or_model_run() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Rejected budget".into(),
            })
            .expect("create conversation");

        let error = service
            .prepare_model_run(
                StartModelRunInput {
                    conversation_id: conversation.id.clone(),
                    parent_node_id: None,
                    branch_type: BranchType::Continues,
                    title: "Must not persist".into(),
                    prompt: "prompt exceeds the trusted context budget".into(),
                    provider_id: "mock".into(),
                    model_id: "mock-chat".into(),
                    capabilities: vec![CapabilityRequirement::TextInput],
                    budget: ModelRunBudget {
                        max_output_tokens: Some(16),
                        max_cost_microunits: None,
                        timeout_ms: 1_000,
                    },
                    idempotency_key: "rejected-budget-run".into(),
                },
                Some(1),
            )
            .expect_err("context budget should reject the run");

        assert!(error.to_string().contains("context budget"));
        let graph = service
            .load_conversation_graph(&conversation.id)
            .expect("reload graph");
        assert!(graph.nodes.is_empty());
        assert!(
            service
                .list_model_runs(Some(&conversation.id))
                .expect("list model runs")
                .is_empty()
        );
    }

    #[test]
    fn idempotency_key_rejects_a_different_request_without_creating_an_orphan_node() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Idempotency invariant".into(),
            })
            .expect("create conversation");
        let original = StartModelRunInput {
            conversation_id: conversation.id.clone(),
            parent_node_id: None,
            branch_type: BranchType::Continues,
            title: "Original".into(),
            prompt: "first payload".into(),
            provider_id: "mock".into(),
            model_id: "mock-chat".into(),
            capabilities: vec![CapabilityRequirement::TextInput],
            budget: ModelRunBudget {
                max_output_tokens: Some(16),
                max_cost_microunits: None,
                timeout_ms: 1_000,
            },
            idempotency_key: "shared-key".into(),
        };
        let (node, request) = service
            .prepare_model_run(original.clone(), None)
            .expect("prepare original run");

        let (replayed_node, replayed_request) = service
            .prepare_model_run(original.clone(), None)
            .expect("replay identical request");
        assert_eq!(replayed_node.id, node.id);
        assert_eq!(replayed_request.run_id, request.run_id);

        let error = service
            .prepare_model_run(
                StartModelRunInput {
                    prompt: "different payload".into(),
                    ..original
                },
                None,
            )
            .expect_err("different payload must not reuse the key");
        assert!(error.to_string().contains("idempotency key"));

        let graph = service
            .load_conversation_graph(&conversation.id)
            .expect("reload graph");
        assert_eq!(graph.nodes.len(), 1);
        let runs = service
            .list_model_runs(Some(&conversation.id))
            .expect("list runs");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_id, request.run_id);
    }

    #[test]
    fn concurrent_idempotent_preparation_creates_exactly_one_node_and_run() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Concurrent idempotency".into(),
            })
            .expect("create conversation");
        let input = StartModelRunInput {
            conversation_id: conversation.id.clone(),
            parent_node_id: None,
            branch_type: BranchType::Continues,
            title: "Only once".into(),
            prompt: "same concurrent payload".into(),
            provider_id: "mock".into(),
            model_id: "mock-chat".into(),
            capabilities: vec![CapabilityRequirement::TextInput],
            budget: ModelRunBudget {
                max_output_tokens: Some(16),
                max_cost_microunits: None,
                timeout_ms: 1_000,
            },
            idempotency_key: "concurrent-shared-key".into(),
        };
        let barrier = Arc::new(Barrier::new(2));

        let handles = (0..2)
            .map(|_| {
                let service = service.clone();
                let input = input.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    service
                        .prepare_model_run(input, None)
                        .expect("prepare concurrent run")
                })
            })
            .collect::<Vec<_>>();
        let prepared = handles
            .into_iter()
            .map(|handle| handle.join().expect("join preparation thread"))
            .collect::<Vec<_>>();

        assert_eq!(prepared[0].0.id, prepared[1].0.id);
        assert_eq!(prepared[0].1.run_id, prepared[1].1.run_id);
        let graph = service
            .load_conversation_graph(&conversation.id)
            .expect("reload graph");
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(
            service
                .list_model_runs(Some(&conversation.id))
                .expect("list runs")
                .len(),
            1
        );
    }
}

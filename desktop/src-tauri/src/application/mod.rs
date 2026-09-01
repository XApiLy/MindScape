use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use crate::{
    adapters::{
        SEMANTIC_MODEL_DIMENSIONS, SEMANTIC_MODEL_VERSION, SemanticEmbedding, SqliteStore,
        provider::{
            KnowledgeVectorSnapshot, RetrievalAvailability, RetrievalProjectionError,
            SemanticQueryEmbedding, build_knowledge_embedding_record_from_vector,
            knowledge_search_text, retrieve_validated_knowledge,
            retrieve_validated_knowledge_with_semantic,
        },
    },
    domain::{
        AppendTurnInput, CanvasViewportState, CompleteTurnInput, ContextCompileInput,
        ContextSnapshot, Conversation, ConversationGraph, ConversationNode,
        CreateConversationInput, FocusFrameLifecycleCommandInput, FocusFrameLifecycleSnapshot,
        FocusFrameQueryProjection, FocusPromotionDecisionAction,
        FocusPromotionDecisionCommandInput, FocusPromotionDecisionProjection,
        FocusPromotionEntityMutation, KernelBootstrap, KernelError, KernelResult,
        KnowledgeRetrievalProjection, SaveCanvasViewportInput, StartModelRunInput,
        UpdateNodePositionInput, compile_context,
        contracts::{
            DISCUSSION_LOG_PROJECTION_CONTRACT_VERSION, DiscussionLog, DiscussionLogProjection,
            FocusFrame, FocusPromotionCandidateSet, GeneratorKind, GeneratorRef,
            ModelRunEventEnvelope, ModelRunProjection, ModelRunRequest, RUNTIME_CONTRACT_VERSION,
        },
        new_id, now_timestamp, plan_focus_promotion_decision,
        validate_focus_frame_query_projection,
    },
};

#[derive(Debug, Clone)]
pub struct KernelService {
    store: SqliteStore,
    run_preparation: Arc<Mutex<()>>,
    vault_projection: Arc<Mutex<()>>,
}

impl KernelService {
    #[cfg(test)]
    pub fn open(database_path: impl AsRef<std::path::Path>) -> KernelResult<Self> {
        Ok(Self {
            store: SqliteStore::open(database_path)?,
            run_preparation: Arc::new(Mutex::new(())),
            vault_projection: Arc::new(Mutex::new(())),
        })
    }

    pub fn open_with_backup_dir(
        database_path: impl AsRef<std::path::Path>,
        backup_dir: impl AsRef<std::path::Path>,
    ) -> KernelResult<Self> {
        Ok(Self {
            store: SqliteStore::open_with_backup_dir(database_path, backup_dir)?,
            run_preparation: Arc::new(Mutex::new(())),
            vault_projection: Arc::new(Mutex::new(())),
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

    pub fn persist_import_bundle(
        &self,
        source: &crate::domain::contracts::ImportSource,
        revision: &crate::domain::contracts::ImportRevision,
        messages: &[crate::domain::contracts::ImportedMessage],
        report: &crate::domain::contracts::ParseReport,
    ) -> KernelResult<()> {
        self.store
            .persist_import_bundle(source, revision, messages, report)
    }

    pub fn list_import_sources(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<crate::domain::contracts::ImportSource>> {
        if conversation_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "Conversation id must not be empty".into(),
            ));
        }
        self.store.list_import_sources(conversation_id)
    }

    pub fn list_import_storage_refs(&self) -> KernelResult<std::collections::HashSet<String>> {
        self.store.list_import_storage_refs()
    }

    pub fn get_import_bundle(
        &self,
        source_id: &str,
    ) -> KernelResult<crate::domain::contracts::ImportBundleQueryProjection> {
        if source_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "Import source id must not be empty".into(),
            ));
        }
        self.store.get_import_bundle(source_id)
    }

    pub fn create_focus_frame(
        &self,
        frame: FocusFrame,
    ) -> KernelResult<FocusFrameLifecycleSnapshot> {
        frame.validate()?;
        let snapshot = FocusFrameLifecycleSnapshot {
            contract_version: crate::domain::FOCUS_LIFECYCLE_CONTRACT_VERSION.into(),
            updated_at: frame.created_at.clone(),
            frame,
            status: crate::domain::FocusFrameLifecycleStatus::Active,
            revision: 1,
            closed_at: None,
        };
        self.store.insert_focus_frame_lifecycle(&snapshot)?;
        Ok(snapshot)
    }

    pub fn get_focus_frame_query(
        &self,
        focus_frame_id: &str,
    ) -> KernelResult<FocusFrameQueryProjection> {
        let lifecycle = self.store.get_focus_frame_lifecycle(focus_frame_id)?;
        let projection = FocusFrameQueryProjection {
            contract_version: crate::domain::FOCUS_QUERY_CONTRACT_VERSION.into(),
            lifecycle,
            focused_context: self.store.get_focused_context_snapshot(focus_frame_id)?,
        };
        validate_focus_frame_query_projection(&projection)?;
        Ok(projection)
    }

    pub fn list_focus_frame_queries(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<FocusFrameQueryProjection>> {
        if conversation_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "conversation id must not be empty".into(),
            ));
        }
        self.store
            .list_focus_frame_lifecycles(conversation_id)?
            .into_iter()
            .map(|lifecycle| {
                let focus_frame_id = lifecycle.frame.id.clone();
                let projection = FocusFrameQueryProjection {
                    contract_version: crate::domain::FOCUS_QUERY_CONTRACT_VERSION.into(),
                    lifecycle,
                    focused_context: self.store.get_focused_context_snapshot(&focus_frame_id)?,
                };
                validate_focus_frame_query_projection(&projection)?;
                Ok(projection)
            })
            .collect()
    }

    pub fn get_focus_promotion_candidates(
        &self,
        focus_frame_id: &str,
        expected_memory_version: Option<u64>,
    ) -> KernelResult<Option<FocusPromotionCandidateSet>> {
        if focus_frame_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "FocusFrame id must not be empty".into(),
            ));
        }
        if expected_memory_version == Some(0) {
            return Err(KernelError::Validation(
                "expected FocusFrame memory version must be greater than zero".into(),
            ));
        }

        let lifecycle = self.store.get_focus_frame_lifecycle(focus_frame_id)?;
        let frame = &lifecycle.frame;
        if expected_memory_version.is_some_and(|expected| expected != frame.memory_version) {
            return Err(KernelError::Integrity(format!(
                "focus frame {} memory version conflict",
                frame.id
            )));
        }
        let Some(mut candidates) = lifecycle.promotion_candidates()? else {
            return Ok(None);
        };
        let decided = self
            .store
            .list_focus_promotion_decisions(focus_frame_id)?
            .into_iter()
            .map(|decision| decision.candidate_ref)
            .collect::<std::collections::HashSet<_>>();
        candidates
            .candidate_refs
            .retain(|candidate_ref| !decided.contains(candidate_ref));
        Ok(Some(candidates))
    }

    pub fn decide_focus_promotion(
        &self,
        vault: &crate::adapters::MarkdownVault,
        input: FocusPromotionDecisionCommandInput,
    ) -> KernelResult<FocusPromotionDecisionProjection> {
        let _projection_guard = self.vault_projection.lock().map_err(|_| {
            KernelError::Integrity("Markdown Vault projection lock was poisoned".into())
        })?;
        let actor = GeneratorRef {
            kind: GeneratorKind::User,
            generator_id: "mindscape-local-user".into(),
            generator_version: "v1".into(),
        };
        if let Some(decision) = self.store.replay_focus_promotion_decision(&input)? {
            self.project_persisted_focus_promotion(vault, &decision)?;
            return Ok(decision);
        }

        let lifecycle = self
            .store
            .get_focus_frame_lifecycle(&input.focus_frame_id)?;
        let candidates = lifecycle.promotion_candidates()?.ok_or_else(|| {
            KernelError::Validation("FocusFrame has no promotion candidates".into())
        })?;
        let conversation_id = lifecycle.frame.conversation_id.clone();
        let entity = self
            .store
            .get_knowledge_entity(&conversation_id, &input.candidate_ref)?;
        let plan = plan_focus_promotion_decision(&input, &candidates, &lifecycle, &entity, &actor)
            .map_err(|error| KernelError::Validation(error.to_string()))?;
        let mut final_entities = self.store.list_all_knowledge_entities()?;
        apply_focus_plan_to_projection(&mut final_entities, &plan.entity_mutation);
        let relations = self.store.list_knowledge_relations(&conversation_id)?;
        let vault_backup = vault.apply_focus_promotion_plan(&plan, &final_entities, &relations)?;

        match self.store.persist_focus_promotion_decision(&input, &actor) {
            Ok(decision) => {
                vault.commit_focus_promotion(vault_backup)?;
                Ok(decision)
            }
            Err(store_error) => match vault.rollback_focus_promotion(vault_backup) {
                Ok(()) => Err(store_error),
                Err(rollback_error) => Err(KernelError::Integrity(format!(
                    "focus promotion SQLite persistence failed ({store_error}); Vault rollback also failed ({rollback_error})"
                ))),
            },
        }
    }

    pub fn get_focus_promotion_decision(
        &self,
        decision_id: &str,
    ) -> KernelResult<FocusPromotionDecisionProjection> {
        self.store.get_focus_promotion_decision(decision_id)
    }

    pub fn list_focus_promotion_decisions(
        &self,
        focus_frame_id: &str,
    ) -> KernelResult<Vec<FocusPromotionDecisionProjection>> {
        self.store.list_focus_promotion_decisions(focus_frame_id)
    }

    pub fn list_all_focus_promotion_decision_ids(
        &self,
    ) -> KernelResult<std::collections::HashSet<String>> {
        Ok(self
            .store
            .list_all_focus_promotion_decisions()?
            .into_iter()
            .map(|decision| decision.decision_id)
            .collect())
    }

    fn project_focus_promotion_entity(
        &self,
        vault: &crate::adapters::MarkdownVault,
        conversation_id: &str,
        entity_id: &str,
    ) -> KernelResult<()> {
        let entity = self
            .store
            .get_knowledge_entity(conversation_id, entity_id)?;
        let relations = self.store.list_knowledge_relations(conversation_id)?;
        vault.write_entity_with_relations(&entity, &relations)?;
        Ok(())
    }

    fn project_persisted_focus_promotion(
        &self,
        vault: &crate::adapters::MarkdownVault,
        decision: &FocusPromotionDecisionProjection,
    ) -> KernelResult<()> {
        match decision.action {
            FocusPromotionDecisionAction::Delete => {
                vault.remove_entity(&decision.candidate_ref)?;
                let entities = self
                    .store
                    .list_knowledge_entities(&decision.conversation_id)?;
                let relations = self
                    .store
                    .list_knowledge_relations(&decision.conversation_id)?;
                for entity in &entities {
                    vault.write_entity_with_relations(entity, &relations)?;
                }
            }
            FocusPromotionDecisionAction::Confirm | FocusPromotionDecisionAction::Reject => {
                self.project_focus_promotion_entity(
                    vault,
                    &decision.conversation_id,
                    &decision.candidate_ref,
                )?;
            }
            FocusPromotionDecisionAction::Promote => {
                self.project_focus_promotion_entity(
                    vault,
                    &decision.conversation_id,
                    &decision.candidate_ref,
                )?;
                let promoted_entity_id =
                    decision.promoted_entity_id.as_deref().ok_or_else(|| {
                        KernelError::Integrity(
                            "persisted promote decision is missing its target entity id".into(),
                        )
                    })?;
                self.project_focus_promotion_entity(
                    vault,
                    &decision.conversation_id,
                    promoted_entity_id,
                )?;
            }
        }
        vault.write_entity_index(&self.store.list_all_knowledge_entities()?)
    }

    pub fn save_focused_context_snapshot(
        &self,
        snapshot: crate::domain::FocusedContextSnapshot,
    ) -> KernelResult<FocusFrameQueryProjection> {
        let focus_frame_id = snapshot.focus_frame.id.clone();
        self.store.upsert_focused_context_snapshot(&snapshot)?;
        self.get_focus_frame_query(&focus_frame_id)
    }

    pub fn upsert_knowledge_entity(
        &self,
        conversation_id: &str,
        entity: crate::domain::contracts::KnowledgeEntity,
    ) -> KernelResult<()> {
        self.store.upsert_knowledge_entity(conversation_id, &entity)
    }

    pub fn list_knowledge_entities(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<crate::domain::contracts::KnowledgeEntity>> {
        self.store.list_knowledge_entities(conversation_id)
    }

    pub fn upsert_knowledge_relation(
        &self,
        conversation_id: &str,
        relation: crate::domain::contracts::KnowledgeRelation,
    ) -> KernelResult<()> {
        self.store
            .upsert_knowledge_relation(conversation_id, &relation)
    }

    pub fn list_knowledge_relations(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<crate::domain::contracts::KnowledgeRelation>> {
        self.store.list_knowledge_relations(conversation_id)
    }

    #[cfg(test)]
    pub fn retrieve_knowledge(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
    ) -> KernelResult<KnowledgeRetrievalProjection> {
        if conversation_id.trim().is_empty()
            || query.trim().is_empty()
            || !(1..=50).contains(&limit)
        {
            return Err(KernelError::Validation(
                "knowledge retrieval requires a conversation, query, and limit from 1 to 50".into(),
            ));
        }
        let expanded_limit = limit.saturating_mul(3).min(150);
        let entities = self.store.list_knowledge_entities(conversation_id)?;
        let relations = self.store.list_knowledge_relations(conversation_id)?;
        let full_text_matches =
            self.store
                .search_knowledge_full_text(conversation_id, query, expanded_limit)?;
        let vector_snapshot = self.store.load_knowledge_vector_snapshot(conversation_id)?;
        retrieve_validated_knowledge(
            conversation_id,
            query,
            limit,
            &entities,
            &relations,
            full_text_matches,
            vector_snapshot,
        )
        .map_err(map_retrieval_error)
    }

    pub fn retrieve_knowledge_with_semantic(
        &self,
        semantic: &SemanticEmbedding,
        conversation_id: &str,
        query: &str,
        limit: usize,
    ) -> KernelResult<KnowledgeRetrievalProjection> {
        let Ok(vector) = semantic.embed(query) else {
            return self.retrieve_knowledge_without_vector(conversation_id, query, limit);
        };
        let expanded_limit = limit.saturating_mul(3).min(150);
        let entities = self.store.list_knowledge_entities(conversation_id)?;
        let relations = self.store.list_knowledge_relations(conversation_id)?;
        let full_text_matches =
            self.store
                .search_knowledge_full_text(conversation_id, query, expanded_limit)?;
        let mut vector_snapshot = self.store.load_knowledge_vector_snapshot(conversation_id)?;
        let confirmed_count = entities
            .iter()
            .filter(|entity| entity.status == crate::domain::contracts::KnowledgeStatus::Confirmed)
            .count();
        let semantic_snapshot_ready = vector_snapshot.availability
            == RetrievalAvailability::Available
            && vector_snapshot.records.len() == confirmed_count
            && vector_snapshot.records.iter().all(|record| {
                record.metadata.model_version == SEMANTIC_MODEL_VERSION
                    && record.metadata.dimensions == SEMANTIC_MODEL_DIMENSIONS
                    && record.vector.len() == SEMANTIC_MODEL_DIMENSIONS
            });
        if !semantic_snapshot_ready {
            if self
                .rebuild_knowledge_vector_index_with_semantic(semantic, conversation_id)
                .is_err()
            {
                return self.retrieve_knowledge_without_vector(conversation_id, query, limit);
            }
            vector_snapshot = self.store.load_knowledge_vector_snapshot(conversation_id)?;
        }
        retrieve_validated_knowledge_with_semantic(
            conversation_id,
            query,
            limit,
            &entities,
            &relations,
            full_text_matches,
            vector_snapshot,
            Some(&SemanticQueryEmbedding {
                model_version: SEMANTIC_MODEL_VERSION.into(),
                dimensions: SEMANTIC_MODEL_DIMENSIONS,
                vector,
            }),
        )
        .map_err(map_retrieval_error)
    }

    pub fn retrieve_knowledge_without_vector(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
    ) -> KernelResult<KnowledgeRetrievalProjection> {
        let expanded_limit = limit.saturating_mul(3).min(150);
        let entities = self.store.list_knowledge_entities(conversation_id)?;
        let relations = self.store.list_knowledge_relations(conversation_id)?;
        let full_text_matches =
            self.store
                .search_knowledge_full_text(conversation_id, query, expanded_limit)?;
        retrieve_validated_knowledge(
            conversation_id,
            query,
            limit,
            &entities,
            &relations,
            full_text_matches,
            KnowledgeVectorSnapshot {
                availability: RetrievalAvailability::Unavailable,
                records: Vec::new(),
            },
        )
        .map_err(map_retrieval_error)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn rebuild_knowledge_vector_index(&self, conversation_id: &str) -> KernelResult<usize> {
        self.store.rebuild_knowledge_vector_index(conversation_id)
    }

    pub fn rebuild_knowledge_vector_index_with_semantic(
        &self,
        semantic: &SemanticEmbedding,
        conversation_id: &str,
    ) -> KernelResult<usize> {
        let entities = self.store.list_knowledge_entities(conversation_id)?;
        let confirmed = entities
            .iter()
            .filter(|entity| entity.status == crate::domain::contracts::KnowledgeStatus::Confirmed)
            .collect::<Vec<_>>();
        let search_texts = confirmed
            .iter()
            .map(|entity| knowledge_search_text(entity))
            .collect::<Vec<_>>();
        let text_refs = search_texts.iter().map(String::as_str).collect::<Vec<_>>();
        let vectors = semantic
            .embed_batch(&text_refs)
            .map_err(|error| KernelError::Integrity(error.to_string()))?;
        let mut records = Vec::with_capacity(confirmed.len());
        for (entity, vector) in confirmed.into_iter().zip(vectors) {
            records.push(
                build_knowledge_embedding_record_from_vector(
                    entity,
                    SEMANTIC_MODEL_VERSION,
                    SEMANTIC_MODEL_DIMENSIONS,
                    vector,
                )
                .map_err(map_retrieval_error)?,
            );
        }
        self.store.replace_knowledge_vector_records(
            conversation_id,
            SEMANTIC_MODEL_VERSION,
            SEMANTIC_MODEL_DIMENSIONS,
            &records,
        )
    }

    pub fn delete_knowledge_entity_and_vault(
        &self,
        vault: &crate::adapters::MarkdownVault,
        conversation_id: &str,
        entity_id: &str,
    ) -> KernelResult<bool> {
        let _projection_guard = self.vault_projection.lock().map_err(|_| {
            KernelError::Integrity("Markdown Vault projection lock was poisoned".into())
        })?;
        let conversation_entities = self.store.list_knowledge_entities(conversation_id)?;
        if !conversation_entities
            .iter()
            .any(|entity| entity.id == entity_id)
        {
            return Ok(false);
        }
        let relations = self.store.list_knowledge_relations(conversation_id)?;
        let neighbor_ids = relations
            .iter()
            .filter_map(|relation| {
                if relation.source_entity_id == entity_id {
                    Some(relation.target_entity_id.as_str())
                } else if relation.target_entity_id == entity_id {
                    Some(relation.source_entity_id.as_str())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        let neighbor_entities = conversation_entities
            .iter()
            .filter(|entity| neighbor_ids.contains(entity.id.as_str()))
            .collect::<Vec<_>>();
        if neighbor_entities.len() != neighbor_ids.len() {
            return Err(KernelError::Integrity(
                "knowledge delete relation references a missing neighbor entity".into(),
            ));
        }
        let final_relations = relations
            .iter()
            .filter(|relation| {
                relation.source_entity_id != entity_id && relation.target_entity_id != entity_id
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut final_entities = self.store.list_all_knowledge_entities()?;
        final_entities.retain(|entity| entity.id != entity_id);
        let vault_backup = vault.apply_knowledge_entity_delete(
            entity_id,
            &neighbor_entities,
            &final_entities,
            &final_relations,
        )?;
        match self
            .store
            .delete_knowledge_entity(conversation_id, entity_id)
        {
            Ok(true) => {
                vault.commit_knowledge_entity_delete(vault_backup)?;
                Ok(true)
            }
            Ok(false) => {
                vault.rollback_knowledge_entity_delete(vault_backup)?;
                Ok(false)
            }
            Err(store_error) => match vault.rollback_knowledge_entity_delete(vault_backup) {
                Ok(()) => Err(store_error),
                Err(rollback_error) => Err(KernelError::Integrity(format!(
                    "knowledge delete SQLite persistence failed ({store_error}); Vault rollback also failed ({rollback_error})"
                ))),
            },
        }
    }

    pub fn list_all_knowledge_entity_ids(&self) -> KernelResult<std::collections::HashSet<String>> {
        self.store.list_all_knowledge_entity_ids()
    }

    pub fn recover_knowledge_entity_delete_vault(
        &self,
        vault: &crate::adapters::MarkdownVault,
    ) -> KernelResult<u64> {
        vault.recover_knowledge_entity_delete_transactions(
            &self.store.list_all_knowledge_entities()?,
            &self.store.list_all_knowledge_relations()?,
        )
    }

    pub fn upsert_evidence_ref(
        &self,
        conversation_id: &str,
        evidence: crate::domain::contracts::EvidenceRef,
    ) -> KernelResult<()> {
        self.store.upsert_evidence_ref(conversation_id, &evidence)
    }

    pub fn close_focus_frame(
        &self,
        input: FocusFrameLifecycleCommandInput,
    ) -> KernelResult<FocusFrameQueryProjection> {
        self.transition_focus_frame(input, crate::domain::FocusFrameLifecycleAction::Close)
    }

    pub fn reopen_focus_frame(
        &self,
        input: FocusFrameLifecycleCommandInput,
    ) -> KernelResult<FocusFrameQueryProjection> {
        self.transition_focus_frame(input, crate::domain::FocusFrameLifecycleAction::Reopen)
    }

    fn transition_focus_frame(
        &self,
        input: FocusFrameLifecycleCommandInput,
        action: crate::domain::FocusFrameLifecycleAction,
    ) -> KernelResult<FocusFrameQueryProjection> {
        if input.focus_frame_id.trim().is_empty() {
            return Err(KernelError::Validation(
                "FocusFrame id must not be empty".into(),
            ));
        }
        let current = self
            .store
            .get_focus_frame_lifecycle(&input.focus_frame_id)?;
        if current.revision != input.expected_revision {
            return Err(KernelError::Integrity(format!(
                "focus frame {} revision conflict",
                input.focus_frame_id
            )));
        }
        let next = crate::domain::transition_focus_frame(&current, action, &input.updated_at)
            .map_err(|error| KernelError::Validation(error.to_string()))?;
        self.store
            .update_focus_frame_lifecycle(&next, input.expected_revision)?;
        self.get_focus_frame_query(&input.focus_frame_id)
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
            effective_run_profile: input.effective_run_profile,
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

    pub fn project_knowledge_entity_markdown(
        &self,
        vault: &crate::adapters::MarkdownVault,
        conversation_id: &str,
        entity_id: &str,
    ) -> KernelResult<crate::domain::contracts::MarkdownProjection> {
        let _projection_guard = self.vault_projection.lock().map_err(|_| {
            KernelError::Integrity("Markdown Vault projection lock was poisoned".into())
        })?;
        let entity = self
            .store
            .get_knowledge_entity(conversation_id, entity_id)?;
        let relations = self.store.list_knowledge_relations(conversation_id)?;
        let projection_revision = self.store.next_markdown_projection_revision(entity_id)?;
        let (relative_path, content_hash) =
            vault.write_entity_with_relations(&entity, &relations)?;
        let projection = crate::domain::contracts::MarkdownProjection {
            contract_version: crate::domain::contracts::MARKDOWN_PROJECTION_CONTRACT_VERSION.into(),
            id: format!("markdown-{}", entity.id),
            target_entity_id: entity.id,
            relative_path,
            entity_revision: entity.revision,
            projection_revision,
            content_hash,
            frontmatter_version: "mindscape.frontmatter.v1".into(),
            created_at: crate::domain::now_timestamp(),
        };
        self.store.persist_markdown_projection(&projection)?;
        vault.write_entity_index(&self.store.list_all_knowledge_entities()?)?;
        Ok(projection)
    }

    pub fn project_discussion_log_markdown(
        &self,
        vault: &crate::adapters::MarkdownVault,
        log: DiscussionLog,
    ) -> KernelResult<DiscussionLogProjection> {
        log.validate()?;
        let _projection_guard = self.vault_projection.lock().map_err(|_| {
            KernelError::Integrity("Markdown Vault projection lock was poisoned".into())
        })?;
        if log.revision != self.store.next_discussion_log_revision(&log.id)? {
            return Err(KernelError::Integrity(format!(
                "discussion log {} revision conflict",
                log.id
            )));
        }
        let (relative_path, content_hash, vault_backup) = vault.apply_discussion_log(&log)?;
        let projection = DiscussionLogProjection {
            contract_version: DISCUSSION_LOG_PROJECTION_CONTRACT_VERSION.into(),
            log,
            relative_path,
            content_hash,
        };
        if let Err(store_error) = self.store.persist_discussion_log_projection(&projection) {
            return match vault.rollback_discussion_log(vault_backup) {
                Ok(()) => Err(store_error),
                Err(rollback_error) => Err(KernelError::Integrity(format!(
                    "DiscussionLog SQLite persistence failed ({store_error}); Vault rollback also failed ({rollback_error})"
                ))),
            };
        }
        vault.write_discussion_index(&self.store.list_all_discussion_logs()?)?;
        vault.commit_discussion_log(vault_backup)?;
        Ok(projection)
    }

    pub fn recover_discussion_vault(
        &self,
        vault: &crate::adapters::MarkdownVault,
    ) -> KernelResult<u64> {
        vault.recover_discussion_transactions(&self.store.list_all_discussion_logs()?)
    }

    pub fn get_discussion_log(
        &self,
        discussion_log_id: &str,
    ) -> KernelResult<DiscussionLogProjection> {
        self.store.get_discussion_log_projection(discussion_log_id)
    }

    pub fn list_conversation_discussion_logs(
        &self,
        conversation_id: &str,
    ) -> KernelResult<Vec<DiscussionLogProjection>> {
        self.store
            .list_conversation_discussion_logs(conversation_id)
    }

    pub fn list_project_discussion_logs(
        &self,
        project_id: &str,
    ) -> KernelResult<Vec<DiscussionLogProjection>> {
        self.store.list_project_discussion_logs(project_id)
    }

    pub fn import_discussion_log_edit(
        &self,
        vault: &crate::adapters::MarkdownVault,
        discussion_log_id: &str,
    ) -> KernelResult<(DiscussionLogProjection, bool)> {
        let current = self
            .store
            .get_discussion_log_projection(discussion_log_id)?;
        let edit = vault.read_discussion_log_edit(discussion_log_id)?;
        if edit.content_hash == current.content_hash {
            return Ok((current, false));
        }
        if edit.relative_path != current.relative_path {
            return Err(KernelError::Integrity(
                "DiscussionLog edit path does not match its latest projection".into(),
            ));
        }
        let mut log = current.log;
        log.title = edit.title;
        log.body_markdown = edit.body_markdown;
        log.revision = log
            .revision
            .checked_add(1)
            .ok_or_else(|| KernelError::Integrity("DiscussionLog revision overflowed".into()))?;
        log.updated_at = crate::domain::now_timestamp();
        let projection = self.project_discussion_log_markdown(vault, log)?;
        Ok((projection, true))
    }

    pub fn list_markdown_projections(
        &self,
        entity_id: &str,
    ) -> KernelResult<Vec<crate::domain::contracts::MarkdownProjection>> {
        self.store.list_markdown_projections(entity_id)
    }

    pub fn import_markdown_entity_edit(
        &self,
        vault: &crate::adapters::MarkdownVault,
        conversation_id: &str,
        entity_id: &str,
    ) -> KernelResult<(crate::domain::contracts::MarkdownProjection, bool)> {
        let edit = vault.read_entity_edit(entity_id)?;
        let history = self.store.list_markdown_projections(entity_id)?;
        if let Some(current) = history.first()
            && current.content_hash == edit.content_hash
        {
            return Ok((current.clone(), false));
        }
        let mut entity = self
            .store
            .get_knowledge_entity(conversation_id, entity_id)?;
        entity.name = edit.name;
        entity.revision = entity
            .revision
            .checked_add(1)
            .ok_or_else(|| KernelError::Integrity("KnowledgeEntity revision overflowed".into()))?;
        entity.updated_at = crate::domain::now_timestamp();
        entity.generator = crate::domain::contracts::GeneratorRef {
            kind: crate::domain::contracts::GeneratorKind::User,
            generator_id: "vault-edit".into(),
            generator_version: "v1".into(),
        };
        let projection = if let Some(current) = history.first() {
            current.next_revision(
                edit.relative_path,
                entity.revision,
                edit.content_hash,
                "mindscape.frontmatter.v1".into(),
                entity.updated_at.clone(),
            )?
        } else {
            crate::domain::contracts::MarkdownProjection {
                contract_version: crate::domain::contracts::MARKDOWN_PROJECTION_CONTRACT_VERSION
                    .into(),
                id: format!("markdown-{entity_id}"),
                target_entity_id: entity_id.into(),
                relative_path: edit.relative_path,
                entity_revision: entity.revision,
                projection_revision: 1,
                content_hash: edit.content_hash,
                frontmatter_version: "mindscape.frontmatter.v1".into(),
                created_at: entity.updated_at.clone(),
            }
        };
        self.store
            .persist_markdown_entity_revision(conversation_id, &entity, &projection)?;
        Ok((projection, true))
    }
}

fn apply_focus_plan_to_projection(
    entities: &mut Vec<crate::domain::contracts::KnowledgeEntity>,
    mutation: &FocusPromotionEntityMutation,
) {
    let replace = |entities: &mut Vec<crate::domain::contracts::KnowledgeEntity>,
                   entity: &crate::domain::contracts::KnowledgeEntity| {
        if let Some(current) = entities.iter_mut().find(|current| current.id == entity.id) {
            *current = entity.clone();
        } else {
            entities.push(entity.clone());
        }
    };
    match mutation {
        FocusPromotionEntityMutation::UpsertSource(source) => replace(entities, source),
        FocusPromotionEntityMutation::Promote { source, promoted } => {
            replace(entities, source);
            replace(entities, promoted);
        }
        FocusPromotionEntityMutation::DeleteSource { entity_id, .. } => {
            entities.retain(|entity| entity.id != *entity_id);
        }
    }
    entities.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn map_retrieval_error(error: RetrievalProjectionError) -> KernelError {
    match error {
        RetrievalProjectionError::InvalidQuery => KernelError::Validation(error.to_string()),
        RetrievalProjectionError::InvalidFact(_)
        | RetrievalProjectionError::InvalidScore { .. }
        | RetrievalProjectionError::EntityMismatch { .. }
        | RetrievalProjectionError::InvalidProjection(_) => {
            KernelError::Integrity(error.to_string())
        }
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
        && input.budget == request.budget
        && input.effective_run_profile == request.effective_run_profile;

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

    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;
    use crate::domain::{
        BranchType, RunState,
        contracts::{
            CapabilityRequirement, DISCUSSION_LOG_CONTRACT_VERSION, DiscussionLog,
            DiscussionLogScope, EvidenceRef, EvidenceTarget, FOCUS_CONTRACT_VERSION,
            FocusBranchKind, FocusContextPolicy, FocusMemoryScope, GeneratorKind, GeneratorRef,
            KNOWLEDGE_CONTRACT_VERSION, KnowledgeEntity, KnowledgeEntityKind, KnowledgeRelation,
            KnowledgeRelationKind, KnowledgeScope, KnowledgeStatus, ModelRunBudget,
            ScopedEvidenceRef,
        },
    };

    fn service() -> (TempDir, KernelService) {
        let directory = TempDir::new().expect("temp directory");
        let service =
            KernelService::open(directory.path().join("mindscape.sqlite3")).expect("open kernel");
        (directory, service)
    }

    fn focus_frame(conversation_id: &str) -> FocusFrame {
        FocusFrame {
            contract_version: FOCUS_CONTRACT_VERSION.into(),
            id: "focus-service-1".into(),
            conversation_id: conversation_id.into(),
            parent_node_id: None,
            objective: "Validate lifecycle service wiring".into(),
            active_work_item: None,
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
            created_at: "2026-08-25T15:30:00Z".into(),
        }
    }

    fn knowledge_entity(
        workspace_id: &str,
        conversation_id: &str,
        id: &str,
        name: &str,
        status: KnowledgeStatus,
    ) -> KnowledgeEntity {
        KnowledgeEntity {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: id.into(),
            kind: KnowledgeEntityKind::Decision,
            name: name.into(),
            aliases: vec![],
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

    fn knowledge_relation(
        workspace_id: &str,
        conversation_id: &str,
        source_entity_id: &str,
        target_entity_id: &str,
    ) -> KnowledgeRelation {
        KnowledgeRelation {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "relation-hybrid".into(),
            kind: KnowledgeRelationKind::Supports,
            source_entity_id: source_entity_id.into(),
            target_entity_id: target_entity_id.into(),
            scope: KnowledgeScope::Conversation {
                workspace_id: workspace_id.into(),
                conversation_id: conversation_id.into(),
            },
            status: KnowledgeStatus::Confirmed,
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

    #[test]
    fn knowledge_retrieval_combines_full_text_vector_and_relation_after_restart() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Hybrid retrieval".into(),
            })
            .expect("create conversation");
        let primary = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-sqlite",
            "SQLite local persistence",
            KnowledgeStatus::Confirmed,
        );
        let related = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-rebuild",
            "Deterministic index rebuild",
            KnowledgeStatus::Confirmed,
        );
        service
            .upsert_knowledge_entity(&conversation.id, primary)
            .expect("upsert primary");
        service
            .upsert_knowledge_entity(&conversation.id, related)
            .expect("upsert related");
        service
            .upsert_knowledge_relation(
                &conversation.id,
                knowledge_relation(
                    &bootstrap.workspace.id,
                    &conversation.id,
                    "entity-sqlite",
                    "entity-rebuild",
                ),
            )
            .expect("upsert relation");

        let projection = service
            .retrieve_knowledge(&conversation.id, "SQLite", 10)
            .expect("retrieve knowledge");
        let primary_sources = &projection
            .candidates
            .iter()
            .find(|candidate| candidate.entity.id == "entity-sqlite")
            .expect("primary candidate")
            .sources;
        assert!(primary_sources.contains(&crate::domain::KnowledgeRetrievalSource::FullText));
        assert!(primary_sources.contains(&crate::domain::KnowledgeRetrievalSource::Vector));
        assert!(
            projection
                .candidates
                .iter()
                .find(|candidate| candidate.entity.id == "entity-rebuild")
                .expect("relation candidate")
                .sources
                .contains(&crate::domain::KnowledgeRetrievalSource::Relation)
        );

        drop(service);
        let reopened =
            KernelService::open(directory.path().join("mindscape.sqlite3")).expect("reopen kernel");
        assert_eq!(
            reopened
                .retrieve_knowledge(&conversation.id, "SQLite", 10)
                .expect("retrieve after restart"),
            projection
        );
    }

    #[test]
    fn readable_vault_round_trips_evidence_discussion_edits_rejection_and_restart() {
        let (directory, service) = service();
        let vault = crate::adapters::MarkdownVault::new(directory.path().join("vault"))
            .expect("open vault");
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Readable vault".into(),
            })
            .expect("create conversation");
        let evidence = EvidenceRef {
            id: "evidence-vault-1".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-vault-1".into(),
                content_block_index: 0,
            },
            content_hash: Some("sha256:vault".into()),
            excerpt: Some("Persist knowledge as readable Markdown.".into()),
            created_at: "2026-08-30T00:00:00Z".into(),
        };
        service
            .upsert_evidence_ref(&conversation.id, evidence.clone())
            .expect("persist evidence");
        let mut entity = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-vault",
            "Readable Vault Decision",
            KnowledgeStatus::Confirmed,
        );
        entity.evidence = vec![ScopedEvidenceRef {
            id: "scoped-vault-1".into(),
            evidence: evidence.clone(),
            scope: entity.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: entity.generator.clone(),
        }];
        service
            .upsert_knowledge_entity(&conversation.id, entity.clone())
            .expect("persist entity");
        service
            .project_knowledge_entity_markdown(&vault, &conversation.id, &entity.id)
            .expect("project entity");

        let log = DiscussionLog {
            contract_version: DISCUSSION_LOG_CONTRACT_VERSION.into(),
            id: "discussion-vault-1".into(),
            scope: DiscussionLogScope::Conversation {
                workspace_id: bootstrap.workspace.id.clone(),
                conversation_id: conversation.id.clone(),
                focus_frame_id: None,
            },
            title: "Vault implementation discussion".into(),
            body_markdown:
                "## Objective\n\nKeep knowledge portable.\n\n## Next step\n\nVerify restart.".into(),
            related_entity_ids: vec![entity.id.clone()],
            evidence: vec![evidence.clone()],
            revision: 1,
            created_at: "2026-08-30T00:01:00Z".into(),
            updated_at: "2026-08-30T00:01:00Z".into(),
        };
        let first = service
            .project_discussion_log_markdown(&vault, log.clone())
            .expect("project discussion");
        let project_log = DiscussionLog {
            id: "discussion-project-1".into(),
            scope: DiscussionLogScope::Project {
                workspace_id: bootstrap.workspace.id.clone(),
                project_id: "project-mindscape".into(),
            },
            title: "Project Vault timeline".into(),
            evidence: vec![evidence],
            ..log.clone()
        };
        service
            .project_discussion_log_markdown(&vault, project_log)
            .expect("project project discussion");
        assert_eq!(
            service
                .list_project_discussion_logs("project-mindscape")
                .expect("list project discussions")
                .len(),
            1
        );
        let discussion_path = directory.path().join("vault").join(&first.relative_path);
        let edited = std::fs::read_to_string(&discussion_path)
            .expect("read discussion")
            .replace("Verify restart.", "Verify restart and rejection.");
        std::fs::write(&discussion_path, edited).expect("external edit");
        let (second, changed) = service
            .import_discussion_log_edit(&vault, "discussion-vault-1")
            .expect("import discussion edit");
        assert!(changed);
        assert_eq!(second.log.revision, 2);
        let before_conflict =
            std::fs::read_to_string(&discussion_path).expect("current discussion");
        let conflict = service
            .project_discussion_log_markdown(&vault, log)
            .expect_err("stale discussion revision");
        assert!(conflict.to_string().contains("revision conflict"));
        assert_eq!(
            std::fs::read_to_string(&discussion_path).expect("discussion after conflict"),
            before_conflict
        );
        let orphan_log = DiscussionLog {
            id: "discussion-orphan-1".into(),
            scope: DiscussionLogScope::Project {
                workspace_id: "missing-workspace".into(),
                project_id: "project-mindscape".into(),
            },
            revision: 1,
            ..second.log.clone()
        };
        let persistence_error = service
            .project_discussion_log_markdown(&vault, orphan_log)
            .expect_err("reject missing workspace");
        assert!(persistence_error.to_string().contains("FOREIGN KEY"));
        assert!(
            !directory
                .path()
                .join("vault/logs/discussions/discussion-orphan-1.md")
                .exists()
        );

        entity.status = KnowledgeStatus::Rejected;
        entity.revision = 2;
        entity.updated_at = "2026-08-30T00:02:00Z".into();
        service
            .upsert_knowledge_entity(&conversation.id, entity.clone())
            .expect("reject entity");
        service
            .project_knowledge_entity_markdown(&vault, &conversation.id, &entity.id)
            .expect("project rejected entity");
        assert!(
            service
                .retrieve_knowledge(&conversation.id, "Readable Vault Decision", 10)
                .expect("retrieve rejected entity")
                .candidates
                .is_empty()
        );

        drop(service);
        let restored =
            KernelService::open(directory.path().join("mindscape.sqlite3")).expect("reopen kernel");
        assert_eq!(
            restored
                .get_discussion_log("discussion-vault-1")
                .expect("restore discussion"),
            second
        );
        assert!(
            restored
                .retrieve_knowledge(&conversation.id, "Readable Vault Decision", 10)
                .expect("retrieve after restart")
                .candidates
                .is_empty()
        );
        let entity_markdown =
            std::fs::read_to_string(directory.path().join("vault/entities/entity-vault.md"))
                .expect("read rejected entity markdown");
        assert!(entity_markdown.contains("\"rejected\""));
        assert!(
            directory
                .path()
                .join("vault/sources/evidence-vault-1.md")
                .is_file()
        );
        assert!(
            directory
                .path()
                .join("vault/indexes/discussions.md")
                .is_file()
        );

        assert!(
            restored
                .delete_knowledge_entity_and_vault(&vault, &conversation.id, &entity.id)
                .expect("delete entity")
        );
        assert!(
            !directory
                .path()
                .join("vault/entities/entity-vault.md")
                .exists()
        );
    }

    #[test]
    fn discussion_log_sqlite_failure_restores_new_and_reused_evidence_pages() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Discussion compensation".into(),
            })
            .expect("conversation");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        let original_evidence = EvidenceRef {
            id: "evidence-discussion-reused".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-discussion-original".into(),
                content_block_index: 0,
            },
            content_hash: Some("sha256:discussion-original".into()),
            excerpt: Some("Original evidence bytes must survive rollback.".into()),
            created_at: "2026-08-31T00:00:00Z".into(),
        };
        let baseline = DiscussionLog {
            contract_version: DISCUSSION_LOG_CONTRACT_VERSION.into(),
            id: "discussion-compensation-baseline".into(),
            scope: DiscussionLogScope::Conversation {
                workspace_id: bootstrap.workspace.id,
                conversation_id: conversation.id,
                focus_frame_id: None,
            },
            title: "Compensation baseline".into(),
            body_markdown: "## Decision\n\nKeep pre-images for every Vault file.".into(),
            related_entity_ids: vec![],
            evidence: vec![original_evidence.clone()],
            revision: 1,
            created_at: "2026-08-31T00:00:00Z".into(),
            updated_at: "2026-08-31T00:00:00Z".into(),
        };
        service
            .project_discussion_log_markdown(&vault, baseline.clone())
            .expect("baseline projection");
        let reused_source = directory
            .path()
            .join("vault/sources/evidence-discussion-reused.md");
        let index_path = directory.path().join("vault/indexes/discussions.md");
        let original_source_bytes = std::fs::read(&reused_source).expect("baseline source");
        let original_index_bytes = std::fs::read(&index_path).expect("baseline index");
        let mut changed_evidence = original_evidence;
        changed_evidence.excerpt = Some("This overwrite must be rolled back.".into());
        let new_evidence = EvidenceRef {
            id: "evidence-discussion-orphan".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-discussion-orphan".into(),
                content_block_index: 0,
            },
            content_hash: Some("sha256:discussion-orphan".into()),
            excerpt: Some("This new source must be deleted on rollback.".into()),
            created_at: "2026-08-31T00:01:00Z".into(),
        };
        let invalid = DiscussionLog {
            id: "discussion-compensation-orphan".into(),
            scope: DiscussionLogScope::Project {
                workspace_id: "missing-workspace".into(),
                project_id: "project-compensation".into(),
            },
            title: "Rejected projection".into(),
            evidence: vec![changed_evidence, new_evidence],
            ..baseline
        };

        let error = service
            .project_discussion_log_markdown(&vault, invalid)
            .expect_err("foreign key failure");
        assert!(error.to_string().contains("FOREIGN KEY"));
        assert_eq!(
            std::fs::read(reused_source).expect("restored reused source"),
            original_source_bytes
        );
        assert!(
            !directory
                .path()
                .join("vault/sources/evidence-discussion-orphan.md")
                .exists()
        );
        assert!(
            !directory
                .path()
                .join("vault/logs/discussions/discussion-compensation-orphan.md")
                .exists()
        );
        assert_eq!(
            std::fs::read(index_path).expect("restored discussion index"),
            original_index_bytes
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("vault/.discussion-transactions"))
                .expect("discussion transactions")
                .count(),
            0
        );
    }

    #[test]
    fn discussion_log_old_committed_journal_keeps_a_newer_revision_on_restart() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Discussion index recovery".into(),
            })
            .expect("conversation");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        let log = DiscussionLog {
            contract_version: DISCUSSION_LOG_CONTRACT_VERSION.into(),
            id: "discussion-index-recovery".into(),
            scope: DiscussionLogScope::Conversation {
                workspace_id: bootstrap.workspace.id,
                conversation_id: conversation.id,
                focus_frame_id: None,
            },
            title: "Rebuild committed index".into(),
            body_markdown: "## Recovery\n\nSQLite is authoritative after commit.".into(),
            related_entity_ids: vec![],
            evidence: vec![EvidenceRef {
                id: "evidence-index-recovery".into(),
                target: EvidenceTarget::MessageBlock {
                    message_id: "message-index-recovery".into(),
                    content_block_index: 0,
                },
                content_hash: Some("sha256:index-recovery".into()),
                excerpt: Some("The startup rebuild uses the committed SQLite revision.".into()),
                created_at: "2026-08-31T00:00:00Z".into(),
            }],
            revision: 1,
            created_at: "2026-08-31T00:00:00Z".into(),
            updated_at: "2026-08-31T00:00:00Z".into(),
        };
        vault.inject_next_discussion_index_write_failure();

        let error = service
            .project_discussion_log_markdown(&vault, log.clone())
            .expect_err("injected index failure after SQLite commit");
        assert!(error.to_string().contains("injected DiscussionLog index"));
        assert_eq!(
            service
                .get_discussion_log(&log.id)
                .expect("SQLite commit survives")
                .log,
            log
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("vault/.discussion-transactions"))
                .expect("pending transaction")
                .count(),
            1
        );
        let mut latest = log;
        latest.revision = 2;
        latest.title = "Keep the newest committed revision".into();
        latest.body_markdown =
            "## Recovery\n\nAn older committed journal must never roll back revision two.".into();
        latest.updated_at = "2026-08-31T00:01:00Z".into();
        latest.evidence[0].excerpt = Some("Revision two evidence remains authoritative.".into());
        service
            .project_discussion_log_markdown(&vault, latest.clone())
            .expect("commit revision two while revision one journal remains");
        assert_eq!(
            std::fs::read_dir(directory.path().join("vault/.discussion-transactions"))
                .expect("only old transaction remains")
                .count(),
            1
        );

        drop(service);
        let restored = KernelService::open(directory.path().join("mindscape.sqlite3"))
            .expect("restart service");
        let restored_vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        assert_eq!(
            restored
                .recover_discussion_vault(&restored_vault)
                .expect("recover committed discussion"),
            1
        );
        let index = std::fs::read_to_string(directory.path().join("vault/indexes/discussions.md"))
            .expect("rebuilt index");
        assert!(index.contains("Keep the newest committed revision"));
        let markdown = std::fs::read_to_string(
            directory
                .path()
                .join("vault/logs/discussions/discussion-index-recovery.md"),
        )
        .expect("latest discussion markdown");
        assert!(markdown.contains("older committed journal must never roll back revision two"));
        let source = std::fs::read_to_string(
            directory
                .path()
                .join("vault/sources/evidence-index-recovery.md"),
        )
        .expect("latest evidence markdown");
        assert!(source.contains("Revision two evidence remains authoritative."));
        assert_eq!(
            restored
                .get_discussion_log(&latest.id)
                .expect("latest SQLite projection")
                .log,
            latest
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("vault/.discussion-transactions"))
                .expect("clean transactions")
                .count(),
            0
        );
        assert_eq!(
            restored
                .recover_discussion_vault(&restored_vault)
                .expect("idempotent recovery"),
            0
        );
    }

    #[test]
    fn rejected_knowledge_is_removed_from_full_text_and_vector_retrieval() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Rejected knowledge".into(),
            })
            .expect("create conversation");
        let mut entity = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-rejected",
            "Remove rejected knowledge",
            KnowledgeStatus::Confirmed,
        );
        service
            .upsert_knowledge_entity(&conversation.id, entity.clone())
            .expect("upsert confirmed entity");
        entity.status = KnowledgeStatus::Rejected;
        entity.revision = 2;
        entity.updated_at = "2026-08-27T00:01:00Z".into();
        service
            .upsert_knowledge_entity(&conversation.id, entity)
            .expect("reject entity");

        let projection = service
            .retrieve_knowledge(&conversation.id, "rejected knowledge", 10)
            .expect("retrieve after rejection");
        assert!(projection.candidates.is_empty());
    }

    #[test]
    fn corrupt_vector_snapshot_falls_back_and_rebuild_restores_vector_retrieval() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Vector fallback and rebuild".into(),
            })
            .expect("create conversation");
        service
            .upsert_knowledge_entity(
                &conversation.id,
                knowledge_entity(
                    &bootstrap.workspace.id,
                    &conversation.id,
                    "entity-vector-repair",
                    "Repair durable vector snapshot",
                    KnowledgeStatus::Confirmed,
                ),
            )
            .expect("upsert entity");

        let connection =
            Connection::open(directory.path().join("mindscape.sqlite3")).expect("open database");
        connection
            .execute(
                "UPDATE knowledge_vector_records SET record_json = '{invalid-json'
                 WHERE entity_id = 'entity-vector-repair'",
                [],
            )
            .expect("corrupt derived vector record");
        drop(connection);

        let fallback = service
            .retrieve_knowledge(&conversation.id, "Repair durable vector snapshot", 10)
            .expect("fallback retrieval");
        assert_eq!(
            fallback.notice.vector_status,
            crate::domain::KnowledgeRetrievalAvailability::Unavailable
        );
        assert!(fallback.notice.used_fallback);
        let fallback_candidate = fallback
            .candidates
            .iter()
            .find(|candidate| candidate.entity.id == "entity-vector-repair")
            .expect("full-text fallback candidate");
        assert!(
            fallback_candidate
                .sources
                .contains(&crate::domain::KnowledgeRetrievalSource::FullText)
        );
        assert!(
            !fallback_candidate
                .sources
                .contains(&crate::domain::KnowledgeRetrievalSource::Vector)
        );

        assert_eq!(
            service
                .rebuild_knowledge_vector_index(&conversation.id)
                .expect("rebuild vector index"),
            1
        );
        let restored = service
            .retrieve_knowledge(&conversation.id, "Repair durable vector snapshot", 10)
            .expect("restored vector retrieval");
        assert_eq!(
            restored.notice.vector_status,
            crate::domain::KnowledgeRetrievalAvailability::Available
        );
        assert!(!restored.notice.used_fallback);
        assert!(
            restored
                .candidates
                .iter()
                .find(|candidate| candidate.entity.id == "entity-vector-repair")
                .expect("restored vector candidate")
                .sources
                .contains(&crate::domain::KnowledgeRetrievalSource::Vector)
        );
    }

    #[test]
    fn deleting_knowledge_reprojects_vault_and_cleans_sqlite_indexes_after_restart() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Delete knowledge".into(),
            })
            .expect("create conversation");
        let deleted = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete",
            "Delete this knowledge",
            KnowledgeStatus::Confirmed,
        );
        let evidence = EvidenceRef {
            id: "evidence-keep-after-delete".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-keep-after-delete".into(),
                content_block_index: 0,
            },
            content_hash: Some("sha256:keep-after-delete".into()),
            excerpt: Some("The surviving neighbor remains readable.".into()),
            created_at: "2026-08-31T00:00:00Z".into(),
        };
        service
            .upsert_evidence_ref(&conversation.id, evidence.clone())
            .expect("evidence");
        let mut kept = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-keep",
            "Keep this knowledge",
            KnowledgeStatus::Confirmed,
        );
        kept.evidence = vec![ScopedEvidenceRef {
            id: "scoped-keep-after-delete".into(),
            evidence,
            scope: kept.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: kept.generator.clone(),
        }];
        service
            .upsert_knowledge_entity(&conversation.id, deleted.clone())
            .expect("deleted entity");
        service
            .upsert_knowledge_entity(&conversation.id, kept.clone())
            .expect("kept entity");
        let relation = knowledge_relation(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete",
            "entity-keep",
        );
        service
            .upsert_knowledge_relation(&conversation.id, relation.clone())
            .expect("upsert relation");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        vault
            .write_entity_with_relations(&deleted, std::slice::from_ref(&relation))
            .expect("deleted projection");
        vault
            .write_entity_with_relations(&kept, std::slice::from_ref(&relation))
            .expect("kept projection");
        vault
            .write_entity_index(&[deleted.clone(), kept.clone()])
            .expect("entity index");

        assert!(
            service
                .delete_knowledge_entity_and_vault(&vault, &conversation.id, "entity-delete")
                .expect("delete entity")
        );
        assert!(
            service
                .list_knowledge_relations(&conversation.id)
                .expect("list relations")
                .is_empty()
        );
        assert!(
            service
                .retrieve_knowledge(&conversation.id, "Delete this knowledge", 10)
                .expect("retrieve after deletion")
                .candidates
                .iter()
                .all(|candidate| candidate.entity.id != "entity-delete")
        );
        let connection =
            Connection::open(directory.path().join("mindscape.sqlite3")).expect("open database");
        for table in [
            "knowledge_entities_fts",
            "knowledge_vector_records",
            "knowledge_relations",
        ] {
            let count: u64 = connection
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM {table} WHERE {} = ?1",
                        if table == "knowledge_relations" {
                            "id"
                        } else {
                            "entity_id"
                        }
                    ),
                    [if table == "knowledge_relations" {
                        relation.id.as_str()
                    } else {
                        deleted.id.as_str()
                    }],
                    |row| row.get(0),
                )
                .expect("derived row count");
            assert_eq!(count, 0, "{table} retained deleted state");
        }
        drop(connection);
        assert!(
            !directory
                .path()
                .join("vault/entities/entity-delete.md")
                .exists()
        );
        let kept_path = directory.path().join("vault/entities/entity-keep.md");
        assert!(
            !std::fs::read_to_string(&kept_path)
                .expect("kept projection after delete")
                .contains("entity-delete")
        );
        assert!(
            directory
                .path()
                .join("vault/sources/evidence-keep-after-delete.md")
                .is_file()
        );
        assert!(
            !std::fs::read_to_string(directory.path().join("vault/indexes/entities.md"))
                .expect("entity index after delete")
                .contains("entity-delete")
        );

        drop(service);
        let restored = KernelService::open(directory.path().join("mindscape.sqlite3"))
            .expect("restart service");
        let restored_vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        assert_eq!(
            restored
                .recover_knowledge_entity_delete_vault(&restored_vault)
                .expect("restart recovery"),
            0
        );
        assert!(
            !std::fs::read_to_string(kept_path)
                .expect("kept projection after restart")
                .contains("entity-delete")
        );
    }

    #[test]
    fn knowledge_delete_vault_failure_keeps_sqlite_and_files_unchanged() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Delete compensation".into(),
            })
            .expect("conversation");
        let deleted = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete-failure",
            "Deletion must roll back",
            KnowledgeStatus::Confirmed,
        );
        let kept = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete-failure-neighbor",
            "Neighbor must remain byte stable",
            KnowledgeStatus::Confirmed,
        );
        service
            .upsert_knowledge_entity(&conversation.id, deleted.clone())
            .expect("deleted entity");
        service
            .upsert_knowledge_entity(&conversation.id, kept.clone())
            .expect("kept entity");
        let relation = knowledge_relation(
            &bootstrap.workspace.id,
            &conversation.id,
            &deleted.id,
            &kept.id,
        );
        service
            .upsert_knowledge_relation(&conversation.id, relation.clone())
            .expect("relation");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        vault
            .write_entity_with_relations(&deleted, std::slice::from_ref(&relation))
            .expect("deleted projection");
        vault
            .write_entity_with_relations(&kept, std::slice::from_ref(&relation))
            .expect("kept projection");
        vault
            .write_entity_index(&[deleted.clone(), kept.clone()])
            .expect("index");
        let deleted_path = directory
            .path()
            .join("vault/entities/entity-delete-failure.md");
        let kept_path = directory
            .path()
            .join("vault/entities/entity-delete-failure-neighbor.md");
        let index_path = directory.path().join("vault/indexes/entities.md");
        let before_deleted = std::fs::read(&deleted_path).expect("deleted bytes");
        let before_kept = std::fs::read(&kept_path).expect("kept bytes");
        let before_index = std::fs::read(&index_path).expect("index bytes");
        vault.inject_next_entity_index_write_failure();

        let error = service
            .delete_knowledge_entity_and_vault(&vault, &conversation.id, &deleted.id)
            .expect_err("injected Vault write failure");
        assert!(error.to_string().contains("injected entity index"));
        assert!(
            service
                .list_knowledge_entities(&conversation.id)
                .expect("entities after failure")
                .iter()
                .any(|entity| entity.id == deleted.id)
        );
        assert_eq!(
            service
                .list_knowledge_relations(&conversation.id)
                .expect("relations after failure"),
            vec![relation]
        );
        assert!(
            service
                .retrieve_knowledge(&conversation.id, "Deletion must roll back", 10)
                .expect("retrieval after failure")
                .candidates
                .iter()
                .any(|candidate| candidate.entity.id == deleted.id)
        );
        assert_eq!(
            std::fs::read(deleted_path).expect("restored deleted bytes"),
            before_deleted
        );
        assert_eq!(
            std::fs::read(kept_path).expect("restored kept bytes"),
            before_kept
        );
        assert_eq!(
            std::fs::read(index_path).expect("restored index bytes"),
            before_index
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("vault/.entity-delete-transactions"))
                .expect("delete transactions")
                .count(),
            0
        );
    }

    #[test]
    fn committed_knowledge_delete_journal_converges_from_sqlite_on_restart() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Committed delete recovery".into(),
            })
            .expect("conversation");
        let deleted = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete-committed",
            "Committed deleted entity",
            KnowledgeStatus::Confirmed,
        );
        let kept = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete-committed-neighbor",
            "Committed surviving neighbor",
            KnowledgeStatus::Confirmed,
        );
        service
            .upsert_knowledge_entity(&conversation.id, deleted.clone())
            .expect("deleted entity");
        service
            .upsert_knowledge_entity(&conversation.id, kept.clone())
            .expect("kept entity");
        let relation = knowledge_relation(
            &bootstrap.workspace.id,
            &conversation.id,
            &deleted.id,
            &kept.id,
        );
        service
            .upsert_knowledge_relation(&conversation.id, relation.clone())
            .expect("relation");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        vault
            .write_entity_with_relations(&deleted, std::slice::from_ref(&relation))
            .expect("deleted projection");
        vault
            .write_entity_with_relations(&kept, std::slice::from_ref(&relation))
            .expect("kept projection");
        vault
            .write_entity_index(&[deleted.clone(), kept.clone()])
            .expect("index");
        let pending = vault
            .apply_knowledge_entity_delete(&deleted.id, &[&kept], std::slice::from_ref(&kept), &[])
            .expect("apply delete Vault transaction");
        assert!(
            service
                .store
                .delete_knowledge_entity(&conversation.id, &deleted.id)
                .expect("commit SQLite delete")
        );
        drop(pending);
        assert_eq!(
            std::fs::read_dir(directory.path().join("vault/.entity-delete-transactions"))
                .expect("pending committed transaction")
                .count(),
            1
        );

        drop(service);
        let restored = KernelService::open(directory.path().join("mindscape.sqlite3"))
            .expect("restart service");
        let restored_vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        assert_eq!(
            restored
                .recover_knowledge_entity_delete_vault(&restored_vault)
                .expect("recover committed delete"),
            1
        );
        assert!(
            restored
                .list_knowledge_relations(&conversation.id)
                .expect("relations after restart")
                .is_empty()
        );
        assert!(
            !directory
                .path()
                .join("vault/entities/entity-delete-committed.md")
                .exists()
        );
        assert!(
            !std::fs::read_to_string(
                directory
                    .path()
                    .join("vault/entities/entity-delete-committed-neighbor.md"),
            )
            .expect("neighbor after restart")
            .contains("[[entity-delete-committed|")
        );
        assert!(
            !std::fs::read_to_string(directory.path().join("vault/indexes/entities.md"))
                .expect("index after restart")
                .contains("[[../entities/entity-delete-committed|")
        );
        assert_eq!(
            std::fs::read_dir(directory.path().join("vault/.entity-delete-transactions"))
                .expect("clean transactions")
                .count(),
            0
        );
    }

    #[test]
    fn focus_frame_service_round_trips_query_transitions_and_restart_recovery() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Focus lifecycle service".into(),
            })
            .expect("create conversation");

        let created = service
            .create_focus_frame(focus_frame(&conversation.id))
            .expect("create focus frame");
        assert_eq!(
            created.status,
            crate::domain::FocusFrameLifecycleStatus::Active
        );
        assert_eq!(created.revision, 1);

        let queried = service
            .get_focus_frame_query(&created.frame.id)
            .expect("query focus frame");
        assert_eq!(queried.lifecycle, created);
        assert!(queried.focused_context.is_none());
        let listed = service
            .list_focus_frame_queries(&conversation.id)
            .expect("list focus frames");
        assert_eq!(listed, vec![queried.clone()]);

        let closed = service
            .close_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: created.frame.id.clone(),
                expected_revision: 1,
                updated_at: "2026-08-25T15:31:00Z".into(),
            })
            .expect("close focus frame");
        assert_eq!(
            closed.lifecycle.status,
            crate::domain::FocusFrameLifecycleStatus::Closed
        );
        assert_eq!(closed.lifecycle.revision, 2);

        let stale = service
            .reopen_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: created.frame.id.clone(),
                expected_revision: 1,
                updated_at: "2026-08-25T15:32:00Z".into(),
            })
            .expect_err("stale revision must be rejected");
        assert!(
            matches!(stale, KernelError::Integrity(message) if message.contains("revision conflict"))
        );

        let reopened = service
            .reopen_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: created.frame.id.clone(),
                expected_revision: 2,
                updated_at: "2026-08-25T15:33:00Z".into(),
            })
            .expect("reopen focus frame");
        assert_eq!(
            reopened.lifecycle.status,
            crate::domain::FocusFrameLifecycleStatus::Active
        );
        assert_eq!(reopened.lifecycle.revision, 3);

        drop(service);
        let restored = KernelService::open(directory.path().join("mindscape.sqlite3"))
            .expect("reopen kernel service");
        let restored_query = restored
            .get_focus_frame_query(&created.frame.id)
            .expect("query restored focus frame");
        assert_eq!(restored_query.lifecycle, reopened.lifecycle);
    }

    #[test]
    fn promotion_candidate_query_restores_from_sqlite_and_rejects_stale_memory_versions() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Promotion candidate recovery".into(),
            })
            .expect("create conversation");

        let mut branch = focus_frame(&conversation.id);
        branch.memory_version = 4;
        branch.memory_scope.promote_refs = vec!["entity-result-1".into()];
        service
            .create_focus_frame(branch.clone())
            .expect("create branch frame");
        service
            .close_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: branch.id.clone(),
                expected_revision: 1,
                updated_at: "2026-08-29T05:40:00Z".into(),
            })
            .expect("close branch before promotion");
        let candidates = service
            .get_focus_promotion_candidates(&branch.id, Some(4))
            .expect("query candidates")
            .expect("candidate set");
        assert_eq!(candidates.focus_frame_id, branch.id);
        assert_eq!(candidates.memory_version, 4);
        assert_eq!(candidates.candidate_refs, ["entity-result-1"]);

        drop(service);
        let restored = KernelService::open(directory.path().join("mindscape.sqlite3"))
            .expect("reopen kernel service");
        assert_eq!(
            restored
                .get_focus_promotion_candidates(&branch.id, Some(4))
                .expect("query restored candidates"),
            Some(candidates)
        );
        let stale = restored
            .get_focus_promotion_candidates(&branch.id, Some(3))
            .expect_err("stale memory version");
        assert!(
            matches!(stale, KernelError::Integrity(message) if message.contains("memory version conflict"))
        );
    }

    #[test]
    fn focus_promotion_decision_updates_vault_filters_reclose_and_survives_restart() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Atomic decision recovery".into(),
            })
            .expect("create conversation");
        let mut branch = focus_frame(&conversation.id);
        branch.memory_version = 4;
        branch.memory_scope.promote_refs = vec!["entity-result-1".into()];
        service
            .create_focus_frame(branch.clone())
            .expect("create branch");
        service
            .close_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: branch.id.clone(),
                expected_revision: 1,
                updated_at: "2026-08-31T01:00:00Z".into(),
            })
            .expect("close branch");
        let mut candidate = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-result-1",
            "Persist the verified branch result",
            KnowledgeStatus::Candidate,
        );
        candidate.scope = KnowledgeScope::FocusFrame {
            workspace_id: bootstrap.workspace.id,
            conversation_id: conversation.id.clone(),
            focus_frame_id: branch.id.clone(),
        };
        service
            .upsert_knowledge_entity(&conversation.id, candidate)
            .expect("candidate entity");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        let input = FocusPromotionDecisionCommandInput {
            decision_id: "decision-service-1".into(),
            focus_frame_id: branch.id.clone(),
            candidate_ref: "entity-result-1".into(),
            expected_memory_version: 4,
            expected_lifecycle_revision: 2,
            expected_entity_revision: 1,
            expected_decision_revision: 0,
            action: FocusPromotionDecisionAction::Confirm,
            target_scope: None,
            promoted_entity_id: None,
            decided_at: "2026-08-31T02:00:00Z".into(),
        };

        let decision = service
            .decide_focus_promotion(&vault, input.clone())
            .expect("confirm candidate");
        assert_eq!(decision.source_entity_revision, Some(2));
        let markdown =
            std::fs::read_to_string(directory.path().join("vault/entities/entity-result-1.md"))
                .expect("projected entity");
        assert!(markdown.contains("status: \"confirmed\""));
        assert_eq!(
            service
                .get_focus_promotion_candidates(&branch.id, Some(4))
                .expect("filtered candidates")
                .expect("closed candidate set")
                .candidate_refs,
            Vec::<String>::new()
        );

        service
            .reopen_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: branch.id.clone(),
                expected_revision: 2,
                updated_at: "2026-08-31T03:00:00Z".into(),
            })
            .expect("reopen");
        service
            .close_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: branch.id.clone(),
                expected_revision: 3,
                updated_at: "2026-08-31T04:00:00Z".into(),
            })
            .expect("reclose");
        assert!(
            service
                .get_focus_promotion_candidates(&branch.id, Some(4))
                .expect("reclosed candidates")
                .expect("closed set")
                .candidate_refs
                .is_empty()
        );

        std::fs::remove_file(directory.path().join("vault/entities/entity-result-1.md"))
            .expect("simulate missing projection");
        assert_eq!(
            service
                .decide_focus_promotion(&vault, input)
                .expect("idempotent replay repairs Vault"),
            decision
        );
        assert!(
            directory
                .path()
                .join("vault/entities/entity-result-1.md")
                .is_file()
        );

        drop(service);
        let restored = KernelService::open(directory.path().join("mindscape.sqlite3"))
            .expect("restart service");
        assert_eq!(
            restored
                .get_focus_promotion_decision(&decision.decision_id)
                .expect("restart decision"),
            decision
        );
        assert!(
            restored
                .get_focus_promotion_candidates(&branch.id, Some(4))
                .expect("restart candidates")
                .expect("closed set")
                .candidate_refs
                .is_empty()
        );
    }

    #[test]
    fn focus_promotion_delete_reprojects_surviving_relation_neighbors_on_replay_and_restart() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Delete relation projection".into(),
            })
            .expect("create conversation");
        let mut branch = focus_frame(&conversation.id);
        branch.memory_scope.promote_refs = vec!["entity-delete-candidate".into()];
        service
            .create_focus_frame(branch.clone())
            .expect("create branch");
        service
            .close_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: branch.id.clone(),
                expected_revision: 1,
                updated_at: "2026-08-31T01:00:00Z".into(),
            })
            .expect("close branch");
        let mut candidate = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete-candidate",
            "Disposable branch candidate",
            KnowledgeStatus::Candidate,
        );
        candidate.scope = KnowledgeScope::FocusFrame {
            workspace_id: bootstrap.workspace.id.clone(),
            conversation_id: conversation.id.clone(),
            focus_frame_id: branch.id.clone(),
        };
        let evidence = EvidenceRef {
            id: "evidence-delete-neighbor".into(),
            target: EvidenceTarget::MessageBlock {
                message_id: "message-delete-neighbor".into(),
                content_block_index: 0,
            },
            content_hash: Some("sha256:delete-neighbor".into()),
            excerpt: Some("The surviving entity keeps its source page.".into()),
            created_at: "2026-08-31T00:00:00Z".into(),
        };
        service
            .upsert_evidence_ref(&conversation.id, evidence.clone())
            .expect("persist evidence");
        let mut survivor = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-delete-survivor",
            "Surviving relation neighbor",
            KnowledgeStatus::Confirmed,
        );
        survivor.evidence = vec![ScopedEvidenceRef {
            id: "scoped-delete-neighbor".into(),
            evidence,
            scope: survivor.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            generator: survivor.generator.clone(),
        }];
        service
            .upsert_knowledge_entity(&conversation.id, candidate.clone())
            .expect("candidate");
        service
            .upsert_knowledge_entity(&conversation.id, survivor.clone())
            .expect("survivor");
        let relation = KnowledgeRelation {
            contract_version: KNOWLEDGE_CONTRACT_VERSION.into(),
            id: "relation-delete-neighbor".into(),
            kind: KnowledgeRelationKind::Supports,
            source_entity_id: survivor.id.clone(),
            target_entity_id: candidate.id.clone(),
            scope: survivor.scope.clone(),
            status: KnowledgeStatus::Confirmed,
            revision: 1,
            evidence: vec![],
            generator: survivor.generator.clone(),
            created_at: "2026-08-31T00:00:00Z".into(),
            updated_at: "2026-08-31T00:00:00Z".into(),
        };
        service
            .upsert_knowledge_relation(&conversation.id, relation.clone())
            .expect("relation");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        vault
            .write_entity_with_relations(&candidate, std::slice::from_ref(&relation))
            .expect("candidate projection");
        vault
            .write_entity_with_relations(&survivor, std::slice::from_ref(&relation))
            .expect("survivor projection");
        let survivor_path = directory
            .path()
            .join("vault/entities/entity-delete-survivor.md");
        assert!(
            std::fs::read_to_string(&survivor_path)
                .expect("linked survivor")
                .contains("entity-delete-candidate")
        );
        let input = FocusPromotionDecisionCommandInput {
            decision_id: "decision-delete-neighbor".into(),
            focus_frame_id: branch.id,
            candidate_ref: candidate.id.clone(),
            expected_memory_version: 1,
            expected_lifecycle_revision: 2,
            expected_entity_revision: 1,
            expected_decision_revision: 0,
            action: FocusPromotionDecisionAction::Delete,
            target_scope: None,
            promoted_entity_id: None,
            decided_at: "2026-08-31T02:00:00Z".into(),
        };

        let decision = service
            .decide_focus_promotion(&vault, input.clone())
            .expect("delete candidate");
        assert!(
            service
                .list_knowledge_relations(&conversation.id)
                .expect("relations after delete")
                .is_empty()
        );
        assert!(
            !directory
                .path()
                .join("vault/entities/entity-delete-candidate.md")
                .exists()
        );
        assert!(
            !std::fs::read_to_string(&survivor_path)
                .expect("survivor after delete")
                .contains("entity-delete-candidate")
        );
        assert!(
            directory
                .path()
                .join("vault/sources/evidence-delete-neighbor.md")
                .is_file()
        );

        std::fs::write(&survivor_path, "stale [[entity-delete-candidate]]")
            .expect("simulate stale survivor projection");
        assert_eq!(
            service
                .decide_focus_promotion(&vault, input.clone())
                .expect("idempotent replay repairs neighbors"),
            decision
        );
        assert!(
            !std::fs::read_to_string(&survivor_path)
                .expect("repaired survivor")
                .contains("entity-delete-candidate")
        );

        drop(service);
        let restored = KernelService::open(directory.path().join("mindscape.sqlite3"))
            .expect("restart service");
        assert_eq!(
            restored
                .decide_focus_promotion(&vault, input)
                .expect("restart replay remains consistent"),
            decision
        );
        assert!(
            !std::fs::read_to_string(survivor_path)
                .expect("restart survivor")
                .contains("entity-delete-candidate")
        );
    }

    #[test]
    fn focus_promotion_sqlite_conflict_restores_all_vault_files() {
        let (directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id.clone(),
                title: "Vault compensation".into(),
            })
            .expect("create conversation");
        let mut branch = focus_frame(&conversation.id);
        branch.memory_scope.promote_refs = vec!["entity-result-1".into()];
        service
            .create_focus_frame(branch.clone())
            .expect("create branch");
        service
            .close_focus_frame(FocusFrameLifecycleCommandInput {
                focus_frame_id: branch.id.clone(),
                expected_revision: 1,
                updated_at: "2026-08-31T01:00:00Z".into(),
            })
            .expect("close branch");
        let mut candidate = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-result-1",
            "Candidate before failed promote",
            KnowledgeStatus::Candidate,
        );
        candidate.scope = KnowledgeScope::FocusFrame {
            workspace_id: bootstrap.workspace.id.clone(),
            conversation_id: conversation.id.clone(),
            focus_frame_id: branch.id.clone(),
        };
        let existing_target = knowledge_entity(
            &bootstrap.workspace.id,
            &conversation.id,
            "entity-existing-target",
            "Existing target must survive",
            KnowledgeStatus::Confirmed,
        );
        service
            .upsert_knowledge_entity(&conversation.id, candidate.clone())
            .expect("candidate");
        service
            .upsert_knowledge_entity(&conversation.id, existing_target.clone())
            .expect("existing target");
        let vault =
            crate::adapters::MarkdownVault::new(directory.path().join("vault")).expect("vault");
        vault
            .write_entity(&candidate)
            .expect("candidate projection");
        vault
            .write_entity(&existing_target)
            .expect("target projection");
        vault
            .write_entity_index(&[candidate.clone(), existing_target.clone()])
            .expect("index");
        let source_path = directory.path().join("vault/entities/entity-result-1.md");
        let target_path = directory
            .path()
            .join("vault/entities/entity-existing-target.md");
        let index_path = directory.path().join("vault/indexes/entities.md");
        let before_source = std::fs::read(&source_path).expect("source bytes");
        let before_target = std::fs::read(&target_path).expect("target bytes");
        let before_index = std::fs::read(&index_path).expect("index bytes");
        let input = FocusPromotionDecisionCommandInput {
            decision_id: "decision-conflict-1".into(),
            focus_frame_id: branch.id.clone(),
            candidate_ref: candidate.id.clone(),
            expected_memory_version: 1,
            expected_lifecycle_revision: 2,
            expected_entity_revision: 1,
            expected_decision_revision: 0,
            action: FocusPromotionDecisionAction::Promote,
            target_scope: Some(crate::domain::FocusPromotionTargetScope::Conversation {
                workspace_id: bootstrap.workspace.id,
                conversation_id: conversation.id.clone(),
            }),
            promoted_entity_id: Some(existing_target.id.clone()),
            decided_at: "2026-08-31T02:00:00Z".into(),
        };

        service
            .decide_focus_promotion(&vault, input)
            .expect_err("target conflict rolls back");
        assert_eq!(
            std::fs::read(source_path).expect("source restored"),
            before_source
        );
        assert_eq!(
            std::fs::read(target_path).expect("target restored"),
            before_target
        );
        assert_eq!(
            std::fs::read(index_path).expect("index restored"),
            before_index
        );
        assert_eq!(
            service
                .list_knowledge_entities(&conversation.id)
                .expect("entities"),
            vec![existing_target, candidate]
        );
        assert!(
            service
                .list_focus_promotion_decisions(&branch.id)
                .expect("decisions")
                .is_empty()
        );
    }

    #[test]
    fn promotion_candidate_query_returns_empty_until_branch_is_closed() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Active promotion declaration".into(),
            })
            .expect("create conversation");

        let mut branch = focus_frame(&conversation.id);
        branch.memory_scope.promote_refs = vec!["entity-result-1".into()];
        service
            .create_focus_frame(branch.clone())
            .expect("create active branch");

        let candidates = service
            .get_focus_promotion_candidates(&branch.id, Some(1))
            .expect("query active branch");

        assert_eq!(candidates, None);
    }

    #[test]
    fn promotion_candidate_query_returns_empty_for_mainline_or_undeclared_branch() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Promotion candidate empty states".into(),
            })
            .expect("create conversation");

        let mut mainline = focus_frame(&conversation.id);
        mainline.id = "focus-mainline".into();
        mainline.memory_scope.branch_kind = FocusBranchKind::Mainline;
        service
            .create_focus_frame(mainline.clone())
            .expect("create mainline frame");
        assert_eq!(
            service
                .get_focus_promotion_candidates(&mainline.id, Some(1))
                .expect("query mainline"),
            None
        );

        let mut branch = focus_frame(&conversation.id);
        branch.id = "focus-empty-branch".into();
        service
            .create_focus_frame(branch.clone())
            .expect("create empty branch frame");
        assert_eq!(
            service
                .get_focus_promotion_candidates(&branch.id, None)
                .expect("query empty branch"),
            None
        );
    }

    #[test]
    fn create_focus_frame_rejects_invalid_definition_before_persisting() {
        let (_directory, service) = service();
        let bootstrap = service.bootstrap().expect("bootstrap");
        let conversation = service
            .create_conversation(CreateConversationInput {
                workspace_id: bootstrap.workspace.id,
                title: "Invalid focus lifecycle".into(),
            })
            .expect("create conversation");
        let mut invalid = focus_frame(&conversation.id);
        invalid.id.clear();

        let error = service
            .create_focus_frame(invalid)
            .expect_err("invalid FocusFrame must be rejected");
        assert!(
            matches!(error, KernelError::Validation(message) if message.contains("id must not be empty"))
        );
        assert!(matches!(
            service.get_focus_frame_query("focus-service-1"),
            Err(KernelError::NotFound { .. })
        ));
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
                    effective_run_profile: None,
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
                    effective_run_profile: None,
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
            effective_run_profile: None,
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
            effective_run_profile: None,
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

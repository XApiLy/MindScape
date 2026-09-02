use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use serde::Serialize;
use tauri::{State, ipc::Channel};

use crate::{
    adapters::{
        CredentialService, ImportStorage, MarkdownVault, SemanticEmbedding,
        SemanticModelInstallError, SemanticModelPack, SemanticModelPackStatus,
        parse_generic_import,
        provider::{
            DeterministicImportKnowledgeSuggestionProducer, MockProvider, OpenAiCompatibleConfig,
            OpenAiCompatibleProvider, ProviderConnectionTestResult, ProviderDescriptor,
            ProviderRegistry, ProviderRuntime, ProviderRuntimeError, RunCancellation,
        },
    },
    application::KernelService,
    domain::{
        AppendTurnInput, CanvasViewportState, CompleteTurnInput, ContextSnapshot, Conversation,
        ConversationGraph, ConversationNode, CreateConversationInput, CredentialError,
        CredentialRef, FocusFrameLifecycleCommandInput, FocusFrameLifecycleSnapshot,
        FocusFrameQueryProjection, FocusPromotionCandidateGenerationCommandInput,
        FocusPromotionCandidateGenerationProjection, FocusPromotionDecisionCommandInput,
        FocusPromotionDecisionProjection, FocusedContextSnapshot,
        ImportKnowledgeProposalBatchProjection, ImportKnowledgeProposalDiscoveryProjection,
        ImportKnowledgeProposalDiscoveryQuery, ImportKnowledgeProposalRequestInput,
        ImportKnowledgeProposalReviewCommandInput, ImportKnowledgeProposalReviewProjection,
        KernelBootstrap, KernelError, RunState, SaveCanvasViewportInput, SetCredentialInput,
        StartModelRunInput, UpdateNodePositionInput,
        contracts::{
            DiscussionLog, DiscussionLogProjection, FocusFrame, FocusPromotionCandidateSet,
            ImportRevision, ImportSource, ImportedMessage, ModelRunEvent, ModelRunEventEnvelope,
            ModelRunProjection, ModelRunRequest, ParseReport, ProviderError, ProviderErrorCategory,
        },
        new_id, now_timestamp,
    },
};

#[derive(Clone)]
pub struct KernelState {
    service: KernelService,
    import_storage: ImportStorage,
    markdown_vault: MarkdownVault,
    credentials: CredentialService,
    provider_runtime: ProviderRuntime,
    import_knowledge_suggestion_producer: DeterministicImportKnowledgeSuggestionProducer,
    active_runs: Arc<Mutex<HashMap<String, RunCancellation>>>,
    semantic_model_pack: SemanticModelPack,
    semantic_model_installing: Arc<AtomicBool>,
    semantic_embedding: Arc<Mutex<Option<Arc<SemanticEmbedding>>>>,
}

impl KernelState {
    pub fn new(
        service: KernelService,
        credentials: CredentialService,
        import_storage: ImportStorage,
        markdown_vault: MarkdownVault,
        semantic_model_pack: SemanticModelPack,
    ) -> Self {
        let mut registry = ProviderRegistry::default();
        registry
            .register(MockProvider::standard())
            .expect("the built-in mock provider must register once");
        registry
            .register(
                OpenAiCompatibleProvider::new(
                    OpenAiCompatibleConfig::deepseek(),
                    credentials.clone(),
                )
                .expect("the built-in DeepSeek provider configuration must be valid"),
            )
            .expect("the built-in DeepSeek provider must register once");
        Self {
            service,
            import_storage,
            markdown_vault,
            credentials,
            provider_runtime: ProviderRuntime::new(registry),
            import_knowledge_suggestion_producer: DeterministicImportKnowledgeSuggestionProducer,
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            semantic_model_pack,
            semantic_model_installing: Arc::new(AtomicBool::new(false)),
            semantic_embedding: Arc::new(Mutex::new(None)),
        }
    }

    fn semantic_embedding(&self) -> Option<Arc<SemanticEmbedding>> {
        let mut cached = self.semantic_embedding.lock().ok()?;
        if cached.is_none() {
            *cached = SemanticEmbedding::load(&self.semantic_model_pack)
                .ok()
                .map(Arc::new);
        }
        cached.clone()
    }
}

struct InstallFlagGuard(Arc<AtomicBool>);

impl Drop for InstallFlagGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub fn get_semantic_model_pack_status(state: State<'_, KernelState>) -> SemanticModelPackStatus {
    state.semantic_model_pack.inspect()
}

#[tauri::command]
pub async fn install_semantic_model_pack(
    state: State<'_, KernelState>,
) -> CommandResult<SemanticModelPackStatus> {
    if state
        .semantic_model_installing
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(CommandError {
            code: "semanticModelInstallInProgress",
            safe_message: "The semantic model pack is already being installed.".into(),
            retryable: true,
        });
    }
    let _guard = InstallFlagGuard(Arc::clone(&state.semantic_model_installing));
    let status = state
        .semantic_model_pack
        .install()
        .await
        .map_err(semantic_model_install_error)?;
    if let Ok(mut cached) = state.semantic_embedding.lock() {
        *cached = None;
    }
    Ok(status)
}

fn semantic_model_install_error(error: SemanticModelInstallError) -> CommandError {
    match error {
        SemanticModelInstallError::Download(_)
        | SemanticModelInstallError::AllSourcesUnavailable(_) => CommandError {
            code: "semanticModelDownloadFailed",
            safe_message:
                "The semantic model pack could not be downloaded. Check the network and retry."
                    .into(),
            retryable: true,
        },
        SemanticModelInstallError::Storage(_) => CommandError {
            code: "semanticModelStorageFailed",
            safe_message: "The semantic model pack could not be written to local storage.".into(),
            retryable: true,
        },
        SemanticModelInstallError::SizeLimit(_)
        | SemanticModelInstallError::Integrity
        | SemanticModelInstallError::Archive(_)
        | SemanticModelInstallError::InstallerTask(_) => CommandError {
            code: "semanticModelIntegrityFailed",
            safe_message: "The downloaded semantic model pack failed integrity verification."
                .into(),
            retryable: true,
        },
    }
}

#[tauri::command]
pub fn project_knowledge_entity_markdown(
    state: State<'_, KernelState>,
    conversation_id: String,
    entity_id: String,
) -> CommandResult<crate::domain::contracts::MarkdownProjection> {
    state
        .service
        .project_knowledge_entity_markdown(&state.markdown_vault, &conversation_id, &entity_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_markdown_projections(
    state: State<'_, KernelState>,
    entity_id: String,
) -> CommandResult<Vec<crate::domain::contracts::MarkdownProjection>> {
    state
        .service
        .list_markdown_projections(&entity_id)
        .map_err(Into::into)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownEditCommandResult {
    pub projection: crate::domain::contracts::MarkdownProjection,
    pub changed: bool,
}

#[tauri::command]
pub fn import_markdown_entity_edit(
    state: State<'_, KernelState>,
    conversation_id: String,
    entity_id: String,
) -> CommandResult<MarkdownEditCommandResult> {
    let (projection, changed) = state.service.import_markdown_entity_edit(
        &state.markdown_vault,
        &conversation_id,
        &entity_id,
    )?;
    Ok(MarkdownEditCommandResult {
        projection,
        changed,
    })
}

#[tauri::command]
pub fn project_discussion_log_markdown(
    state: State<'_, KernelState>,
    log: DiscussionLog,
) -> CommandResult<DiscussionLogProjection> {
    state
        .service
        .project_discussion_log_markdown(&state.markdown_vault, log)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_discussion_log(
    state: State<'_, KernelState>,
    discussion_log_id: String,
) -> CommandResult<DiscussionLogProjection> {
    state
        .service
        .get_discussion_log(&discussion_log_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_conversation_discussion_logs(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<Vec<DiscussionLogProjection>> {
    state
        .service
        .list_conversation_discussion_logs(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_project_discussion_logs(
    state: State<'_, KernelState>,
    project_id: String,
) -> CommandResult<Vec<DiscussionLogProjection>> {
    state
        .service
        .list_project_discussion_logs(&project_id)
        .map_err(Into::into)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionLogEditCommandResult {
    pub projection: DiscussionLogProjection,
    pub changed: bool,
}

#[tauri::command]
pub fn import_discussion_log_edit(
    state: State<'_, KernelState>,
    discussion_log_id: String,
) -> CommandResult<DiscussionLogEditCommandResult> {
    let (projection, changed) = state
        .service
        .import_discussion_log_edit(&state.markdown_vault, &discussion_log_id)?;
    Ok(DiscussionLogEditCommandResult {
        projection,
        changed,
    })
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportCommandResult {
    pub source: ImportSource,
    pub revision: ImportRevision,
    pub report: ParseReport,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RawImportContentProjection {
    pub source_id: String,
    pub content_hash: String,
    pub byte_length: u64,
    pub content: String,
    pub truncated: bool,
}

type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub safe_message: String,
    pub retryable: bool,
}

impl From<KernelError> for CommandError {
    fn from(error: KernelError) -> Self {
        match error {
            KernelError::NotFound { entity, id } => Self {
                code: "notFound",
                safe_message: format!("{entity} not found: {id}"),
                retryable: false,
            },
            KernelError::Validation(message) => Self {
                code: "validation",
                safe_message: message,
                retryable: false,
            },
            KernelError::Integrity(message) => Self {
                code: "integrity",
                safe_message: message,
                retryable: false,
            },
            KernelError::Database(_) | KernelError::Serialization(_) | KernelError::Io(_) => Self {
                code: "storageUnavailable",
                safe_message: "Local data operation failed. Retry or restart MindScape.".into(),
                retryable: true,
            },
        }
    }
}

impl From<CredentialError> for CommandError {
    fn from(error: CredentialError) -> Self {
        match error {
            CredentialError::InvalidReference(message) => Self {
                code: "validation",
                safe_message: message,
                retryable: false,
            },
            CredentialError::NotFound => Self {
                code: "credentialNotFound",
                safe_message: "No credential is configured for this provider account.".into(),
                retryable: false,
            },
            CredentialError::Unavailable => Self {
                code: "credentialStoreUnavailable",
                safe_message: "The operating system credential store is unavailable.".into(),
                retryable: true,
            },
        }
    }
}

impl From<ProviderRuntimeError> for CommandError {
    fn from(error: ProviderRuntimeError) -> Self {
        match error {
            ProviderRuntimeError::Provider(error) => Self::from(error),
            ProviderRuntimeError::ProviderNotRegistered(_)
            | ProviderRuntimeError::ModelNotRegistered { .. }
            | ProviderRuntimeError::CapabilityUnsupported { .. } => Self {
                code: "providerConfiguration",
                safe_message: error.to_string(),
                retryable: false,
            },
        }
    }
}

impl From<ProviderError> for CommandError {
    fn from(error: ProviderError) -> Self {
        let code = match error.category {
            ProviderErrorCategory::Authentication => "providerAuthentication",
            ProviderErrorCategory::RateLimit => "providerRateLimit",
            ProviderErrorCategory::InsufficientBalance => "providerBalance",
            ProviderErrorCategory::ModelUnavailable => "providerModelUnavailable",
            ProviderErrorCategory::InvalidRequest => "providerInvalidRequest",
            ProviderErrorCategory::Network => "providerNetwork",
            ProviderErrorCategory::Timeout => "providerTimeout",
            ProviderErrorCategory::ContentPolicy => "providerContentPolicy",
            ProviderErrorCategory::Cancelled => "providerCancelled",
            ProviderErrorCategory::Unknown => "providerUnknown",
        };
        Self {
            code,
            safe_message: error.safe_message,
            retryable: error.retryable,
        }
    }
}

#[tauri::command]
pub fn bootstrap_kernel(state: State<'_, KernelState>) -> CommandResult<KernelBootstrap> {
    state.service.bootstrap().map_err(Into::into)
}

#[tauri::command]
pub fn persist_import_bundle(
    state: State<'_, KernelState>,
    source: ImportSource,
    revision: ImportRevision,
    messages: Vec<ImportedMessage>,
    report: ParseReport,
) -> CommandResult<()> {
    state
        .service
        .persist_import_bundle(&source, &revision, &messages, &report)
        .map_err(Into::into)
}

#[tauri::command]
pub fn import_generic_file(
    state: State<'_, KernelState>,
    conversation_id: String,
    original_file_name: String,
    payload: Vec<u8>,
) -> CommandResult<GenericImportCommandResult> {
    if conversation_id.trim().is_empty() {
        return Err(CommandError {
            code: "validation",
            safe_message: "Conversation id is required.".into(),
            retryable: false,
        });
    }
    let stored = state
        .import_storage
        .store(&original_file_name, &payload)
        .map_err(|error| CommandError {
            code: "validation",
            safe_message: error.to_string(),
            retryable: false,
        })?;
    let source = ImportSource {
        id: new_id("import-source"),
        conversation_id,
        platform: crate::domain::contracts::ImportPlatform::Generic,
        original_file_name: Some(original_file_name),
        content_hash: stored.content_hash.clone(),
        storage_ref: stored.storage_ref.clone(),
        created_at: now_timestamp(),
    };
    let revision_id = new_id("import-revision");
    let bundle = match parse_generic_import(source, revision_id, stored.format, &payload) {
        Ok(bundle) => bundle,
        Err(error) => {
            let _ = state.import_storage.discard_if_new(&stored);
            return Err(CommandError {
                code: "importParse",
                safe_message: error.to_string(),
                retryable: true,
            });
        }
    };
    let result = GenericImportCommandResult {
        source: bundle.source.clone(),
        revision: bundle.revision.clone(),
        report: bundle.report.clone(),
        duplicate: stored.duplicate,
    };
    if let Err(error) = state.service.persist_import_bundle(
        &bundle.source,
        &bundle.revision,
        &bundle.messages,
        &bundle.report,
    ) {
        let _ = state.import_storage.discard_if_new(&stored);
        return Err(error.into());
    }
    Ok(result)
}

#[tauri::command]
pub fn list_import_sources(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<Vec<ImportSource>> {
    state
        .service
        .list_import_sources(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_import_bundle(
    state: State<'_, KernelState>,
    source_id: String,
) -> CommandResult<crate::domain::contracts::ImportBundleQueryProjection> {
    state
        .service
        .get_import_bundle(&source_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_raw_import_content(
    state: State<'_, KernelState>,
    source_id: String,
) -> CommandResult<RawImportContentProjection> {
    let bundle = state.service.get_import_bundle(&source_id)?;
    let (content, byte_length, truncated) = state
        .import_storage
        .read_verified_text(
            &bundle.source.storage_ref,
            &bundle.source.content_hash,
            1024 * 1024,
        )
        .map_err(|error| CommandError {
            code: "importStorage",
            safe_message: error.to_string(),
            retryable: false,
        })?;
    Ok(RawImportContentProjection {
        source_id: bundle.source.id,
        content_hash: bundle.source.content_hash,
        byte_length,
        content,
        truncated,
    })
}

#[tauri::command]
pub fn request_import_knowledge_proposals(
    state: State<'_, KernelState>,
    input: ImportKnowledgeProposalRequestInput,
) -> CommandResult<ImportKnowledgeProposalBatchProjection> {
    state
        .service
        .request_import_knowledge_proposals(input, &state.import_knowledge_suggestion_producer)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_import_knowledge_proposal_batch(
    state: State<'_, KernelState>,
    request_id: String,
) -> CommandResult<ImportKnowledgeProposalBatchProjection> {
    state
        .service
        .get_import_knowledge_proposal_batch(&request_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn discover_import_knowledge_proposals(
    state: State<'_, KernelState>,
    query: ImportKnowledgeProposalDiscoveryQuery,
) -> CommandResult<ImportKnowledgeProposalDiscoveryProjection> {
    state
        .service
        .discover_import_knowledge_proposals(query)
        .map_err(Into::into)
}

#[tauri::command]
pub fn review_import_knowledge_proposal(
    state: State<'_, KernelState>,
    input: ImportKnowledgeProposalReviewCommandInput,
) -> CommandResult<ImportKnowledgeProposalReviewProjection> {
    state
        .service
        .review_import_knowledge_proposal(&state.markdown_vault, input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_import_knowledge_proposal_reviews(
    state: State<'_, KernelState>,
    request_id: String,
) -> CommandResult<Vec<ImportKnowledgeProposalReviewProjection>> {
    state
        .service
        .list_import_knowledge_proposal_reviews(&request_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_focus_frame(
    state: State<'_, KernelState>,
    frame: FocusFrame,
) -> CommandResult<FocusFrameLifecycleSnapshot> {
    state.service.create_focus_frame(frame).map_err(Into::into)
}

#[tauri::command]
pub fn get_focus_frame_query(
    state: State<'_, KernelState>,
    focus_frame_id: String,
) -> CommandResult<FocusFrameQueryProjection> {
    state
        .service
        .get_focus_frame_query(&focus_frame_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_focus_promotion_candidates(
    state: State<'_, KernelState>,
    focus_frame_id: String,
    expected_memory_version: Option<u64>,
) -> CommandResult<Option<FocusPromotionCandidateSet>> {
    state
        .service
        .get_focus_promotion_candidates(&focus_frame_id, expected_memory_version)
        .map_err(Into::into)
}

#[tauri::command]
pub fn generate_focus_promotion_candidates(
    state: State<'_, KernelState>,
    input: FocusPromotionCandidateGenerationCommandInput,
) -> CommandResult<FocusPromotionCandidateGenerationProjection> {
    state
        .service
        .generate_focus_promotion_candidates(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn decide_focus_promotion(
    state: State<'_, KernelState>,
    input: FocusPromotionDecisionCommandInput,
) -> CommandResult<FocusPromotionDecisionProjection> {
    state
        .service
        .decide_focus_promotion(&state.markdown_vault, input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_focus_promotion_decision(
    state: State<'_, KernelState>,
    decision_id: String,
) -> CommandResult<FocusPromotionDecisionProjection> {
    state
        .service
        .get_focus_promotion_decision(&decision_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_focus_promotion_decisions(
    state: State<'_, KernelState>,
    focus_frame_id: String,
) -> CommandResult<Vec<FocusPromotionDecisionProjection>> {
    state
        .service
        .list_focus_promotion_decisions(&focus_frame_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn save_focused_context_snapshot(
    state: State<'_, KernelState>,
    snapshot: FocusedContextSnapshot,
) -> CommandResult<FocusFrameQueryProjection> {
    state
        .service
        .save_focused_context_snapshot(snapshot)
        .map_err(Into::into)
}

#[tauri::command]
pub fn upsert_knowledge_entity(
    state: State<'_, KernelState>,
    conversation_id: String,
    entity: crate::domain::contracts::KnowledgeEntity,
) -> CommandResult<()> {
    state
        .service
        .upsert_knowledge_entity(&conversation_id, entity)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_knowledge_entities(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<Vec<crate::domain::contracts::KnowledgeEntity>> {
    state
        .service
        .list_knowledge_entities(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn upsert_knowledge_relation(
    state: State<'_, KernelState>,
    conversation_id: String,
    relation: crate::domain::contracts::KnowledgeRelation,
) -> CommandResult<()> {
    state
        .service
        .upsert_knowledge_relation(&conversation_id, relation)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_knowledge_relations(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<Vec<crate::domain::contracts::KnowledgeRelation>> {
    state
        .service
        .list_knowledge_relations(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn retrieve_knowledge(
    state: State<'_, KernelState>,
    conversation_id: String,
    query: String,
    limit: Option<u32>,
) -> CommandResult<crate::domain::KnowledgeRetrievalProjection> {
    let limit = usize::try_from(limit.unwrap_or(12)).map_err(|_| CommandError {
        code: "validation",
        safe_message: "Knowledge retrieval limit is invalid.".into(),
        retryable: false,
    })?;
    match state.semantic_embedding() {
        Some(semantic) => state.service.retrieve_knowledge_with_semantic(
            &semantic,
            &conversation_id,
            &query,
            limit,
        ),
        None => state
            .service
            .retrieve_knowledge_without_vector(&conversation_id, &query, limit),
    }
    .map_err(Into::into)
}

#[tauri::command]
pub fn rebuild_knowledge_vector_index(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<usize> {
    let semantic = state.semantic_embedding().ok_or_else(|| CommandError {
        code: "semanticModelUnavailable",
        safe_message:
            "Install and verify the local semantic model pack before rebuilding the vector index."
                .into(),
        retryable: false,
    })?;
    state
        .service
        .rebuild_knowledge_vector_index_with_semantic(&semantic, &conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn delete_knowledge_entity(
    state: State<'_, KernelState>,
    conversation_id: String,
    entity_id: String,
) -> CommandResult<bool> {
    state
        .service
        .delete_knowledge_entity_and_vault(&state.markdown_vault, &conversation_id, &entity_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn upsert_evidence_ref(
    state: State<'_, KernelState>,
    conversation_id: String,
    evidence: crate::domain::contracts::EvidenceRef,
) -> CommandResult<()> {
    state
        .service
        .upsert_evidence_ref(&conversation_id, evidence)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_focus_frames(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<Vec<FocusFrameQueryProjection>> {
    state
        .service
        .list_focus_frame_queries(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn close_focus_frame(
    state: State<'_, KernelState>,
    input: FocusFrameLifecycleCommandInput,
) -> CommandResult<FocusFrameQueryProjection> {
    state.service.close_focus_frame(input).map_err(Into::into)
}

#[tauri::command]
pub fn reopen_focus_frame(
    state: State<'_, KernelState>,
    input: FocusFrameLifecycleCommandInput,
) -> CommandResult<FocusFrameQueryProjection> {
    state.service.reopen_focus_frame(input).map_err(Into::into)
}

#[tauri::command]
pub fn create_conversation(
    state: State<'_, KernelState>,
    input: CreateConversationInput,
) -> CommandResult<Conversation> {
    state.service.create_conversation(input).map_err(Into::into)
}

#[tauri::command]
pub fn load_conversation_graph(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<ConversationGraph> {
    state
        .service
        .load_conversation_graph(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn append_turn(
    state: State<'_, KernelState>,
    input: AppendTurnInput,
) -> CommandResult<ConversationNode> {
    state.service.append_turn(input).map_err(Into::into)
}

#[tauri::command]
pub fn complete_turn(
    state: State<'_, KernelState>,
    input: CompleteTurnInput,
) -> CommandResult<ConversationNode> {
    state.service.complete_turn(input).map_err(Into::into)
}

#[tauri::command]
pub fn get_context_snapshot(
    state: State<'_, KernelState>,
    snapshot_id: String,
) -> CommandResult<ContextSnapshot> {
    state
        .service
        .get_context_snapshot(&snapshot_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn update_node_position(
    state: State<'_, KernelState>,
    input: UpdateNodePositionInput,
) -> CommandResult<()> {
    state
        .service
        .update_node_position(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn save_canvas_viewport(
    state: State<'_, KernelState>,
    input: SaveCanvasViewportInput,
) -> CommandResult<CanvasViewportState> {
    state
        .service
        .save_canvas_viewport(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_canvas_viewport(
    state: State<'_, KernelState>,
    conversation_id: String,
) -> CommandResult<Option<CanvasViewportState>> {
    state
        .service
        .get_canvas_viewport(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_provider_credential(
    state: State<'_, KernelState>,
    input: SetCredentialInput,
) -> CommandResult<()> {
    state
        .credentials
        .set(&input.reference, &input.secret)
        .map_err(Into::into)
}

#[tauri::command]
pub fn has_provider_credential(
    state: State<'_, KernelState>,
    reference: CredentialRef,
) -> CommandResult<bool> {
    state.credentials.exists(&reference).map_err(Into::into)
}

#[tauri::command]
pub fn delete_provider_credential(
    state: State<'_, KernelState>,
    reference: CredentialRef,
) -> CommandResult<()> {
    state.credentials.delete(&reference).map_err(Into::into)
}

#[tauri::command]
pub fn list_providers(state: State<'_, KernelState>) -> Vec<ProviderDescriptor> {
    state.provider_runtime.descriptors()
}

#[tauri::command]
pub async fn test_provider_connection(
    state: State<'_, KernelState>,
    provider_id: String,
) -> CommandResult<ProviderConnectionTestResult> {
    let runtime = state.provider_runtime.clone();
    tauri::async_runtime::spawn_blocking(move || runtime.test_connection(&provider_id))
        .await
        .map_err(|_| CommandError {
            code: "runtimeUnavailable",
            safe_message: "The provider connection test could not be completed.".into(),
            retryable: true,
        })?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn run_model(
    state: State<'_, KernelState>,
    request: ModelRunRequest,
    on_event: Channel<ModelRunEventEnvelope>,
) -> CommandResult<()> {
    state.service.create_model_run(&request)?;
    let cancellation = RunCancellation::default();
    register_active_run(&state.active_runs, &request.run_id, cancellation.clone())?;

    let runtime = state.provider_runtime.clone();
    let service = state.service.clone();
    let run_id = request.run_id.clone();
    let joined = tauri::async_runtime::spawn_blocking(move || {
        let mut callback_error = None;
        let mut channel_open = true;
        let runtime_result = runtime.run(&request, &cancellation, &mut |event| {
            if let Err(error) = service.record_model_run_event(&event) {
                if callback_error.is_none() {
                    callback_error = Some(CommandError::from(error));
                }
                cancellation.cancel();
                return;
            }
            if channel_open && on_event.send(event).is_err() {
                cancellation.cancel();
                channel_open = false;
            }
        });
        if callback_error.is_none()
            && let Err(error) = &runtime_result
            && let Ok(Some(run)) = service
                .list_model_runs(None)
                .map(|runs| runs.into_iter().find(|run| run.run_id == request.run_id))
            && matches!(run.state, RunState::Pending | RunState::Streaming)
        {
            let failed = ModelRunEventEnvelope {
                contract_version: request.contract_version.clone(),
                event_id: new_id("run-event"),
                run_id: request.run_id.clone(),
                node_id: request.node_id.clone(),
                sequence: run.last_sequence + 1,
                occurred_at: now_timestamp(),
                event: ModelRunEvent::Failed {
                    error: provider_failure(error),
                    partial_content_retained: !run.partial_content.is_empty(),
                },
            };
            if let Err(error) = service.record_model_run_event(&failed) {
                callback_error = Some(CommandError::from(error));
            } else if channel_open {
                let _ = on_event.send(failed);
            }
        }
        callback_error.map_or_else(|| runtime_result.map_err(Into::into), Err)
    })
    .await;

    if let Ok(mut active_runs) = state.active_runs.lock() {
        active_runs.remove(&run_id);
    }
    joined.map_err(|_| CommandError {
        code: "runtimeUnavailable",
        safe_message: "The model runtime stopped unexpectedly.".into(),
        retryable: true,
    })?
}

fn register_active_run(
    active_runs: &Arc<Mutex<HashMap<String, RunCancellation>>>,
    run_id: &str,
    cancellation: RunCancellation,
) -> CommandResult<()> {
    let mut active_runs = active_runs.lock().map_err(|_| CommandError {
        code: "runtimeUnavailable",
        safe_message: "The model runtime is unavailable.".into(),
        retryable: true,
    })?;
    match active_runs.entry(run_id.to_owned()) {
        Entry::Vacant(entry) => {
            entry.insert(cancellation);
            Ok(())
        }
        Entry::Occupied(_) => Err(CommandError {
            code: "runAlreadyActive",
            safe_message: "This model run is already active.".into(),
            retryable: false,
        }),
    }
}

fn provider_failure(error: &ProviderRuntimeError) -> ProviderError {
    match error {
        ProviderRuntimeError::Provider(error) => error.clone(),
        _ => ProviderError {
            category: ProviderErrorCategory::InvalidRequest,
            provider_code: None,
            safe_message: error.to_string(),
            retryable: false,
            retry_after_ms: None,
            provider_status: None,
        },
    }
}

#[tauri::command]
pub async fn start_model_run(
    state: State<'_, KernelState>,
    input: StartModelRunInput,
    on_event: Channel<ModelRunEventEnvelope>,
) -> CommandResult<ModelRunProjection> {
    let model_capabilities = state
        .provider_runtime
        .model_capabilities(&input.provider_id, &input.model_id)?;
    let max_context_tokens = model_context_budget(
        model_capabilities.context_window_tokens,
        input.budget.max_output_tokens,
    )?;
    let (_node, request) = state.service.prepare_model_run(input, max_context_tokens)?;
    let run_id = request.run_id.clone();
    if let Some(existing) = state
        .service
        .list_model_runs(None)?
        .into_iter()
        .find(|run| run.run_id == run_id)
        && matches!(
            existing.state,
            RunState::Completed | RunState::Cancelled | RunState::Failed
        )
    {
        return Ok(existing);
    }
    run_model(state.clone(), request, on_event).await?;
    state
        .service
        .list_model_runs(None)?
        .into_iter()
        .find(|run| run.run_id == run_id)
        .ok_or_else(|| {
            CommandError::from(KernelError::NotFound {
                entity: "model run",
                id: run_id,
            })
        })
}

fn model_context_budget(
    context_window_tokens: Option<u64>,
    max_output_tokens: Option<u64>,
) -> CommandResult<Option<i64>> {
    context_window_tokens
        .map(|window| {
            let reserved_output = max_output_tokens.unwrap_or(0);
            window
                .checked_sub(reserved_output)
                .filter(|available| *available > 0)
                .ok_or_else(|| CommandError {
                    code: "contextBudgetInvalid",
                    safe_message:
                        "The requested output budget must leave room for model input context."
                            .into(),
                    retryable: false,
                })
        })
        .transpose()?
        .map(|tokens| {
            i64::try_from(tokens).map_err(|_| CommandError {
                code: "contextBudgetInvalid",
                safe_message: "The model context window is outside the supported range.".into(),
                retryable: false,
            })
        })
        .transpose()
}

#[tauri::command]
pub fn list_model_runs(
    state: State<'_, KernelState>,
    conversation_id: Option<String>,
) -> CommandResult<Vec<ModelRunProjection>> {
    state
        .service
        .list_model_runs(conversation_id.as_deref())
        .map_err(Into::into)
}

#[tauri::command]
pub fn cancel_model_run(state: State<'_, KernelState>, run_id: String) -> CommandResult<bool> {
    let active_runs = state.active_runs.lock().map_err(|_| CommandError {
        code: "runtimeUnavailable",
        safe_message: "The model runtime is unavailable.".into(),
        retryable: true,
    })?;
    if let Some(cancellation) = active_runs.get(&run_id) {
        cancellation.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_run_registration_preserves_the_original_cancellation_handle() {
        let active_runs = Arc::new(Mutex::new(HashMap::new()));
        let original = RunCancellation::default();
        let duplicate = RunCancellation::default();

        register_active_run(&active_runs, "run-1", original.clone()).unwrap();
        let error = register_active_run(&active_runs, "run-1", duplicate.clone()).unwrap_err();
        assert_eq!(error.code, "runAlreadyActive");

        active_runs.lock().unwrap().get("run-1").unwrap().cancel();
        assert!(original.is_cancelled());
        assert!(!duplicate.is_cancelled());
    }

    #[test]
    fn internal_storage_errors_are_not_exposed_to_the_frontend() {
        let error = CommandError::from(KernelError::Io(std::io::Error::other(
            "C:/private/user/path/mindscape.sqlite3",
        )));
        let json = serde_json::to_string(&error).expect("serialize command error");

        assert_eq!(error.code, "storageUnavailable");
        assert!(!json.contains("private/user/path"));
    }

    #[test]
    fn trusted_context_window_reserves_requested_output_tokens() {
        assert_eq!(
            model_context_budget(Some(16_384), Some(2_048)).unwrap(),
            Some(14_336)
        );
        assert_eq!(model_context_budget(None, Some(2_048)).unwrap(), None);

        for (window, output) in [(1_024, 2_048), (1_024, 1_024), (0, 0)] {
            let error = model_context_budget(Some(window), Some(output)).unwrap_err();
            assert_eq!(error.code, "contextBudgetInvalid");
            assert!(!error.retryable);
        }
    }
}

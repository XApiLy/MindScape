import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppendTurnInput,
  CanvasViewportState,
  CompleteTurnInput,
  CredentialRef,
  ContextSnapshot,
  Conversation,
  ConversationGraph,
  ConversationNode,
  DiscussionLog,
  DiscussionLogEditCommandResult,
  DiscussionLogProjection,
  CreateFocusFrameResult,
  CreateConversationInput,
  FocusFrame,
  FocusFrameLifecycleCommandInput,
  FocusPromotionCandidateSet,
  FocusPromotionDecisionCommandInput,
  FocusPromotionDecisionProjection,
  FocusFrameQueryResult,
  GenericImportCommandResult,
  ImportBundleQueryProjection,
  ImportGenericFileInput,
  ImportSource,
  KernelBootstrap,
  ModelRunEventEnvelope,
  ModelRunProjection,
  ModelRunRequest,
  PersistImportBundleInput,
  ProviderDescriptor,
  ProviderConnectionTestResult,
  SaveCanvasViewportInput,
  SetCredentialInput,
  StartModelRunInput,
  UpdateNodePositionInput,
  KnowledgeEntity,
  KnowledgeRelation,
  KnowledgeRetrievalProjection,
  EvidenceRef,
  MarkdownProjection,
  MarkdownEditCommandResult,
  RawImportContentProjection,
  SemanticModelPackStatus,
} from "../domain";

export const kernelClient = {
  bootstrap: () => invoke<KernelBootstrap>("bootstrap_kernel"),
  getSemanticModelPackStatus: () =>
    invoke<SemanticModelPackStatus>("get_semantic_model_pack_status"),
  installSemanticModelPack: () =>
    invoke<SemanticModelPackStatus>("install_semantic_model_pack"),
  persistImportBundle: (input: PersistImportBundleInput) =>
    invoke<void>("persist_import_bundle", input),
  importGenericFile: (input: ImportGenericFileInput) =>
    invoke<GenericImportCommandResult>("import_generic_file", input),
  listImportSources: (conversationId: string) =>
    invoke<ImportSource[]>("list_import_sources", { conversationId }),
  getImportBundle: (sourceId: string) =>
    invoke<ImportBundleQueryProjection>("get_import_bundle", { sourceId }),
  getRawImportContent: (sourceId: string) =>
    invoke<RawImportContentProjection>("get_raw_import_content", { sourceId }),
  createFocusFrame: (frame: FocusFrame) =>
    invoke<CreateFocusFrameResult>("create_focus_frame", { frame }),
  getFocusFrameQuery: (focusFrameId: string) =>
    invoke<FocusFrameQueryResult>("get_focus_frame_query", { focusFrameId }),
  getFocusPromotionCandidates: (
    focusFrameId: string,
    expectedMemoryVersion?: number,
  ) =>
    invoke<FocusPromotionCandidateSet | null>("get_focus_promotion_candidates", {
      focusFrameId,
      expectedMemoryVersion,
    }),
  decideFocusPromotion: (input: FocusPromotionDecisionCommandInput) =>
    invoke<FocusPromotionDecisionProjection>("decide_focus_promotion", { input }),
  listFocusPromotionDecisions: (focusFrameId: string) =>
    invoke<FocusPromotionDecisionProjection[]>("list_focus_promotion_decisions", {
      focusFrameId,
    }),
  getFocusPromotionDecision: (decisionId: string) =>
    invoke<FocusPromotionDecisionProjection>("get_focus_promotion_decision", {
      decisionId,
    }),
  listFocusFrames: (conversationId: string) =>
    invoke<FocusFrameQueryResult[]>("list_focus_frames", { conversationId }),
  closeFocusFrame: (input: FocusFrameLifecycleCommandInput) =>
    invoke<FocusFrameQueryResult>("close_focus_frame", { input }),
  reopenFocusFrame: (input: FocusFrameLifecycleCommandInput) =>
    invoke<FocusFrameQueryResult>("reopen_focus_frame", { input }),
  upsertKnowledgeEntity: (conversationId: string, entity: KnowledgeEntity) =>
    invoke<void>("upsert_knowledge_entity", { conversationId, entity }),
  listKnowledgeEntities: (conversationId: string) =>
    invoke<KnowledgeEntity[]>("list_knowledge_entities", { conversationId }),
  upsertKnowledgeRelation: (conversationId: string, relation: KnowledgeRelation) =>
    invoke<void>("upsert_knowledge_relation", { conversationId, relation }),
  listKnowledgeRelations: (conversationId: string) =>
    invoke<KnowledgeRelation[]>("list_knowledge_relations", { conversationId }),
  retrieveKnowledge: (conversationId: string, query: string, limit = 12) =>
    invoke<KnowledgeRetrievalProjection>("retrieve_knowledge", {
      conversationId,
      query,
      limit,
    }),
  rebuildKnowledgeVectorIndex: (conversationId: string) =>
    invoke<number>("rebuild_knowledge_vector_index", { conversationId }),
  deleteKnowledgeEntity: (conversationId: string, entityId: string) =>
    invoke<boolean>("delete_knowledge_entity", { conversationId, entityId }),
  upsertEvidenceRef: (conversationId: string, evidence: EvidenceRef) =>
    invoke<void>("upsert_evidence_ref", { conversationId, evidence }),
  projectKnowledgeEntityMarkdown: (conversationId: string, entityId: string) =>
    invoke<MarkdownProjection>("project_knowledge_entity_markdown", {
      conversationId,
      entityId,
    }),
  listMarkdownProjections: (entityId: string) =>
    invoke<MarkdownProjection[]>("list_markdown_projections", { entityId }),
  importMarkdownEntityEdit: (conversationId: string, entityId: string) =>
    invoke<MarkdownEditCommandResult>("import_markdown_entity_edit", {
      conversationId,
      entityId,
    }),
  projectDiscussionLogMarkdown: (log: DiscussionLog) =>
    invoke<DiscussionLogProjection>("project_discussion_log_markdown", { log }),
  getDiscussionLog: (discussionLogId: string) =>
    invoke<DiscussionLogProjection>("get_discussion_log", { discussionLogId }),
  listConversationDiscussionLogs: (conversationId: string) =>
    invoke<DiscussionLogProjection[]>("list_conversation_discussion_logs", {
      conversationId,
    }),
  listProjectDiscussionLogs: (projectId: string) =>
    invoke<DiscussionLogProjection[]>("list_project_discussion_logs", { projectId }),
  importDiscussionLogEdit: (discussionLogId: string) =>
    invoke<DiscussionLogEditCommandResult>("import_discussion_log_edit", {
      discussionLogId,
    }),
  createConversation: (input: CreateConversationInput) =>
    invoke<Conversation>("create_conversation", { input }),
  loadConversationGraph: (conversationId: string) =>
    invoke<ConversationGraph>("load_conversation_graph", { conversationId }),
  appendTurn: (input: AppendTurnInput) =>
    invoke<ConversationNode>("append_turn", { input }),
  completeTurn: (input: CompleteTurnInput) =>
    invoke<ConversationNode>("complete_turn", { input }),
  getContextSnapshot: (snapshotId: string) =>
    invoke<ContextSnapshot>("get_context_snapshot", { snapshotId }),
  updateNodePosition: (input: UpdateNodePositionInput) =>
    invoke<void>("update_node_position", { input }),
  saveCanvasViewport: (input: SaveCanvasViewportInput) =>
    invoke<CanvasViewportState>("save_canvas_viewport", { input }),
  getCanvasViewport: (conversationId: string) =>
    invoke<CanvasViewportState | null>("get_canvas_viewport", { conversationId }),
  setProviderCredential: (input: SetCredentialInput) =>
    invoke<void>("set_provider_credential", { input }),
  hasProviderCredential: (reference: CredentialRef) =>
    invoke<boolean>("has_provider_credential", { reference }),
  deleteProviderCredential: (reference: CredentialRef) =>
    invoke<void>("delete_provider_credential", { reference }),
  listProviders: () => invoke<ProviderDescriptor[]>("list_providers"),
  testProviderConnection: (providerId: string) =>
    invoke<ProviderConnectionTestResult>("test_provider_connection", { providerId }),
  runModel: (request: ModelRunRequest, onEvent: (event: ModelRunEventEnvelope) => void) => {
    const channel = new Channel<ModelRunEventEnvelope>();
    channel.onmessage = onEvent;
    return invoke<void>("run_model", { request, onEvent: channel });
  },
  cancelModelRun: (runId: string) => invoke<boolean>("cancel_model_run", { runId }),
  startModelRun: (
    input: StartModelRunInput,
    onEvent: (event: ModelRunEventEnvelope) => void,
  ) => {
    const channel = new Channel<ModelRunEventEnvelope>();
    channel.onmessage = onEvent;
    return invoke<ModelRunProjection>("start_model_run", { input, onEvent: channel });
  },
  listModelRuns: (conversationId?: string) =>
    invoke<ModelRunProjection[]>("list_model_runs", {
      conversationId: conversationId ?? null,
    }),
};

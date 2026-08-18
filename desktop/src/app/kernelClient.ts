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
  CreateConversationInput,
  KernelBootstrap,
  ModelRunEventEnvelope,
  ModelRunProjection,
  ModelRunRequest,
  ProviderDescriptor,
  ProviderConnectionTestResult,
  SaveCanvasViewportInput,
  SetCredentialInput,
  StartModelRunInput,
  UpdateNodePositionInput,
} from "../domain";

export const kernelClient = {
  bootstrap: () => invoke<KernelBootstrap>("bootstrap_kernel"),
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

import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppendTurnInput,
  CompleteTurnInput,
  CredentialRef,
  ContextSnapshot,
  Conversation,
  ConversationGraph,
  ConversationNode,
  CreateConversationInput,
  KernelBootstrap,
  ModelRunEventEnvelope,
  ModelRunRequest,
  SetCredentialInput,
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
  setProviderCredential: (input: SetCredentialInput) =>
    invoke<void>("set_provider_credential", { input }),
  hasProviderCredential: (reference: CredentialRef) =>
    invoke<boolean>("has_provider_credential", { reference }),
  deleteProviderCredential: (reference: CredentialRef) =>
    invoke<void>("delete_provider_credential", { reference }),
  runModel: (request: ModelRunRequest, onEvent: (event: ModelRunEventEnvelope) => void) => {
    const channel = new Channel<ModelRunEventEnvelope>();
    channel.onmessage = onEvent;
    return invoke<void>("run_model", { request, onEvent: channel });
  },
  cancelModelRun: (runId: string) => invoke<boolean>("cancel_model_run", { runId }),
};

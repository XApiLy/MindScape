import type { BranchType } from "./common";
import type { CapabilityRequirement, ModelRunBudget } from "./runtime";

export type CreateConversationInput = {
  workspaceId: string;
  title: string;
};

export type AppendTurnInput = {
  conversationId: string;
  parentNodeId: string | null;
  branchType: BranchType;
  title: string;
  prompt: string;
  providerId: string | null;
  modelId: string | null;
};

export type CompleteTurnInput = {
  nodeId: string;
  content: string;
  providerId: string;
  modelId: string;
};

export type StartModelRunInput = {
  conversationId: string;
  parentNodeId: string | null;
  branchType: BranchType;
  title: string;
  prompt: string;
  providerId: string;
  modelId: string;
  capabilities: CapabilityRequirement[];
  budget: ModelRunBudget;
  idempotencyKey: string;
};

export type UpdateNodePositionInput = {
  conversationId: string;
  nodeId: string;
  x: number;
  y: number;
};

export type CanvasViewportState = {
  conversationId: string;
  x: number;
  y: number;
  zoom: number;
  updatedAt: string;
};

export type SaveCanvasViewportInput = Omit<CanvasViewportState, "updatedAt">;

export type CredentialRef = {
  providerId: string;
  accountId: string;
};

export type SetCredentialInput = CredentialRef & {
  secret: string;
};

export type CommandError = {
  code: string;
  safeMessage: string;
  retryable: boolean;
};

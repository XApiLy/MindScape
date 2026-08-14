import type { BranchType } from "./common";

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

export type UpdateNodePositionInput = {
  conversationId: string;
  nodeId: string;
  x: number;
  y: number;
};

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

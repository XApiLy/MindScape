import type { BranchType, MessageRole } from "./common";
import type { ContentBlock } from "./content";
import type { EvidenceRef } from "./evidence";

export type ContextMessageRef = {
  messageId: string;
  role: MessageRole;
  contentBlocks: ContentBlock[];
  sourceNodeId: string;
};

export type OmittedContextRef = {
  messageId: string;
  reason: string;
};

export type ContextConstraint = {
  text: string;
  evidence: EvidenceRef[];
  userConfirmed: boolean;
};

export type ContextSnapshot = {
  id: string;
  conversationId: string;
  parentNodeId: string | null;
  branchType: BranchType;
  currentInput: string;
  selectedMessages: ContextMessageRef[];
  selectedImportRefs: EvidenceRef[];
  explicitConstraints: ContextConstraint[];
  omittedMessages: OmittedContextRef[];
  systemContractVersion: string;
  estimatedTokens: number;
  createdAt: string;
};

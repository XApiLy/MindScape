import type { ContextSnapshot } from "./context";
import type { KnowledgeContextSelection } from "./knowledge";

export type FocusContextPolicy =
  | "continueCurrent"
  | "focusNew"
  | "branchFromNode"
  | "continueImportedRaw";

export type FocusBranchKind = "mainline" | "exploration" | "task" | "retrospective";

export type FocusMemoryScope = {
  branchKind: FocusBranchKind;
  inheritRefs: string[];
  localRefs: string[];
  excludeRefs: string[];
  promoteRefs: string[];
};

export type FocusFrame = {
  contractVersion: string;
  id: string;
  conversationId: string;
  parentNodeId: string | null;
  objective: string;
  activeWorkItem: string | null;
  contextPolicy: FocusContextPolicy;
  memoryScope: FocusMemoryScope;
  includeRefs: string[];
  excludeRefs: string[];
  memoryVersion: number;
  createdAt: string;
};

/** Kernel-authored, read-only candidates awaiting explicit user confirmation. */
export type FocusPromotionCandidateSet = {
  contractVersion: "mindscape.focus.v1";
  focusFrameId: string;
  conversationId: string;
  branchKind: Exclude<FocusBranchKind, "mainline">;
  memoryVersion: number;
  candidateRefs: string[];
};

export type FocusPromotionDecisionAction = "confirm" | "promote" | "reject" | "delete";

export type FocusPromotionTargetScope =
  | { type: "conversation"; workspaceId: string; conversationId: string }
  | { type: "project"; workspaceId: string; projectId: string };

export type FocusPromotionDecisionCommandInput = {
  decisionId: string;
  focusFrameId: string;
  candidateRef: string;
  expectedMemoryVersion: number;
  expectedLifecycleRevision: number;
  expectedEntityRevision: number;
  expectedDecisionRevision: 0;
  action: FocusPromotionDecisionAction;
  targetScope: FocusPromotionTargetScope | null;
  promotedEntityId: string | null;
  decidedAt: string;
};

export type FocusPromotionDecisionProjection = {
  contractVersion: "mindscape.focus-promotion-decision.v1";
  decisionId: string;
  focusFrameId: string;
  conversationId: string;
  candidateRef: string;
  action: FocusPromotionDecisionAction;
  targetScope: FocusPromotionTargetScope | null;
  promotedEntityId: string | null;
  sourceEntityRevision: number | null;
  decisionRevision: number;
  memoryVersion: number;
  lifecycleRevision: number;
  decidedAt: string;
};

export type FocusFrameLifecycleStatus = "active" | "closed";

export type FocusFrameLifecycleSnapshot = {
  contractVersion: "mindscape.focus-lifecycle.v1";
  frame: FocusFrame;
  status: FocusFrameLifecycleStatus;
  revision: number;
  updatedAt: string;
  closedAt: string | null;
};

export type OmittedFocusRef = {
  referenceId: string;
  reason: string;
};

export type FocusedContextSnapshot = {
  contractVersion: string;
  focusFrame: FocusFrame;
  contextSnapshot: ContextSnapshot;
  selectedMemoryRefs: string[];
  omittedMemoryRefs: OmittedFocusRef[];
  knowledgeContext: KnowledgeContextSelection | null;
};

/** Read-only query projection; lifecycle status remains kernel-owned. */
export type FocusFrameQueryProjection = {
  contractVersion: "mindscape.focus-query.v1";
  lifecycle: FocusFrameLifecycleSnapshot;
  focusedContext: FocusedContextSnapshot | null;
};

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

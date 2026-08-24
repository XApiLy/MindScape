import type { ContextSnapshot } from "./context";

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
};

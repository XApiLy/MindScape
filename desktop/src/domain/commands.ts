import type { BranchType } from "./common";
import type {
  FocusFrameLifecycleSnapshot,
  FocusFrameQueryProjection,
} from "./focus";
import type { CapabilityRequirement, EffectiveRunProfile, ModelRunBudget } from "./runtime";
import type { ImportBundleQueryProjection, ImportRevision, ImportSource, ImportedMessage, ParseReport } from "./imports";
import type { MarkdownProjection } from "./knowledge";

export type { ImportBundleQueryProjection };

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

/** Structured bundle already parsed by the local import pipeline. */
export type PersistImportBundleInput = {
  source: ImportSource;
  revision: ImportRevision;
  messages: ImportedMessage[];
  report: ParseReport;
};

export type ImportGenericFileInput = {
  conversationId: string;
  originalFileName: string;
  payload: number[];
};

export type GenericImportCommandResult = {
  source: ImportSource;
  revision: ImportRevision;
  report: ParseReport;
  duplicate: boolean;
};

export type RawImportContentProjection = {
  sourceId: string;
  contentHash: string;
  byteLength: number;
  content: string;
  truncated: boolean;
};

export type MarkdownEditCommandResult = {
  projection: MarkdownProjection;
  changed: boolean;
};

export type FocusFrameLifecycleCommandInput = {
  focusFrameId: string;
  expectedRevision: number;
  updatedAt: string;
};

export type CreateFocusFrameResult = FocusFrameLifecycleSnapshot;
export type FocusFrameQueryResult = FocusFrameQueryProjection;

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
  effectiveRunProfile?: EffectiveRunProfile | null;
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

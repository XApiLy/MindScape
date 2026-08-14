import type { MessageRole } from "./common";
import type { ContentBlock } from "./content";
import type { EvidenceRef } from "./evidence";

export type ImportPlatform = "chatGpt" | "claude" | "codex" | "generic";
export type ImportRevisionStatus = "parsing" | "parsed" | "partiallyParsed" | "failed";
export type RecoveryStatus = "recovered" | "partial" | "unavailable";

export type ImportSource = {
  id: string;
  conversationId: string;
  platform: ImportPlatform;
  originalFileName: string | null;
  contentHash: string;
  storageRef: string;
  createdAt: string;
};

export type ImportRevision = {
  id: string;
  importSourceId: string;
  adapterId: string;
  adapterVersion: string;
  status: ImportRevisionStatus;
  createdAt: string;
};

export type FieldRecovery = {
  field: string;
  status: RecoveryStatus;
  detail: string | null;
};

export type ImportIssue = {
  code: string;
  message: string;
  sourceLocator: string | null;
  recoverable: boolean;
};

export type ParseReport = {
  importRevisionId: string;
  conversationCount: number;
  messageCount: number;
  attachmentCount: number;
  toolRecordCount: number;
  fieldRecovery: FieldRecovery[];
  warnings: ImportIssue[];
  errors: ImportIssue[];
};

export type ImportedMessage = {
  id: string;
  importRevisionId: string;
  role: MessageRole;
  contentBlocks: ContentBlock[];
  occurredAt: string | null;
  sourceLocator: string;
  parentImportedMessageId: string | null;
  platformExtension: unknown;
};

export type DerivedContinuation = {
  id: string;
  importSourceId: string;
  importRevisionId: string;
  revision: number;
  analysisMode: "quick" | "detailed";
  generatorId: string;
  generatorVersion: string;
  status: "active" | "superseded" | "invalidated" | "deleted";
  claims: Array<{
    id: string;
    kind: string;
    value: string;
    evidence: EvidenceRef[];
    userConfirmed: boolean;
  }>;
  createdAt: string;
};

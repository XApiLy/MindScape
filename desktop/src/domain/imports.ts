import type { MessageRole } from "./common";
import type { ContentBlock } from "./content";
import type { EvidenceRef } from "./evidence";
import type {
  GeneratorRef,
  KnowledgeEntityKind,
  KnowledgeScope,
} from "./knowledge";

export type ImportPlatform = "chatGpt" | "claude" | "codex" | "generic";
export type ImportRevisionStatus = "parsing" | "parsed" | "partiallyParsed" | "failed";
export type RecoveryStatus = "recovered" | "partial" | "unavailable";
export type ImportFormat = "markdown" | "jsonLines" | "text" | "json";
export type ImportIngress = "filePicker" | "dragAndDrop" | "paste";
export type ImportAnalysisPolicy = "disabled";

export type GenericImportDescriptor = {
  contractVersion: string;
  importSourceId: string;
  format: ImportFormat;
  ingress: ImportIngress;
  mediaType: string | null;
  encoding: string;
  byteLength: number;
  contentHash: string;
  immutableStorageRef: string;
};

export type RawTrackEntry = {
  id: string;
  importSourceId: string;
  importRevisionId: string;
  importedMessageId: string;
  sourceLocator: string;
  contentHash: string;
  ordinal: number;
};

export type ImportGraphProjection = {
  importSourceId: string;
  importRevisionId: string;
  conversationId: string;
  entryNodeId: string;
  rawTrackEntryIds: string[];
  analysisPolicy: ImportAnalysisPolicy;
};

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

export type ImportBundleQueryProjection = {
  source: ImportSource;
  revision: ImportRevision;
  messages: ImportedMessage[];
  report: ParseReport;
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

export type ImportKnowledgeProposalRequestInput = {
  requestId: string;
  importSourceId: string;
  importRevisionId: string;
  expectedSourceContentHash: string;
  selectedMessageIds: string[];
  targetScope: KnowledgeScope;
  requestedAt: string;
};

export type ImportKnowledgeEntityProposal = {
  contractVersion: "mindscape.import-knowledge-proposal.v1";
  proposalId: string;
  requestId: string;
  importSourceId: string;
  importRevisionId: string;
  conversationId: string;
  suggestedKind: KnowledgeEntityKind;
  suggestedName: string;
  suggestedAliases: string[];
  targetScope: KnowledgeScope;
  evidence: EvidenceRef[];
  generator: GeneratorRef;
  proposalRevision: number;
  proposedAt: string;
};

export type ImportKnowledgeProposalBatchProjection = {
  contractVersion: "mindscape.import-knowledge-proposal.v1";
  requestId: string;
  importSourceId: string;
  importRevisionId: string;
  conversationId: string;
  sourceContentHash: string;
  targetScope: KnowledgeScope;
  generationRunId: string;
  generator: GeneratorRef;
  proposals: ImportKnowledgeEntityProposal[];
  batchRevision: number;
  requestedAt: string;
  generatedAt: string;
};

export type ImportKnowledgeProposalReviewChoice =
  | {
      action: "confirm";
      kind: KnowledgeEntityKind;
      name: string;
      aliases: string[];
    }
  | { action: "reject"; reason: string | null };

export type ImportKnowledgeProposalReviewCommandInput = {
  decisionId: string;
  requestId: string;
  proposalId: string;
  expectedProposalRevision: number;
  choice: ImportKnowledgeProposalReviewChoice;
  decidedAt: string;
};

export type ImportKnowledgeProposalReviewProjection = {
  contractVersion: "mindscape.import-knowledge-proposal.v1";
  decisionId: string;
  requestId: string;
  proposalId: string;
  proposalRevision: number;
  action: "confirm" | "reject";
  entityId: string | null;
  entityStatus: "candidate" | "confirmed" | null;
  decidedBy: GeneratorRef;
  decidedAt: string;
};

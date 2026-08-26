import type { EvidenceRef } from "./evidence";

export type KnowledgeEntityKind =
  | "goal"
  | "decision"
  | "constraint"
  | "question"
  | "source"
  | "project"
  | "topic";

export type KnowledgeStatus =
  | "candidate"
  | "inferred"
  | "confirmed"
  | "rejected"
  | "superseded"
  | "stale";

export type KnowledgeScope =
  | { type: "workspace"; workspaceId: string }
  | { type: "project"; workspaceId: string; projectId: string }
  | { type: "conversation"; workspaceId: string; conversationId: string }
  | {
      type: "focusFrame";
      workspaceId: string;
      conversationId: string;
      focusFrameId: string;
    };

export type GeneratorRef = {
  kind: "user" | "deterministicRule" | "model";
  generatorId: string;
  generatorVersion: string;
};

export type ScopedEvidenceRef = {
  id: string;
  evidence: EvidenceRef;
  scope: KnowledgeScope;
  status: KnowledgeStatus;
  revision: number;
  generator: GeneratorRef;
};

export type KnowledgeEntity = {
  contractVersion: string;
  id: string;
  kind: KnowledgeEntityKind;
  name: string;
  aliases: string[];
  scope: KnowledgeScope;
  status: KnowledgeStatus;
  revision: number;
  evidence: ScopedEvidenceRef[];
  generator: GeneratorRef;
  createdAt: string;
  updatedAt: string;
};

export type KnowledgeRetrievalContext = {
  workspaceId: string;
  projectId: string | null;
  conversationId: string;
  focusFrameId: string;
};

export type KnowledgeRetrievalCandidate = {
  entity: KnowledgeEntity;
  evidence: EvidenceRef[];
  retrievalScore: number;
  estimatedTokens: number;
};

export type KnowledgeContextReference = {
  entityId: string;
  status: KnowledgeStatus;
  scope: KnowledgeScope;
  revision: number;
  evidence: EvidenceRef[];
  retrievalScore: number;
  estimatedTokens: number;
};

export type OmittedKnowledgeRef = {
  referenceId: string;
  reason: string;
};

export type KnowledgeContextSelection = {
  contractVersion: "mindscape.knowledge-context.v1";
  retrievalVersion: string;
  selected: KnowledgeContextReference[];
  omitted: OmittedKnowledgeRef[];
  estimatedTokens: number;
};

export type KnowledgeRetrievalSource = "vector" | "fullText" | "relation";

export type KnowledgeRetrievalAvailability = "available" | "unavailable";

export type KnowledgeRetrievalNotice = {
  vectorStatus: KnowledgeRetrievalAvailability;
  usedFallback: boolean;
  safeMessage: string | null;
};

export type KnowledgeEmbeddingProvenance = {
  modelVersion: string;
  dimensions: number;
  sourceHash: string;
  chunkVersion: string;
};

export type KnowledgeRetrievalCandidateProjection = {
  entity: KnowledgeEntity;
  evidence: EvidenceRef[];
  retrievalScore: number;
  estimatedTokens: number;
  sources: KnowledgeRetrievalSource[];
  embedding: KnowledgeEmbeddingProvenance | null;
};

export type KnowledgeRetrievalProjection = {
  contractVersion: "mindscape.knowledge-retrieval.v1";
  retrievalVersion: string;
  candidates: KnowledgeRetrievalCandidateProjection[];
  omitted: OmittedKnowledgeRef[];
  notice: KnowledgeRetrievalNotice;
};

export type KnowledgeRelationKind =
  | "mentions"
  | "belongsTo"
  | "supports"
  | "contradicts"
  | "dependsOn"
  | "derivedFrom"
  | "supersedes"
  | "relatedTo"
  | "continuedBy";

export type KnowledgeRelation = {
  contractVersion: string;
  id: string;
  kind: KnowledgeRelationKind;
  sourceEntityId: string;
  targetEntityId: string;
  scope: KnowledgeScope;
  status: KnowledgeStatus;
  revision: number;
  evidence: ScopedEvidenceRef[];
  generator: GeneratorRef;
  createdAt: string;
  updatedAt: string;
};

export type MarkdownProjection = {
  contractVersion: string;
  id: string;
  targetEntityId: string;
  relativePath: string;
  entityRevision: number;
  projectionRevision: number;
  contentHash: string;
  frontmatterVersion: string;
  createdAt: string;
};

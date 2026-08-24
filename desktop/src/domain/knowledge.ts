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

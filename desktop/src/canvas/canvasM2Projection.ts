import type {
  FocusFrame,
  FocusFrameQueryProjection,
  ImportGraphProjection,
  ImportRevision,
  ImportSource,
  KnowledgeEntity,
  KnowledgeContextSelection,
  KnowledgeRetrievalProjection,
  ParseReport,
} from "../domain";

/**
 * M2 contract-to-canvas projections.
 *
 * These are intentionally view-only shapes. The canvas must not reproduce
 * FocusFrame inheritance, knowledge lifecycle transitions, or import parsing;
 * those remain authoritative in the domain/query boundary.
 */
export type CanvasFocusFrameProjection = {
  contractVersion: string;
  id: string;
  conversationId: string;
  parentNodeId: string | null;
  objective: string;
  activeWorkItem: string | null;
  contextPolicy: FocusFrame["contextPolicy"];
  memoryScope: {
    branchKind: FocusFrame["memoryScope"]["branchKind"];
    inheritRefs: string[];
    localRefs: string[];
    excludeRefs: string[];
    promoteRefs: string[];
  };
  includeRefs: string[];
  excludeRefs: string[];
  memoryVersion: number;
  createdAt: string;
};

export function projectFocusFrame(frame: FocusFrame | null): CanvasFocusFrameProjection | null {
  if (!frame) return null;

  return {
    contractVersion: frame.contractVersion,
    id: frame.id,
    conversationId: frame.conversationId,
    parentNodeId: frame.parentNodeId,
    objective: frame.objective,
    activeWorkItem: frame.activeWorkItem,
    contextPolicy: frame.contextPolicy,
    memoryScope: {
      branchKind: frame.memoryScope.branchKind,
      inheritRefs: [...frame.memoryScope.inheritRefs],
      localRefs: [...frame.memoryScope.localRefs],
      excludeRefs: [...frame.memoryScope.excludeRefs],
      promoteRefs: [...frame.memoryScope.promoteRefs],
    },
    includeRefs: [...frame.includeRefs],
    excludeRefs: [...frame.excludeRefs],
    memoryVersion: frame.memoryVersion,
    createdAt: frame.createdAt,
  };
}

export type CanvasFocusedContextState =
  | "unavailable"
  | "availableWithoutKnowledge"
  | "availableWithKnowledge";

export type CanvasFocusedContextProjection = {
  snapshotId: string;
  focusFrameId: string;
  selectedMemoryRefs: string[];
  omittedMemoryRefs: Array<{ referenceId: string; reason: string }>;
  knowledgeContext: CanvasKnowledgeContextProjection | null;
};

export type CanvasFocusFrameQueryProjection = {
  contractVersion: FocusFrameQueryProjection["contractVersion"];
  lifecycle: {
    focusFrame: CanvasFocusFrameProjection;
    status: FocusFrameQueryProjection["lifecycle"]["status"];
    revision: number;
    updatedAt: string;
    closedAt: string | null;
  };
  focusedContext: CanvasFocusedContextProjection | null;
  focusedContextState: CanvasFocusedContextState;
};

export function projectFocusFrameQuery(
  query: FocusFrameQueryProjection,
): CanvasFocusFrameQueryProjection | null {
  const focusFrame = projectFocusFrame(query.lifecycle.frame);
  if (!focusFrame) return null;

  const focusedContext = query.focusedContext
    ? {
        snapshotId: query.focusedContext.contextSnapshot.id,
        focusFrameId: query.focusedContext.focusFrame.id,
        selectedMemoryRefs: [...query.focusedContext.selectedMemoryRefs],
        omittedMemoryRefs: query.focusedContext.omittedMemoryRefs.map((reference) => ({ ...reference })),
        knowledgeContext: projectKnowledgeContext(query.focusedContext.knowledgeContext),
      }
    : null;

  return {
    contractVersion: query.contractVersion,
    lifecycle: {
      focusFrame,
      status: query.lifecycle.status,
      revision: query.lifecycle.revision,
      updatedAt: query.lifecycle.updatedAt,
      closedAt: query.lifecycle.closedAt,
    },
    focusedContext,
    focusedContextState: !focusedContext
      ? "unavailable"
      : focusedContext.knowledgeContext === null
        ? "availableWithoutKnowledge"
        : "availableWithKnowledge",
  };
}

/**
 * Indexes a query result by the explicit FocusFrame parent node.
 *
 * This is a view lookup only: a null parent is a valid mainline frame and is
 * intentionally not assigned to an arbitrary canvas node. The caller remains
 * responsible for obtaining the query result from the kernel.
 */
export function upsertFocusFrameQueryByNodeId(
  current: ReadonlyMap<string, CanvasFocusFrameQueryProjection>,
  query: CanvasFocusFrameQueryProjection,
): Map<string, CanvasFocusFrameQueryProjection> {
  const parentNodeId = query.lifecycle.focusFrame.parentNodeId;
  const next = new Map(current);
  if (!parentNodeId) return next;

  const existing = next.get(parentNodeId);
  if (!existing || isNewerFocusFrameQuery(query, existing)) {
    next.set(parentNodeId, query);
  }
  return next;
}

function isNewerFocusFrameQuery(
  candidate: CanvasFocusFrameQueryProjection,
  existing: CanvasFocusFrameQueryProjection,
): boolean {
  const candidateId = candidate.lifecycle.focusFrame.id;
  const existingId = existing.lifecycle.focusFrame.id;

  // Revision is monotonic for one FocusFrame, so it wins over a stale
  // response even if the request timestamps arrive out of order.
  if (candidateId === existingId) {
    return candidate.lifecycle.revision > existing.lifecycle.revision
      || (candidate.lifecycle.revision === existing.lifecycle.revision
        && candidate.lifecycle.updatedAt >= existing.lifecycle.updatedAt);
  }

  // Multiple task frames may share a parent node. Prefer the one most
  // recently changed; equal timestamps retain the first result from the
  // kernel's deterministic list order.
  return candidate.lifecycle.updatedAt > existing.lifecycle.updatedAt
    || (candidate.lifecycle.updatedAt === existing.lifecycle.updatedAt
      && candidate.lifecycle.revision > existing.lifecycle.revision);
}

export function removeFocusFrameQueryByNodeId(
  current: ReadonlyMap<string, CanvasFocusFrameQueryProjection>,
  query: CanvasFocusFrameQueryProjection,
): Map<string, CanvasFocusFrameQueryProjection> {
  const parentNodeId = query.lifecycle.focusFrame.parentNodeId;
  const next = new Map(current);
  if (parentNodeId) next.delete(parentNodeId);
  return next;
}

export type CanvasKnowledgeReferenceProjection = {
  id: string;
  kind: KnowledgeEntity["kind"];
  name: string;
  aliases: string[];
  status: KnowledgeEntity["status"];
  scopeType: KnowledgeEntity["scope"]["type"];
  revision: number;
  evidence: Array<{
    id: string;
    excerpt: string | null;
    targetType: KnowledgeEntity["evidence"][number]["evidence"]["target"]["type"];
  }>;
};

export function projectKnowledgeReference(
  entity: KnowledgeEntity,
): CanvasKnowledgeReferenceProjection {
  return {
    id: entity.id,
    kind: entity.kind,
    name: entity.name,
    aliases: [...entity.aliases],
    status: entity.status,
    scopeType: entity.scope.type,
    revision: entity.revision,
    evidence: entity.evidence.map((reference) => ({
      id: reference.id,
      excerpt: reference.evidence.excerpt,
      targetType: reference.evidence.target.type,
    })),
  };
}

export type CanvasKnowledgeRetrievalProjection = {
  contractVersion: KnowledgeRetrievalProjection["contractVersion"];
  retrievalVersion: string;
  candidates: Array<{
    entity: CanvasKnowledgeReferenceProjection;
    retrievalScore: number;
    estimatedTokens: number;
    sources: KnowledgeRetrievalProjection["candidates"][number]["sources"];
    evidence: Array<{
      id: string;
      excerpt: string | null;
      targetType: KnowledgeRetrievalProjection["candidates"][number]["evidence"][number]["target"]["type"];
    }>;
    embedding: KnowledgeRetrievalProjection["candidates"][number]["embedding"];
  }>;
  omitted: Array<{ referenceId: string; reason: string }>;
  notice: {
    vectorStatus: KnowledgeRetrievalProjection["notice"]["vectorStatus"];
    usedFallback: boolean;
    safeMessage: string | null;
  };
};

/**
 * Converts the validated unified retrieval boundary into immutable canvas
 * display data. Candidate order, scores, omissions and fallback facts are
 * preserved exactly; the canvas must not filter or re-rank them.
 */
export function projectKnowledgeRetrieval(
  retrieval: KnowledgeRetrievalProjection,
): CanvasKnowledgeRetrievalProjection {
  return {
    contractVersion: retrieval.contractVersion,
    retrievalVersion: retrieval.retrievalVersion,
    candidates: retrieval.candidates.map((candidate) => ({
      entity: projectKnowledgeReference(candidate.entity),
      retrievalScore: candidate.retrievalScore,
      estimatedTokens: candidate.estimatedTokens,
      sources: [...candidate.sources],
      evidence: candidate.evidence.map((evidence) => ({
        id: evidence.id,
        excerpt: evidence.excerpt,
        targetType: evidence.target.type,
      })),
      embedding: candidate.embedding ? { ...candidate.embedding } : null,
    })),
    omitted: retrieval.omitted.map((reference) => ({ ...reference })),
    notice: { ...retrieval.notice },
  };
}

export type CanvasKnowledgeContextProjection = {
  contractVersion: KnowledgeContextSelection["contractVersion"];
  retrievalVersion: string;
  estimatedTokens: number;
  selected: Array<{
    entityId: string;
    status: KnowledgeContextSelection["selected"][number]["status"];
    scopeType: KnowledgeContextSelection["selected"][number]["scope"]["type"];
    revision: number;
    evidence: Array<{
      id: string;
      excerpt: string | null;
      targetType: KnowledgeContextSelection["selected"][number]["evidence"][number]["target"]["type"];
    }>;
    retrievalScore: number;
    estimatedTokens: number;
  }>;
  omitted: Array<{
    referenceId: string;
    reason: string;
  }>;
};

export function projectKnowledgeContext(
  selection: KnowledgeContextSelection | null,
): CanvasKnowledgeContextProjection | null {
  if (!selection) return null;

  return {
    contractVersion: selection.contractVersion,
    retrievalVersion: selection.retrievalVersion,
    estimatedTokens: selection.estimatedTokens,
    selected: selection.selected.map((reference) => ({
      entityId: reference.entityId,
      status: reference.status,
      scopeType: reference.scope.type,
      revision: reference.revision,
      evidence: reference.evidence.map((evidence) => ({
        id: evidence.id,
        excerpt: evidence.excerpt,
        targetType: evidence.target.type,
      })),
      retrievalScore: reference.retrievalScore,
      estimatedTokens: reference.estimatedTokens,
    })),
    omitted: selection.omitted.map((reference) => ({ ...reference })),
  };
}

export type CanvasImportSourceProjection = {
  sourceId: string;
  revisionId: string;
  conversationId: string;
  platform: ImportSource["platform"];
  originalFileName: string | null;
  contentHash: string;
  revisionStatus: ImportRevision["status"];
  analysisPolicy: ImportGraphProjection["analysisPolicy"];
  rawTrackEntryCount: number;
  parseReport: {
    conversationCount: number;
    messageCount: number;
    attachmentCount: number;
    toolRecordCount: number;
    warningCount: number;
    errorCount: number;
  } | null;
};

export function projectImportSource(
  source: ImportSource,
  revision: ImportRevision,
  graph: ImportGraphProjection,
  parseReport: ParseReport | null,
): CanvasImportSourceProjection {
  return {
    sourceId: source.id,
    revisionId: revision.id,
    conversationId: graph.conversationId,
    platform: source.platform,
    originalFileName: source.originalFileName,
    contentHash: source.contentHash,
    revisionStatus: revision.status,
    analysisPolicy: graph.analysisPolicy,
    rawTrackEntryCount: graph.rawTrackEntryIds.length,
    parseReport: parseReport
      ? {
          conversationCount: parseReport.conversationCount,
          messageCount: parseReport.messageCount,
          attachmentCount: parseReport.attachmentCount,
          toolRecordCount: parseReport.toolRecordCount,
          warningCount: parseReport.warnings.length,
          errorCount: parseReport.errors.length,
        }
      : null,
  };
}

import assert from "node:assert/strict";
import test from "node:test";
import type {
  FocusFrame,
  FocusFrameQueryProjection,
  ImportGraphProjection,
  ImportRevision,
  ImportSource,
  KnowledgeContextSelection,
  KnowledgeEntity,
  KnowledgeRetrievalProjection,
  ParseReport,
} from "../domain";
import {
  projectFocusFrame,
  projectFocusFrameQuery,
  projectImportSource,
  projectKnowledgeContext,
  projectKnowledgeReference,
  projectKnowledgeRetrieval,
  removeFocusFrameQueryByNodeId,
  upsertFocusFrameQueryByNodeId,
} from "./canvasM2Projection.ts";

const timestamp = "2026-08-24T10:00:00.000Z";

function focusFrame(): FocusFrame {
  return {
    contractVersion: "m2",
    id: "focus-1",
    conversationId: "conversation-1",
    parentNodeId: "node-1",
    objective: "验证导入来源投影",
    activeWorkItem: "CAN-M2-004",
    contextPolicy: "branchFromNode",
    memoryScope: {
      branchKind: "exploration",
      inheritRefs: ["node-1"],
      localRefs: ["note-1"],
      excludeRefs: ["node-ignored"],
      promoteRefs: ["entity-1"],
    },
    includeRefs: ["node-1", "entity-1"],
    excludeRefs: ["node-ignored"],
    memoryVersion: 3,
    createdAt: timestamp,
  };
}

test("projects a FocusFrame without evaluating its memory semantics", () => {
  const source = focusFrame();
  const projection = projectFocusFrame(source);

  assert.deepEqual(projection, {
    contractVersion: "m2",
    id: "focus-1",
    conversationId: "conversation-1",
    parentNodeId: "node-1",
    objective: "验证导入来源投影",
    activeWorkItem: "CAN-M2-004",
    contextPolicy: "branchFromNode",
    memoryScope: {
      branchKind: "exploration",
      inheritRefs: ["node-1"],
      localRefs: ["note-1"],
      excludeRefs: ["node-ignored"],
      promoteRefs: ["entity-1"],
    },
    includeRefs: ["node-1", "entity-1"],
    excludeRefs: ["node-ignored"],
    memoryVersion: 3,
    createdAt: timestamp,
  });
  assert.notStrictEqual(projection?.includeRefs, source.includeRefs);
  assert.notStrictEqual(projection?.memoryScope, source.memoryScope);
  assert.equal(projectFocusFrame(null), null);
});

test("keeps FocusFrame lifecycle and focused-context availability as separate states", () => {
  const source = focusFrame();
  const baseQuery: FocusFrameQueryProjection = {
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      contractVersion: "mindscape.focus-lifecycle.v1",
      frame: source,
      status: "active",
      revision: 2,
      updatedAt: timestamp,
      closedAt: null,
    },
    focusedContext: null,
  };

  const unavailable = projectFocusFrameQuery(baseQuery);
  assert.equal(unavailable?.lifecycle.status, "active");
  assert.equal(unavailable?.lifecycle.revision, 2);
  assert.equal(unavailable?.focusedContext, null);
  assert.equal(unavailable?.focusedContextState, "unavailable");

  const withoutKnowledge = projectFocusFrameQuery({
    ...baseQuery,
    lifecycle: { ...baseQuery.lifecycle, status: "closed", closedAt: "2026-08-24T11:00:00.000Z" },
    focusedContext: {
      contractVersion: "m2",
      focusFrame: source,
      contextSnapshot: {
        id: "snapshot-1",
        conversationId: "conversation-1",
        parentNodeId: "node-1",
        branchType: "deepens",
        currentInput: "继续任务",
        selectedMessages: [],
        selectedImportRefs: [],
        explicitConstraints: [],
        omittedMessages: [],
        systemContractVersion: "v1",
        estimatedTokens: 12,
        createdAt: timestamp,
      },
      selectedMemoryRefs: ["node-1"],
      omittedMemoryRefs: [{ referenceId: "node-ignored", reason: "excludedByFocusFrame" }],
      knowledgeContext: null,
    },
  });
  assert.equal(withoutKnowledge?.lifecycle.status, "closed");
  assert.equal(withoutKnowledge?.focusedContextState, "availableWithoutKnowledge");
  assert.deepEqual(withoutKnowledge?.focusedContext?.selectedMemoryRefs, ["node-1"]);
  assert.notStrictEqual(
    withoutKnowledge?.focusedContext?.selectedMemoryRefs,
    baseQuery.focusedContext?.selectedMemoryRefs,
  );
});

test("indexes FocusFrame queries only by their explicit parent node", () => {
  const source = focusFrame();
  const projected = projectFocusFrameQuery({
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      contractVersion: "mindscape.focus-lifecycle.v1",
      frame: source,
      status: "active",
      revision: 1,
      updatedAt: timestamp,
      closedAt: null,
    },
    focusedContext: null,
  });
  assert.ok(projected);

  const seed = new Map<string, typeof projected>([["other-node", projected]]);
  const indexed = upsertFocusFrameQueryByNodeId(new Map(), projected);
  assert.equal(indexed.get("node-1"), projected);
  assert.equal(indexed.has("other-node"), false);
  assert.equal(seed.size, 1, "indexing must not mutate the caller-owned map");

  const removed = removeFocusFrameQueryByNodeId(indexed, projected);
  assert.equal(removed.has("node-1"), false);

  const mainline = projectFocusFrameQuery({
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      contractVersion: "mindscape.focus-lifecycle.v1",
      frame: { ...source, parentNodeId: null },
      status: "active",
      revision: 1,
      updatedAt: timestamp,
      closedAt: null,
    },
    focusedContext: null,
  });
  assert.ok(mainline);
  assert.equal(upsertFocusFrameQueryByNodeId(new Map(), mainline).size, 0);
});

test("keeps the newest FocusFrame query when a node has multiple frames", () => {
  const source = focusFrame();
  const makeQuery = (
    id: string,
    updatedAt: string,
    revision: number,
  ) => projectFocusFrameQuery({
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      contractVersion: "mindscape.focus-lifecycle.v1",
      frame: { ...source, id },
      status: "active",
      revision,
      updatedAt,
      closedAt: null,
    },
    focusedContext: null,
  });

  const older = makeQuery("focus-older", "2026-08-25T10:00:00Z", 1);
  const newer = makeQuery("focus-newer", "2026-08-25T11:00:00Z", 1);
  assert.ok(older);
  assert.ok(newer);

  const indexed = upsertFocusFrameQueryByNodeId(
    upsertFocusFrameQueryByNodeId(new Map(), newer),
    older,
  );
  assert.equal(indexed.get("node-1")?.lifecycle.focusFrame.id, "focus-newer");

  const sameFrameRevision = makeQuery("focus-newer", "2026-08-25T09:00:00Z", 1);
  assert.ok(sameFrameRevision);
  const unchanged = upsertFocusFrameQueryByNodeId(indexed, sameFrameRevision);
  assert.equal(unchanged.get("node-1")?.lifecycle.updatedAt, "2026-08-25T11:00:00Z");
});

test("projects knowledge references as explicit lifecycle and evidence fields", () => {
  const entity: KnowledgeEntity = {
    contractVersion: "m2",
    id: "entity-1",
    kind: "decision",
    name: "原文不可执行",
    aliases: ["安全边界"],
    scope: { type: "conversation", workspaceId: "workspace-1", conversationId: "conversation-1" },
    status: "candidate",
    revision: 2,
    evidence: [{
      id: "evidence-1",
      evidence: {
        id: "evidence-1",
        target: {
          type: "importContent",
          importSourceId: "source-1",
          importRevisionId: "revision-1",
          locator: "line:4",
        },
        contentHash: "sha256:abc",
        excerpt: "导入原文仅作为事实来源",
        createdAt: timestamp,
      },
      scope: { type: "conversation", workspaceId: "workspace-1", conversationId: "conversation-1" },
      status: "candidate",
      revision: 1,
      generator: { kind: "user", generatorId: "user", generatorVersion: "1" },
    }],
    generator: { kind: "user", generatorId: "user", generatorVersion: "1" },
    createdAt: timestamp,
    updatedAt: timestamp,
  };

  const projection = projectKnowledgeReference(entity);
  assert.deepEqual(projection, {
    id: "entity-1",
    kind: "decision",
    name: "原文不可执行",
    aliases: ["安全边界"],
    status: "candidate",
    scopeType: "conversation",
    revision: 2,
    evidence: [{ id: "evidence-1", excerpt: "导入原文仅作为事实来源", targetType: "importContent" }],
  });
  assert.notStrictEqual(projection.aliases, entity.aliases);
});

test("projects selected and omitted knowledge context without re-running compiler rules", () => {
  const selection: KnowledgeContextSelection = {
    contractVersion: "mindscape.knowledge-context.v1",
    retrievalVersion: "fts-v1",
    estimatedTokens: 42,
    selected: [{
      entityId: "entity-1",
      status: "confirmed",
      scope: { type: "focusFrame", workspaceId: "workspace-1", conversationId: "conversation-1", focusFrameId: "focus-1" },
      revision: 4,
      evidence: [{
        id: "evidence-1",
        target: {
          type: "importContent",
          importSourceId: "source-1",
          importRevisionId: "revision-1",
          locator: "line:4",
        },
        contentHash: "sha256:abc",
        excerpt: "可审计来源",
        createdAt: timestamp,
      }],
      retrievalScore: 0.91,
      estimatedTokens: 42,
    }],
    omitted: [{ referenceId: "entity-2", reason: "excludedByFocusFrame" }],
  };

  const projection = projectKnowledgeContext(selection);
  assert.deepEqual(projection, {
    contractVersion: "mindscape.knowledge-context.v1",
    retrievalVersion: "fts-v1",
    estimatedTokens: 42,
    selected: [{
      entityId: "entity-1",
      status: "confirmed",
      scopeType: "focusFrame",
      revision: 4,
      evidence: [{ id: "evidence-1", excerpt: "可审计来源", targetType: "importContent" }],
      retrievalScore: 0.91,
      estimatedTokens: 42,
    }],
    omitted: [{ referenceId: "entity-2", reason: "excludedByFocusFrame" }],
  });
  assert.notStrictEqual(projection?.selected, selection.selected);
  assert.notStrictEqual(projection?.omitted, selection.omitted);
  assert.equal(projectKnowledgeContext(null), null);
});

test("projects unified retrieval candidates without filtering, re-ranking or losing provenance", () => {
  const entity: KnowledgeEntity = {
    contractVersion: "mindscape.knowledge.v1",
    id: "entity-vector",
    kind: "constraint",
    name: "原文不可执行",
    aliases: ["安全边界"],
    scope: { type: "conversation", workspaceId: "workspace-1", conversationId: "conversation-1" },
    status: "confirmed",
    revision: 3,
    evidence: [],
    generator: { kind: "user", generatorId: "user", generatorVersion: "1" },
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  const evidence = {
    id: "evidence-vector",
    target: {
      type: "importContent" as const,
      importSourceId: "source-1",
      importRevisionId: "revision-1",
      locator: "line:4",
    },
    contentHash: "sha256:abc",
    excerpt: "不可执行导入原文",
    createdAt: timestamp,
  };
  const retrieval: KnowledgeRetrievalProjection = {
    contractVersion: "mindscape.knowledge-retrieval.v1",
    retrievalVersion: "hybrid-v1",
    candidates: [{
      entity,
      evidence: [evidence],
      retrievalScore: 93,
      estimatedTokens: 24,
      sources: ["fullText", "vector"],
      embedding: {
        modelVersion: "local-hash-v1",
        dimensions: 64,
        sourceHash: "sha256:abc",
        chunkVersion: "markdown-v1",
      },
    }, {
      entity: { ...entity, id: "entity-relation", name: "关系来源", revision: 1 },
      evidence: [],
      retrievalScore: 71,
      estimatedTokens: 12,
      sources: ["relation"],
      embedding: null,
    }],
    omitted: [{ referenceId: "entity-stale", reason: "stale knowledge is not eligible" }],
    notice: {
      vectorStatus: "available",
      usedFallback: false,
      safeMessage: null,
    },
  };

  const projection = projectKnowledgeRetrieval(retrieval);
  assert.deepEqual(projection.candidates.map((candidate) => candidate.entity.id), [
    "entity-vector",
    "entity-relation",
  ]);
  assert.deepEqual(projection.candidates[0], {
    entity: {
      id: "entity-vector",
      kind: "constraint",
      name: "原文不可执行",
      aliases: ["安全边界"],
      status: "confirmed",
      scopeType: "conversation",
      revision: 3,
      evidence: [],
    },
    retrievalScore: 93,
    estimatedTokens: 24,
    sources: ["fullText", "vector"],
    evidence: [{ id: "evidence-vector", excerpt: "不可执行导入原文", targetType: "importContent" }],
    embedding: {
      modelVersion: "local-hash-v1",
      dimensions: 64,
      sourceHash: "sha256:abc",
      chunkVersion: "markdown-v1",
    },
  });
  assert.deepEqual(projection.omitted, retrieval.omitted);
  assert.deepEqual(projection.notice, retrieval.notice);
  assert.notStrictEqual(projection.candidates, retrieval.candidates);
  assert.notStrictEqual(projection.candidates[0].sources, retrieval.candidates[0].sources);
  assert.notStrictEqual(projection.candidates[0].embedding, retrieval.candidates[0].embedding);
  assert.notStrictEqual(projection.omitted, retrieval.omitted);
  assert.notStrictEqual(projection.notice, retrieval.notice);
});

test("projects import source metadata without parsing or synthesizing records", () => {
  const source: ImportSource = {
    id: "source-1",
    conversationId: "conversation-1",
    platform: "generic",
    originalFileName: "archive.md",
    contentHash: "sha256:abc",
    storageRef: "raw/sha256/abc",
    createdAt: timestamp,
  };
  const revision: ImportRevision = {
    id: "revision-1",
    importSourceId: "source-1",
    adapterId: "generic-markdown",
    adapterVersion: "m2",
    status: "partiallyParsed",
    createdAt: timestamp,
  };
  const graph: ImportGraphProjection = {
    importSourceId: "source-1",
    importRevisionId: "revision-1",
    conversationId: "conversation-1",
    entryNodeId: "import-root",
    rawTrackEntryIds: ["track-1", "track-2"],
    analysisPolicy: "disabled",
  };
  const report: ParseReport = {
    importRevisionId: "revision-1",
    conversationCount: 1,
    messageCount: 2,
    attachmentCount: 0,
    toolRecordCount: 0,
    fieldRecovery: [],
    warnings: [{ code: "missing_time", message: "缺少时间", sourceLocator: "line:2", recoverable: true }],
    errors: [],
  };

  assert.deepEqual(projectImportSource(source, revision, graph, report), {
    sourceId: "source-1",
    revisionId: "revision-1",
    conversationId: "conversation-1",
    platform: "generic",
    originalFileName: "archive.md",
    contentHash: "sha256:abc",
    revisionStatus: "partiallyParsed",
    analysisPolicy: "disabled",
    rawTrackEntryCount: 2,
    parseReport: {
      conversationCount: 1,
      messageCount: 2,
      attachmentCount: 0,
      toolRecordCount: 0,
      warningCount: 1,
      errorCount: 0,
    },
  });
  assert.equal(projectImportSource(source, revision, graph, null).parseReport, null);
});

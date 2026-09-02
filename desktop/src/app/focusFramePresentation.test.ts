import assert from "node:assert/strict";
import test from "node:test";
import { projectFocusFrameQuery } from "./focusFramePresentation.ts";
import type { FocusFrame, FocusFrameQueryProjection } from "../domain/focus.ts";

const frame: FocusFrame = {
  contractVersion: "mindscape.focus.v1",
  id: "focus-1",
  conversationId: "conversation-1",
  parentNodeId: null,
  objective: "验证 FocusFrame 查询投影",
  activeWorkItem: null,
  contextPolicy: "continueCurrent",
  memoryScope: {
    branchKind: "mainline",
    inheritRefs: [],
    localRefs: [],
    excludeRefs: [],
    promoteRefs: [],
  },
  includeRefs: [],
  excludeRefs: [],
  memoryVersion: 1,
  createdAt: "2026-08-25T00:00:00Z",
};

function projection(
  status: "active" | "closed",
  focusedContext: FocusFrameQueryProjection["focusedContext"] = null,
): FocusFrameQueryProjection {
  return {
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      contractVersion: "mindscape.focus-lifecycle.v1",
      frame,
      status,
      revision: status === "active" ? 1 : 2,
      updatedAt: "2026-08-25T00:01:00Z",
      closedAt: status === "closed" ? "2026-08-25T00:01:00Z" : null,
    },
    focusedContext,
  };
}

test("keeps active lifecycle separate from an uncompiled context", () => {
  assert.deepEqual(projectFocusFrameQuery(projection("active")), {
    lifecycleStatus: "active",
    lifecycleLabel: "当前聚焦",
    contextState: "unavailable",
    contextLabel: "上下文快照待内核接入",
    revision: 1,
    closedAt: null,
  });
});

test("preserves closed and recovered lifecycle metadata", () => {
  const result = projectFocusFrameQuery(projection("closed"));
  assert.equal(result.lifecycleStatus, "closed");
  assert.equal(result.lifecycleLabel, "已关闭");
  assert.equal(result.revision, 2);
  assert.equal(result.closedAt, "2026-08-25T00:01:00Z");
});

test("reports an available focused context without re-projecting its contents", () => {
  const focusedContext = {
    contractVersion: "mindscape.focused-context.v1",
    focusFrame: frame,
    contextSnapshot: {
      id: "snapshot-1",
      conversationId: "conversation-1",
      parentNodeId: null,
      branchType: "continues" as const,
      currentInput: "当前问题",
      selectedMessages: [],
      selectedImportRefs: [],
      explicitConstraints: [],
      omittedMessages: [],
      systemContractVersion: "mindscape.context.v1",
      estimatedTokens: 12,
      createdAt: "2026-08-25T00:01:00Z",
    },
    selectedMemoryRefs: [],
    omittedMemoryRefs: [],
    knowledgeContext: null,
  };
  const result = projectFocusFrameQuery(projection("active", focusedContext));
  assert.equal(result?.contextState, "availableWithoutKnowledge");
  assert.equal(result?.contextLabel, "已生成快照，暂无知识引用");
});

test("labels a persisted focused context with knowledge without re-ranking it", () => {
  const focusedContext = {
    contractVersion: "mindscape.focused-context.v1",
    focusFrame: frame,
    contextSnapshot: {
      id: "snapshot-knowledge-1",
      conversationId: "conversation-1",
      parentNodeId: null,
      branchType: "continues" as const,
      currentInput: "当前问题",
      selectedMessages: [],
      selectedImportRefs: [],
      explicitConstraints: [],
      omittedMessages: [],
      systemContractVersion: "mindscape.context.v1",
      estimatedTokens: 18,
      createdAt: "2026-08-25T00:01:00Z",
    },
    selectedMemoryRefs: [],
    omittedMemoryRefs: [],
    knowledgeContext: {
      contractVersion: "mindscape.knowledge-context.v1",
      retrievalVersion: "fts-v1",
      estimatedTokens: 7,
      selected: [{
        entityId: "entity-1",
        status: "confirmed" as const,
        scope: { type: "focusFrame" as const, workspaceId: "workspace-1", conversationId: "conversation-1", focusFrameId: "focus-1" },
        revision: 1,
        evidence: [],
        retrievalScore: 0.8,
        estimatedTokens: 7,
      }],
      omitted: [{ referenceId: "entity-2", reason: "excludedByFocusFrame" }],
    },
  };
  const result = projectFocusFrameQuery(projection("active", focusedContext));
  assert.equal(result?.contextState, "availableWithKnowledge");
  assert.equal(result?.contextLabel, "已生成知识上下文");
});

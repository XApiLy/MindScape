import assert from "node:assert/strict";
import test from "node:test";
import { projectBranchMemoryAudit } from "./branchMemoryAudit.ts";
import type { CanvasFocusFrameQueryProjection } from "./canvasM2Projection.ts";

function query(
  focusedContext: CanvasFocusFrameQueryProjection["focusedContext"],
): CanvasFocusFrameQueryProjection {
  return {
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      focusFrame: {
        contractVersion: "mindscape.focus.v1",
        id: "focus-1",
        conversationId: "conversation-1",
        parentNodeId: "node-1",
        objective: "审计任务分支",
        activeWorkItem: "CAN-M2-008",
        contextPolicy: "branchFromNode",
        memoryScope: {
          branchKind: "task",
          inheritRefs: ["entity-inherit"],
          localRefs: ["entity-local"],
          excludeRefs: ["entity-excluded"],
          promoteRefs: ["entity-promote"],
        },
        includeRefs: ["entity-inherit", "entity-local"],
        excludeRefs: ["entity-excluded"],
        memoryVersion: 4,
        createdAt: "2026-08-29T00:00:00Z",
      },
      status: "active",
      revision: 2,
      updatedAt: "2026-08-29T00:01:00Z",
      closedAt: null,
    },
    focusedContext,
    focusedContextState: focusedContext ? "availableWithoutKnowledge" : "unavailable",
  };
}

test("preserves declared branch memory groups without applying inheritance rules", () => {
  const source = query(null);
  const audit = projectBranchMemoryAudit(source);

  assert.equal(audit.branchKind, "task");
  assert.equal(audit.memoryVersion, 4);
  assert.equal(audit.contextPolicy, "branchFromNode");
  assert.deepEqual(audit.declared, {
    inheritRefs: ["entity-inherit"],
    localRefs: ["entity-local"],
    excludeRefs: ["entity-excluded"],
    promoteRefs: ["entity-promote"],
  });
  assert.equal(audit.promotionDeclarationState, "declared");
  assert.equal(audit.frozen.state, "unavailable");
});

test("keeps frozen selected and omitted references separate with original reasons", () => {
  const source = query({
    snapshotId: "snapshot-1",
    focusFrameId: "focus-1",
    selectedMemoryRefs: ["entity-inherit", "entity-local"],
    omittedMemoryRefs: [
      { referenceId: "entity-excluded", reason: "excludedByFocusFrame" },
    ],
    knowledgeContext: null,
  });
  const audit = projectBranchMemoryAudit(source);

  assert.deepEqual(audit.frozen.selectedRefs, ["entity-inherit", "entity-local"]);
  assert.deepEqual(audit.frozen.omittedRefs, [
    { referenceId: "entity-excluded", reason: "excludedByFocusFrame" },
  ]);
  assert.notStrictEqual(audit.frozen.selectedRefs, source.focusedContext?.selectedMemoryRefs);
  assert.notStrictEqual(audit.frozen.omittedRefs, source.focusedContext?.omittedMemoryRefs);
});

test("does not claim a promotion projection when no refs are declared", () => {
  const source = query(null);
  source.lifecycle.focusFrame.memoryScope.promoteRefs = [];

  assert.equal(projectBranchMemoryAudit(source).promotionDeclarationState, "noneDeclared");
});

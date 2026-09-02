import assert from "node:assert/strict";
import test from "node:test";
import type { CanvasFocusFrameQueryProjection } from "../canvas/canvasM2Projection.ts";
import { buildImportKnowledgeProposalTargets } from "./importKnowledgeProposalTargets.ts";

function focusQuery({
  id,
  conversationId = "conversation-1",
  status = "active",
  branchKind = "task",
}: {
  id: string;
  conversationId?: string;
  status?: "active" | "closed";
  branchKind?: "mainline" | "exploration" | "task" | "retrospective";
}): CanvasFocusFrameQueryProjection {
  return {
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      focusFrame: {
        contractVersion: "mindscape.focus.v1",
        id,
        conversationId,
        parentNodeId: branchKind === "mainline" ? null : "node-1",
        objective: `目标 ${id}`,
        activeWorkItem: null,
        contextPolicy: "focusNew",
        memoryScope: {
          branchKind,
          inheritRefs: [],
          localRefs: [],
          excludeRefs: [],
          promoteRefs: [],
        },
        includeRefs: [],
        excludeRefs: [],
        memoryVersion: 1,
        createdAt: "2026-09-02T00:00:00Z",
      },
      status,
      revision: 1,
      updatedAt: "2026-09-02T00:00:00Z",
      closedAt: status === "closed" ? "2026-09-02T01:00:00Z" : null,
    },
    focusedContext: null,
    focusedContextState: "unavailable",
  };
}

test("proposal destinations keep the conversation first and include active local branches", () => {
  const targets = buildImportKnowledgeProposalTargets(
    { id: "conversation-1", workspaceId: "workspace-1" },
    [focusQuery({ id: "focus-1" })],
  );

  assert.deepEqual(targets.map((target) => target.id), [
    "conversation:conversation-1",
    "focusFrame:focus-1",
  ]);
  assert.equal(targets[1]?.scope.type, "focusFrame");
});

test("proposal destinations exclude stale conversations closed frames and mainline", () => {
  const targets = buildImportKnowledgeProposalTargets(
    { id: "conversation-1", workspaceId: "workspace-1" },
    [
      focusQuery({ id: "focus-other", conversationId: "conversation-2" }),
      focusQuery({ id: "focus-closed", status: "closed" }),
      focusQuery({ id: "focus-mainline", branchKind: "mainline" }),
    ],
  );

  assert.deepEqual(targets.map((target) => target.id), ["conversation:conversation-1"]);
});

test("proposal destinations deduplicate repeated node projections for the same frame", () => {
  const repeated = focusQuery({ id: "focus-1" });
  const targets = buildImportKnowledgeProposalTargets(
    { id: "conversation-1", workspaceId: "workspace-1" },
    [repeated, repeated],
  );

  assert.equal(targets.filter((target) => target.id === "focusFrame:focus-1").length, 1);
});

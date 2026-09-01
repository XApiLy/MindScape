import assert from "node:assert/strict";
import test from "node:test";
import type { CanvasNodeProjection } from "./graphProjection.ts";
import { projectCanvasBranchTrail } from "./canvasBranchNavigation.ts";

const timestamp = "2026-08-27T00:00:00Z";

function node(
  id: string,
  parentNodeId: string | null,
  branchType: CanvasNodeProjection["branchType"],
  origin: CanvasNodeProjection["origin"] = { kind: "localRun" },
): CanvasNodeProjection {
  return {
    id,
    title: `节点 ${id}`,
    question: id,
    answer: null,
    providerId: null,
    modelId: null,
    runState: "completed",
    runError: null,
    partialContentRetained: false,
    branchType,
    origin,
    parentNodeId,
    createdAt: timestamp,
    position: { x: 0, y: 0 },
  };
}

test("projects the selected branch trail from explicit parent links", () => {
  const nodes = [
    node("root", null, "continues"),
    node("deep", "root", "deepens"),
    node("task", "deep", "diverges"),
    node("unrelated", "root", "reframes"),
  ];

  assert.deepEqual(projectCanvasBranchTrail(nodes, "task"), [{
    nodeId: "root",
    title: "节点 root",
    parentNodeId: null,
    branchType: "continues",
    origin: "localRun",
    isCurrent: false,
  }, {
    nodeId: "deep",
    title: "节点 deep",
    parentNodeId: "root",
    branchType: "deepens",
    origin: "localRun",
    isCurrent: false,
  }, {
    nodeId: "task",
    title: "节点 task",
    parentNodeId: "deep",
    branchType: "diverges",
    origin: "localRun",
    isCurrent: true,
  }]);
});

test("preserves imported origins and stops safely at missing parents or cycles", () => {
  const imported = node("imported", "missing", "importedFrom", { kind: "importedSource" });
  assert.deepEqual(projectCanvasBranchTrail([imported], "imported"), [{
    nodeId: "imported",
    title: "节点 imported",
    parentNodeId: "missing",
    branchType: "importedFrom",
    origin: "importedSource",
    isCurrent: true,
  }]);

  const cycleA = node("cycle-a", "cycle-b", "deepens");
  const cycleB = node("cycle-b", "cycle-a", "diverges");
  const cycle = projectCanvasBranchTrail([cycleA, cycleB], "cycle-a");
  assert.equal(cycle.length, 2);
  assert.equal(new Set(cycle.map((item) => item.nodeId)).size, 2);
  assert.equal(cycle.at(-1)?.isCurrent, true);
  assert.deepEqual(projectCanvasBranchTrail([cycleA], null), []);
  assert.deepEqual(projectCanvasBranchTrail([cycleA], "unknown"), []);
});

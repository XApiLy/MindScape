import type { BranchType } from "../domain";
import type { CanvasNodeOriginProjection, CanvasNodeProjection } from "./graphProjection";

export type CanvasBranchTrailItem = {
  nodeId: string;
  title: string;
  parentNodeId: string | null;
  branchType: BranchType;
  origin: CanvasNodeOriginProjection["kind"];
  isCurrent: boolean;
};

/**
 * Projects the explicit parent chain for the selected canvas node.
 *
 * This is navigation only. It does not infer a knowledge relationship,
 * FocusFrame scope or "mainline" from coordinates. Missing parents and
 * malformed cycles stop the trail safely at the last authoritative node.
 */
export function projectCanvasBranchTrail(
  nodes: readonly CanvasNodeProjection[],
  selectedNodeId: string | null,
): CanvasBranchTrailItem[] {
  if (!selectedNodeId) return [];

  const nodesById = new Map(nodes.map((node) => [node.id, node] as const));
  const trail: CanvasNodeProjection[] = [];
  const visited = new Set<string>();
  let current = nodesById.get(selectedNodeId);

  while (current && !visited.has(current.id)) {
    visited.add(current.id);
    trail.unshift(current);
    current = current.parentNodeId ? nodesById.get(current.parentNodeId) : undefined;
  }

  return trail.map((node) => ({
    nodeId: node.id,
    title: node.title,
    parentNodeId: node.parentNodeId,
    branchType: node.branchType,
    origin: node.origin.kind,
    isCurrent: node.id === selectedNodeId,
  }));
}

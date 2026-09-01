import type { CanvasFocusFrameQueryProjection } from "./canvasM2Projection.ts";

export type CanvasBranchMemoryAudit = {
  branchKind: CanvasFocusFrameQueryProjection["lifecycle"]["focusFrame"]["memoryScope"]["branchKind"];
  memoryVersion: number;
  contextPolicy: CanvasFocusFrameQueryProjection["lifecycle"]["focusFrame"]["contextPolicy"];
  declared: {
    inheritRefs: readonly string[];
    localRefs: readonly string[];
    excludeRefs: readonly string[];
    promoteRefs: readonly string[];
  };
  frozen: {
    state: CanvasFocusFrameQueryProjection["focusedContextState"];
    selectedRefs: readonly string[];
    omittedRefs: ReadonlyArray<{ referenceId: string; reason: string }>;
  };
  promotionDeclarationState: "noneDeclared" | "declared";
};

/**
 * Builds a read-only audit view from kernel-owned FocusFrame facts. It does not
 * apply inheritance rules or turn promoteRefs into confirmed promotion candidates.
 */
export function projectBranchMemoryAudit(
  query: CanvasFocusFrameQueryProjection,
): CanvasBranchMemoryAudit {
  const frame = query.lifecycle.focusFrame;
  const promoteRefs = [...frame.memoryScope.promoteRefs];

  return {
    branchKind: frame.memoryScope.branchKind,
    memoryVersion: frame.memoryVersion,
    contextPolicy: frame.contextPolicy,
    declared: {
      inheritRefs: [...frame.memoryScope.inheritRefs],
      localRefs: [...frame.memoryScope.localRefs],
      excludeRefs: [...frame.memoryScope.excludeRefs],
      promoteRefs,
    },
    frozen: {
      state: query.focusedContextState,
      selectedRefs: [...(query.focusedContext?.selectedMemoryRefs ?? [])],
      omittedRefs: (query.focusedContext?.omittedMemoryRefs ?? []).map((reference) => ({
        ...reference,
      })),
    },
    promotionDeclarationState: promoteRefs.length
      ? "declared"
      : "noneDeclared",
  };
}

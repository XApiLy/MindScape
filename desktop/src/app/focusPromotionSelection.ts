import type { CanvasFocusFrameQueryProjection } from "../canvas/canvasM2Projection";
import type { KnowledgeEntity } from "../domain";

/**
 * Narrows the kernel-returned conversation inventory for presentation only.
 * The generation command reloads and validates every selected entity again.
 */
export function selectableFocusPromotionEntities(
  entities: readonly KnowledgeEntity[],
  query: CanvasFocusFrameQueryProjection | null | undefined,
): KnowledgeEntity[] {
  if (!query || query.lifecycle.status !== "active") return [];

  const frame = query.lifecycle.focusFrame;
  return entities
    .filter((entity) => (
      entity.scope.type === "focusFrame"
      && entity.scope.conversationId === frame.conversationId
      && entity.scope.focusFrameId === frame.id
      && (entity.status === "candidate" || entity.status === "inferred")
      && entity.evidence.length > 0
    ))
    .sort((left, right) => left.id.localeCompare(right.id));
}

export function isFocusPromotionSelectionChanged(
  selectedRefs: readonly string[],
  currentRefs: readonly string[],
): boolean {
  if (selectedRefs.length !== currentRefs.length) return true;
  const selected = [...selectedRefs].sort();
  const current = [...currentRefs].sort();
  return selected.some((reference, index) => reference !== current[index]);
}

/**
 * Keeps only references present in the selectable inventory returned by the
 * kernel. Eligibility is deliberately not derived in React.
 */
export function reconcileFocusPromotionSelection(
  currentRefs: readonly string[],
  selectableRefs: readonly string[],
): string[] {
  const selectable = new Set(selectableRefs);
  return currentRefs.filter((reference) => selectable.has(reference)).sort();
}

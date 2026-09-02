import assert from "node:assert/strict";
import test from "node:test";
import type { CanvasFocusFrameQueryProjection } from "../canvas/canvasM2Projection.ts";
import type { KnowledgeEntity } from "../domain/index.ts";
import {
  isFocusPromotionSelectionChanged,
  reconcileFocusPromotionSelection,
  selectableFocusPromotionEntities,
} from "./focusPromotionSelection.ts";

const activeQuery = {
  lifecycle: {
    status: "active",
    focusFrame: {
      id: "focus-1",
      conversationId: "conversation-1",
    },
  },
} as CanvasFocusFrameQueryProjection;

function candidate(
  id: string,
  overrides: Partial<KnowledgeEntity> = {},
): KnowledgeEntity {
  return {
    id,
    status: "candidate",
    scope: {
      type: "focusFrame",
      workspaceId: "workspace-1",
      conversationId: "conversation-1",
      focusFrameId: "focus-1",
    },
    evidence: [{ id: `evidence-${id}` }],
    ...overrides,
  } as KnowledgeEntity;
}

test("presentation selector only shows grounded candidate-like entities from the active frame", () => {
  assert.deepEqual(
    selectableFocusPromotionEntities([
      candidate("candidate-b"),
      candidate("confirmed", { status: "confirmed" }),
      candidate("ungrounded", { evidence: [] }),
      candidate("candidate-a", { status: "inferred" }),
      candidate("other-frame", {
        scope: {
          type: "focusFrame",
          workspaceId: "workspace-1",
          conversationId: "conversation-1",
          focusFrameId: "focus-2",
        },
      }),
    ], activeQuery).map((entity) => entity.id),
    ["candidate-a", "candidate-b"],
  );
});

test("selection comparison is stable regardless of checkbox order", () => {
  assert.equal(isFocusPromotionSelectionChanged(["b", "a"], ["a", "b"]), false);
  assert.equal(isFocusPromotionSelectionChanged(["a"], ["a", "b"]), true);
  assert.equal(isFocusPromotionSelectionChanged(["a", "c"], ["a", "b"]), true);
});

test("selection reconciliation consumes the kernel inventory without deriving eligibility", () => {
  assert.deepEqual(
    reconcileFocusPromotionSelection(
      ["candidate-c", "candidate-a", "not-returned"],
      ["candidate-a", "candidate-b", "candidate-c"],
    ),
    ["candidate-a", "candidate-c"],
  );
});

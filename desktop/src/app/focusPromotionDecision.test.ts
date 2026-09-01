import assert from "node:assert/strict";
import test from "node:test";
import { buildFocusPromotionDecisionInput } from "./focusPromotionDecision.ts";
import type {
  FocusFrameQueryProjection,
  FocusPromotionCandidateSet,
  KnowledgeEntity,
} from "../domain/index.ts";

const candidateSet: FocusPromotionCandidateSet = {
  contractVersion: "mindscape.focus.v1",
  focusFrameId: "focus-1",
  conversationId: "conversation-1",
  branchKind: "task",
  memoryVersion: 7,
  candidateRefs: ["entity-1"],
};

const entity: KnowledgeEntity = {
  contractVersion: "mindscape.knowledge.v1",
  id: "entity-1",
  kind: "decision",
  name: "保留原子回流",
  aliases: [],
  scope: {
    type: "focusFrame",
    workspaceId: "workspace-1",
    conversationId: "conversation-1",
    focusFrameId: "focus-1",
  },
  status: "candidate",
  revision: 3,
  evidence: [],
  generator: { kind: "model", generatorId: "model-1", generatorVersion: "v1" },
  createdAt: "2026-08-31T00:00:00Z",
  updatedAt: "2026-08-31T00:00:00Z",
};

const query = {
  contractVersion: "mindscape.focus-query.v1",
  lifecycle: {
    contractVersion: "mindscape.focus-lifecycle.v1",
    frame: {
      contractVersion: "mindscape.focus.v1",
      id: "focus-1",
      conversationId: "conversation-1",
      parentNodeId: "node-1",
      objective: "验证回流",
      activeWorkItem: null,
      contextPolicy: "branchFromNode",
      memoryScope: {
        branchKind: "task",
        inheritRefs: [],
        localRefs: ["entity-1"],
        excludeRefs: [],
        promoteRefs: ["entity-1"],
      },
      includeRefs: [],
      excludeRefs: [],
      memoryVersion: 7,
      createdAt: "2026-08-31T00:00:00Z",
    },
    status: "closed",
    revision: 4,
    updatedAt: "2026-08-31T00:01:00Z",
    closedAt: "2026-08-31T00:01:00Z",
  },
  focusedContext: null,
} satisfies FocusFrameQueryProjection;

const identity = {
  decisionId: "decision-1",
  promotedEntityId: "entity-promoted-1",
  decidedAt: "2026-08-31T00:02:00Z",
};

test("builds an optimistic create-only confirmation without a target scope", () => {
  const input = buildFocusPromotionDecisionInput("confirm", candidateSet, query, entity, identity);
  assert.equal(input.expectedMemoryVersion, 7);
  assert.equal(input.expectedLifecycleRevision, 4);
  assert.equal(input.expectedEntityRevision, 3);
  assert.equal(input.expectedDecisionRevision, 0);
  assert.equal(input.targetScope, null);
  assert.equal(input.promotedEntityId, null);
});

test("promotes into the current conversation without mutating the source identity", () => {
  const input = buildFocusPromotionDecisionInput("promote", candidateSet, query, entity, identity);
  assert.deepEqual(input.targetScope, {
    type: "conversation",
    workspaceId: "workspace-1",
    conversationId: "conversation-1",
  });
  assert.equal(input.candidateRef, "entity-1");
  assert.equal(input.promotedEntityId, "entity-promoted-1");
});

test("reject and delete retain version gates but never carry promotion targets", () => {
  for (const action of ["reject", "delete"] as const) {
    const input = buildFocusPromotionDecisionInput(action, candidateSet, query, entity, identity);
    assert.equal(input.action, action);
    assert.equal(input.targetScope, null);
    assert.equal(input.promotedEntityId, null);
    assert.equal(input.expectedEntityRevision, entity.revision);
  }
});

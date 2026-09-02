import { strict as assert } from "node:assert";
import test from "node:test";
import type { KnowledgeEntity, KnowledgeRelation } from "../domain/index.ts";
import { summarizeKnowledgeInventory } from "./knowledgeInventory.ts";

const scope = { type: "conversation", workspaceId: "w1", conversationId: "c1" } as const;
const generator = { kind: "user", generatorId: "test", generatorVersion: "1" } as const;

function entity(id: string, status: KnowledgeEntity["status"]): KnowledgeEntity {
  return {
    contractVersion: "mindscape.knowledge-entity.v1",
    id,
    kind: "topic",
    name: id,
    aliases: [],
    scope,
    status,
    revision: 1,
    evidence: [],
    generator,
    createdAt: "2026-08-26T00:00:00.000Z",
    updatedAt: "2026-08-26T00:00:00.000Z",
  };
}

function relation(id: string, status: KnowledgeRelation["status"]): KnowledgeRelation {
  return {
    contractVersion: "mindscape.knowledge-relation.v1",
    id,
    kind: "relatedTo",
    sourceEntityId: "e1",
    targetEntityId: "e2",
    scope,
    status,
    revision: 1,
    evidence: [],
    generator,
    createdAt: "2026-08-26T00:00:00.000Z",
    updatedAt: "2026-08-26T00:00:00.000Z",
  };
}

test("summarizes query results without changing kernel ordering or status", () => {
  const summary = summarizeKnowledgeInventory(
    [entity("e1", "candidate"), entity("e2", "confirmed")],
    [relation("r1", "rejected")],
  );

  assert.deepEqual(summary, {
    entityCount: 2,
    relationCount: 1,
    candidateCount: 1,
    confirmedCount: 1,
    rejectedCount: 1,
    statusCounts: {
      candidate: 1,
      inferred: 0,
      confirmed: 1,
      rejected: 1,
      superseded: 0,
      stale: 0,
    },
  });
});

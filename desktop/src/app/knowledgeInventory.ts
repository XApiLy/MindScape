import type { KnowledgeEntity, KnowledgeRelation, KnowledgeStatus } from "../domain";

export type KnowledgeInventorySummary = {
  entityCount: number;
  relationCount: number;
  candidateCount: number;
  confirmedCount: number;
  rejectedCount: number;
  statusCounts: Readonly<Record<KnowledgeStatus, number>>;
};

const EMPTY_STATUS_COUNTS: Readonly<Record<KnowledgeStatus, number>> = {
  candidate: 0,
  inferred: 0,
  confirmed: 0,
  rejected: 0,
  superseded: 0,
  stale: 0,
};

/**
 * Summarizes already-queryable knowledge objects for a read-only UI indicator.
 * It deliberately does not apply scope, status, or retrieval rules; those
 * semantics remain owned by the kernel/query layer.
 */
export function summarizeKnowledgeInventory(
  entities: readonly KnowledgeEntity[],
  relations: readonly KnowledgeRelation[],
): KnowledgeInventorySummary {
  const statusCounts = { ...EMPTY_STATUS_COUNTS };
  for (const item of [...entities, ...relations]) {
    statusCounts[item.status] += 1;
  }
  return {
    entityCount: entities.length,
    relationCount: relations.length,
    candidateCount: statusCounts.candidate,
    confirmedCount: statusCounts.confirmed,
    rejectedCount: statusCounts.rejected,
    statusCounts,
  };
}

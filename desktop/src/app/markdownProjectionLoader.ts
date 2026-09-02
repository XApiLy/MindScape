import type { MarkdownProjection } from "../domain/knowledge.ts";

export type MarkdownProjectionFetcher = (
  entityId: string,
) => Promise<MarkdownProjection[]>;

export type MarkdownProjectionLoadResult = {
  projectionsByEntityId: ReadonlyMap<string, readonly MarkdownProjection[]>;
  errorsByEntityId: ReadonlyMap<string, unknown>;
};

type MarkdownProjectionLoadAttempt =
  | { kind: "success"; entityId: string; projections: MarkdownProjection[] }
  | { kind: "error"; entityId: string; error: unknown };

/**
 * Loads projection history for explicit entity IDs. Calls start together and
 * failures stay scoped to one entity so the rest of the knowledge UI remains usable.
 */
export async function loadMarkdownProjections(
  entityIds: readonly string[],
  fetchProjections: MarkdownProjectionFetcher,
): Promise<MarkdownProjectionLoadResult> {
  const ids = [...new Set(entityIds.map((id) => id.trim()).filter(Boolean))];
  const projectionsByEntityId = new Map<string, readonly MarkdownProjection[]>();
  const errorsByEntityId = new Map<string, unknown>();
  const attempts: MarkdownProjectionLoadAttempt[] = await Promise.all(ids.map(async (entityId) => {
    try {
      return {
        kind: "success" as const,
        entityId,
        projections: await fetchProjections(entityId),
      };
    } catch (error) {
      return { kind: "error" as const, entityId, error };
    }
  }));

  for (const attempt of attempts) {
    if (attempt.kind === "success") {
      projectionsByEntityId.set(attempt.entityId, attempt.projections);
    } else {
      errorsByEntityId.set(attempt.entityId, attempt.error);
    }
  }

  return { projectionsByEntityId, errorsByEntityId };
}

/** Keeps the command-returned newest revision first without rewriting history. */
export function prependMarkdownProjectionRevision(
  history: readonly MarkdownProjection[] | undefined,
  projection: MarkdownProjection,
): readonly MarkdownProjection[] {
  return [
    projection,
    ...(history ?? []).filter((item) => !(
      item.id === projection.id
      && item.projectionRevision === projection.projectionRevision
    )),
  ];
}

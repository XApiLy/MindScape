import type { FocusFrameQueryProjection } from "../domain/focus.ts";

export type FocusFrameQueryFetcher = (
  focusFrameId: string,
) => Promise<FocusFrameQueryProjection>;

export type FocusFrameQueryLoadResult = {
  projections: ReadonlyMap<string, FocusFrameQueryProjection>;
  errors: ReadonlyMap<string, unknown>;
};

type FocusFrameQueryLoadAttempt =
  | { kind: "success"; id: string; projection: FocusFrameQueryProjection }
  | { kind: "error"; id: string; error: unknown };

/**
 * Loads only caller-provided FocusFrame IDs. It never discovers IDs from
 * canvas/React state and keeps per-frame failures isolated for recoverable UI.
 */
export async function loadFocusFrameQueries(
  focusFrameIds: readonly string[],
  fetchQuery: FocusFrameQueryFetcher,
): Promise<FocusFrameQueryLoadResult> {
  const ids = [...new Set(focusFrameIds.map((id) => id.trim()).filter(Boolean))];
  const projections = new Map<string, FocusFrameQueryProjection>();
  const errors = new Map<string, unknown>();
  const results: FocusFrameQueryLoadAttempt[] = await Promise.all(ids.map(async (id) => {
    try {
      return { kind: "success" as const, id, projection: await fetchQuery(id) };
    } catch (error) {
      return { kind: "error" as const, id, error };
    }
  }));

  for (const result of results) {
    if (result.kind === "success") projections.set(result.id, result.projection);
    else errors.set(result.id, result.error);
  }
  return { projections, errors };
}

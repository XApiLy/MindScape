import assert from "node:assert/strict";
import test from "node:test";
import {
  loadMarkdownProjections,
  prependMarkdownProjectionRevision,
} from "./markdownProjectionLoader.ts";
import type { MarkdownProjection } from "../domain/knowledge.ts";

function projection(entityId: string, projectionRevision: number): MarkdownProjection {
  return {
    contractVersion: "mindscape.markdown-projection.v1",
    id: `projection-${entityId}-${projectionRevision}`,
    targetEntityId: entityId,
    relativePath: `entities/${entityId}.md`,
    entityRevision: 3,
    projectionRevision,
    contentHash: `sha256:${entityId}-${projectionRevision}`,
    frontmatterVersion: "mindscape.frontmatter.v1",
    createdAt: "2026-08-28T04:00:00Z",
  };
}

test("deduplicates explicit entity IDs and skips blank IDs", async () => {
  const calls: string[] = [];
  const result = await loadMarkdownProjections(
    [" entity-1 ", "entity-1", "", "   "],
    async (entityId) => {
      calls.push(entityId);
      return [projection(entityId, 2), projection(entityId, 1)];
    },
  );

  assert.deepEqual(calls, ["entity-1"]);
  assert.deepEqual(
    result.projectionsByEntityId.get("entity-1")?.map((item) => item.projectionRevision),
    [2, 1],
  );
  assert.equal(result.errorsByEntityId.size, 0);
});

test("starts all entity requests before waiting for any one result", async () => {
  const started: string[] = [];
  const resolvers = new Map<string, (value: MarkdownProjection[]) => void>();
  const loading = loadMarkdownProjections(["entity-1", "entity-2"], (entityId) => {
    started.push(entityId);
    return new Promise((resolve) => resolvers.set(entityId, resolve));
  });

  await Promise.resolve();
  assert.deepEqual(started, ["entity-1", "entity-2"]);
  resolvers.get("entity-2")?.([projection("entity-2", 1)]);
  resolvers.get("entity-1")?.([projection("entity-1", 1)]);
  const result = await loading;
  assert.equal(result.projectionsByEntityId.size, 2);
});

test("isolates one entity failure and keeps successful or empty histories", async () => {
  const result = await loadMarkdownProjections(
    ["entity-ok", "entity-empty", "entity-fail"],
    async (entityId) => {
      if (entityId === "entity-fail") throw new Error("vault unavailable");
      return entityId === "entity-empty" ? [] : [projection(entityId, 1)];
    },
  );

  assert.equal(result.projectionsByEntityId.get("entity-ok")?.length, 1);
  assert.deepEqual(result.projectionsByEntityId.get("entity-empty"), []);
  assert.equal(result.projectionsByEntityId.has("entity-fail"), false);
  assert.match(String(result.errorsByEntityId.get("entity-fail")), /vault unavailable/);
});

test("does not call the kernel when there are no entity IDs", async () => {
  let calls = 0;
  const result = await loadMarkdownProjections([], async () => {
    calls += 1;
    return [];
  });

  assert.equal(calls, 0);
  assert.equal(result.projectionsByEntityId.size, 0);
  assert.equal(result.errorsByEntityId.size, 0);
});

test("prepends a command-returned revision without duplicating the same revision", () => {
  const latest = projection("entity-1", 3);
  const history = [projection("entity-1", 2), projection("entity-1", 1)];

  assert.deepEqual(
    prependMarkdownProjectionRevision(history, latest).map((item) => item.projectionRevision),
    [3, 2, 1],
  );
  assert.deepEqual(
    prependMarkdownProjectionRevision([latest, ...history], latest).map((item) => item.projectionRevision),
    [3, 2, 1],
  );
});

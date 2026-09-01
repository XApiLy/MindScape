import assert from "node:assert/strict";
import test from "node:test";
import { loadFocusFrameQueries } from "./focusFrameQueryLoader.ts";
import type { FocusFrameQueryProjection } from "../domain/focus.ts";

function query(id: string): FocusFrameQueryProjection {
  return {
    contractVersion: "mindscape.focus-query.v1",
    lifecycle: {
      contractVersion: "mindscape.focus-lifecycle.v1",
      frame: {
        contractVersion: "mindscape.focus.v1",
        id,
        conversationId: "conversation-1",
        parentNodeId: null,
        objective: "测试",
        activeWorkItem: null,
        contextPolicy: "continueCurrent",
        memoryScope: {
          branchKind: "mainline",
          inheritRefs: [],
          localRefs: [],
          excludeRefs: [],
          promoteRefs: [],
        },
        includeRefs: [],
        excludeRefs: [],
        memoryVersion: 1,
        createdAt: "2026-08-25T00:00:00Z",
      },
      status: "active",
      revision: 1,
      updatedAt: "2026-08-25T00:00:00Z",
      closedAt: null,
    },
    focusedContext: null,
  };
}

test("deduplicates caller-provided IDs and ignores blank IDs", async () => {
  const calls: string[] = [];
  const result = await loadFocusFrameQueries([" focus-1 ", "focus-1", "   "], async (id) => {
    calls.push(id);
    return query(id);
  });
  assert.deepEqual(calls, ["focus-1"]);
  assert.equal(result.projections.size, 1);
  assert.equal(result.errors.size, 0);
});

test("isolates one query failure while retaining successful projections", async () => {
  const result = await loadFocusFrameQueries(["focus-ok", "focus-fail"], async (id) => {
    if (id === "focus-fail") throw new Error("query unavailable");
    return query(id);
  });
  assert.equal(result.projections.has("focus-ok"), true);
  assert.equal(result.errors.has("focus-fail"), true);
});

test("does not call the kernel for an empty ID list", async () => {
  let calls = 0;
  const result = await loadFocusFrameQueries([], async () => {
    calls += 1;
    return query("unexpected");
  });
  assert.equal(calls, 0);
  assert.equal(result.projections.size, 0);
  assert.equal(result.errors.size, 0);
});

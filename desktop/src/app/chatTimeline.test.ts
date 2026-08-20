import assert from "node:assert/strict";
import test from "node:test";
import { projectChatTimeline } from "./chatTimeline.ts";

test("replaces a persisted run node instead of rendering the same run twice", () => {
  const run = { nodeId: "node-2", status: "cancelled" };
  const timeline = projectChatTimeline(
    [{ id: "node-1" }, { id: "node-2" }],
    run,
  );

  assert.deepEqual(timeline, [
    { kind: "node", node: { id: "node-1" } },
    { kind: "run", run },
  ]);
});

test("appends a transient run that is not in the conversation graph yet", () => {
  const run = { nodeId: "node-pending", status: "streaming" };
  const timeline = projectChatTimeline([{ id: "node-1" }], run);

  assert.deepEqual(timeline, [
    { kind: "node", node: { id: "node-1" } },
    { kind: "run", run },
  ]);
});

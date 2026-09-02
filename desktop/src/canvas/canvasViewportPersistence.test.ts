import assert from "node:assert/strict";
import test from "node:test";
import type { SaveCanvasViewportInput } from "../domain/commands.ts";
import {
  CanvasViewportPersistence,
  loadCanvasViewport,
} from "./canvasViewportPersistence.ts";

type ScheduledTimer = {
  callback: () => void;
  delayMs: number;
};

function createHarness() {
  let now = 1_000;
  const timers: ScheduledTimer[] = [];
  const writes: SaveCanvasViewportInput[] = [];
  const errors: unknown[] = [];
  const persistence = new CanvasViewportPersistence(
    async (input) => {
      writes.push(input);
    },
    {
      intervalMs: 250,
      now: () => now,
      setTimer: (callback, delayMs) => {
        const timer = { callback, delayMs } as ScheduledTimer & ReturnType<typeof setTimeout>;
        timers.push(timer);
        return timer;
      },
      clearTimer: (timer) => {
        const index = timers.indexOf(timer as unknown as ScheduledTimer);
        if (index >= 0) timers.splice(index, 1);
      },
      onError: (error) => errors.push(error),
    },
  );

  return {
    persistence,
    writes,
    errors,
    timers,
    advance(ms: number) {
      now += ms;
    },
    runNextTimer() {
      const timer = timers.shift();
      assert.ok(timer);
      now += timer.delayMs;
      timer.callback();
    },
  };
}

test("persists the first viewport immediately and coalesces rapid changes", async () => {
  const harness = createHarness();

  harness.persistence.schedule("conversation-a", { x: 10, y: 20, zoom: 0.8 });
  harness.advance(20);
  harness.persistence.schedule("conversation-a", { x: 30, y: 40, zoom: 0.9 });
  harness.persistence.schedule("conversation-a", { x: 50, y: 60, zoom: 1.1 });

  assert.equal(harness.timers.length, 1);
  harness.runNextTimer();
  await harness.persistence.flushAll();

  assert.deepEqual(harness.writes, [
    { conversationId: "conversation-a", x: 10, y: 20, zoom: 0.8 },
    { conversationId: "conversation-a", x: 50, y: 60, zoom: 1.1 },
  ]);
});

test("keeps scheduling and flush state isolated by conversation", async () => {
  const harness = createHarness();

  harness.persistence.schedule("conversation-a", { x: 10, y: 20, zoom: 0.8 });
  harness.persistence.schedule("conversation-b", { x: -10, y: -20, zoom: 1.2 });
  harness.advance(20);
  harness.persistence.schedule("conversation-a", { x: 80, y: 90, zoom: 1 });

  await harness.persistence.flush("conversation-a");
  await harness.persistence.flush("conversation-b");

  assert.deepEqual(harness.writes, [
    { conversationId: "conversation-a", x: 10, y: 20, zoom: 0.8 },
    { conversationId: "conversation-b", x: -10, y: -20, zoom: 1.2 },
    { conversationId: "conversation-a", x: 80, y: 90, zoom: 1 },
  ]);
});

test("reports a failed write and continues with the latest viewport", async () => {
  let attempts = 0;
  const writes: SaveCanvasViewportInput[] = [];
  const errors: unknown[] = [];
  const persistence = new CanvasViewportPersistence(
    async (input) => {
      attempts += 1;
      if (attempts === 1) throw new Error("temporary write failure");
      writes.push(input);
    },
    { intervalMs: 0, onError: (error) => errors.push(error) },
  );

  persistence.schedule("conversation-a", { x: 1, y: 2, zoom: 0.8 });
  persistence.schedule("conversation-a", { x: 3, y: 4, zoom: 1 });
  await persistence.flushAll();

  assert.equal(errors.length, 1);
  assert.deepEqual(writes, [
    { conversationId: "conversation-a", x: 3, y: 4, zoom: 1 },
  ]);
});

test("loads only the requested conversation viewport in a fresh session", async () => {
  const states = new Map([
    ["conversation-a", {
      conversationId: "conversation-a",
      x: 120,
      y: -60,
      zoom: 0.75,
      updatedAt: "2026-08-18T10:00:00.000Z",
    }],
    ["conversation-b", {
      conversationId: "conversation-b",
      x: -320,
      y: 180,
      zoom: 1.25,
      updatedAt: "2026-08-18T10:00:01.000Z",
    }],
  ]);
  const read = async (conversationId: string) => states.get(conversationId) ?? null;

  assert.deepEqual(await loadCanvasViewport(read, "conversation-b"), {
    x: -320,
    y: 180,
    zoom: 1.25,
  });
  assert.equal(await loadCanvasViewport(read, "conversation-missing"), null);
});

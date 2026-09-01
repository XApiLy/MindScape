import assert from "node:assert/strict";
import test from "node:test";
import { shouldFollowLatest } from "./chatScrollFollow.ts";

test("keeps follow mode while the reader remains near the latest content", () => {
  assert.equal(shouldFollowLatest({ scrollHeight: 1_000, scrollTop: 320, clientHeight: 600 }), true);
  assert.equal(shouldFollowLatest({ scrollHeight: 1_000, scrollTop: 304, clientHeight: 600 }), true);
});

test("locks the current reading position after the reader scrolls away", () => {
  assert.equal(shouldFollowLatest({ scrollHeight: 1_000, scrollTop: 200, clientHeight: 600 }), false);
});

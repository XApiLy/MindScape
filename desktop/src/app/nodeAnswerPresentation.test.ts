import assert from "node:assert/strict";
import test from "node:test";

import { presentMissingNodeAnswer } from "./nodeAnswerPresentation.ts";

test("does not present a completed empty response as recovery in progress", () => {
  assert.deepEqual(presentMissingNodeAnswer("completed"), {
    message: "本次运行已完成，但模型未返回可显示内容。",
    showSpinner: false,
  });
});

test("only active nodes show a waiting spinner", () => {
  assert.equal(presentMissingNodeAnswer("pending").showSpinner, true);
  assert.equal(presentMissingNodeAnswer("streaming").showSpinner, true);
  assert.equal(presentMissingNodeAnswer("cancelled").showSpinner, false);
  assert.equal(presentMissingNodeAnswer("failed").showSpinner, false);
});

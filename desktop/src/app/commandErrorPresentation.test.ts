import assert from "node:assert/strict";
import test from "node:test";
import { commandErrorMessage } from "./commandErrorPresentation.ts";

test("explains a context budget rejection without implying a failed persisted run", () => {
  const message = commandErrorMessage({
    code: "contextBudgetInvalid",
    safeMessage: "The requested output budget must leave room for model input context.",
    retryable: false,
  });
  assert.match(message, /上下文预算不足/);
  assert.match(message, /未创建残留节点或运行/);
});

test("routes preflight authentication errors back to safe settings actions", () => {
  const message = commandErrorMessage({
    code: "providerAuthentication",
    safeMessage: "The configured credential was rejected.",
    retryable: false,
  });
  assert.match(message, /替换凭据/);
  assert.match(message, /测试连接/);
});

test("turns provider invalid requests into executable run-profile guidance", () => {
  const message = commandErrorMessage({
    code: "providerInvalidRequest",
    safeMessage: "temperature cannot be combined with thinking mode.",
    retryable: false,
  });
  assert.match(message, /运行参数/);
  assert.match(message, /reasoning/);
  assert.doesNotMatch(message, /temperature cannot/);
});

test("preserves unknown safe command messages", () => {
  assert.equal(
    commandErrorMessage({ code: "futureCode", safeMessage: "Future safe message" }),
    "Future safe message",
  );
});

import assert from "node:assert/strict";
import test from "node:test";
import type { ModelRunEvent, ModelRunEventEnvelope, ProviderError } from "../domain/runtime.ts";
import {
  SUPPORTED_RUNTIME_CONTRACT_VERSION,
  createChatRunState,
  createChatRunStateFromProjection,
  presentProviderError,
  rejectChatRunCancellation,
  requestChatRunCancellation,
  reduceModelRunEnvelope,
} from "./chatRunState.ts";

function envelope(sequence: number, event: ModelRunEvent): ModelRunEventEnvelope {
  return {
    contractVersion: SUPPORTED_RUNTIME_CONTRACT_VERSION,
    eventId: `event-${sequence}`,
    runId: "run-1",
    nodeId: "node-1",
    providerId: "mock",
    modelId: "mock-stream-v1",
    sequence,
    occurredAt: "2026-08-14T10:00:00Z",
    event,
  };
}

function initialState() {
  return createChatRunState({
    runId: "run-1",
    nodeId: "node-1",
    prompt: "question",
    parentNodeId: null,
    branchType: "continues",
  });
}

const emptyUsage = {
  inputTokens: null,
  outputTokens: null,
  cachedInputTokens: null,
  costMicrounits: null,
};

test("reduces a complete streamed run deterministically", () => {
  let state = initialState();
  state = reduceModelRunEnvelope(state, envelope(1, { type: "started" }));
  assert.equal(state.status, "streaming");
  state = reduceModelRunEnvelope(state, envelope(2, { type: "text_delta", delta: "Hello" }));
  state = reduceModelRunEnvelope(state, envelope(3, { type: "text_delta", delta: " world" }));
  state = reduceModelRunEnvelope(
    state,
    envelope(4, { type: "completed", finishReason: "stop", usage: emptyUsage }),
  );

  assert.equal(state.content, "Hello world");
  assert.equal(state.status, "completed");
  assert.equal(state.lastSequence, 4);
  assert.equal(state.protocolWarning, null);
});

test("ignores duplicate events and reports sequence gaps", () => {
  let state = reduceModelRunEnvelope(initialState(), envelope(1, { type: "started" }));
  const duplicate = reduceModelRunEnvelope(state, envelope(1, { type: "text_delta", delta: "duplicate" }));
  assert.equal(duplicate, state);

  state = reduceModelRunEnvelope(state, envelope(3, { type: "text_delta", delta: "kept" }));
  assert.equal(state.content, "kept");
  assert.match(state.protocolWarning ?? "", /序号不连续/);
});

test("preserves partial content on a cancelled run", () => {
  let state = reduceModelRunEnvelope(initialState(), envelope(1, { type: "started" }));
  state = reduceModelRunEnvelope(state, envelope(2, { type: "text_delta", delta: "partial" }));
  state = requestChatRunCancellation(state);
  assert.equal(state.cancelRequested, true);
  state = reduceModelRunEnvelope(
    state,
    envelope(3, { type: "cancelled", reason: "userRequested", partialContentRetained: true }),
  );
  assert.equal(state.status, "cancelled");
  assert.equal(state.content, "partial");
  assert.equal(state.partialContentRetained, true);
  assert.equal(state.cancelRequested, false);
  assert.equal(state.cancelErrorMessage, null);
});

test("prevents duplicate cancellation and restores the action after a command rejection", () => {
  const streaming = reduceModelRunEnvelope(initialState(), envelope(1, { type: "started" }));
  const requested = requestChatRunCancellation(streaming);
  assert.equal(requested.cancelRequested, true);
  assert.equal(requestChatRunCancellation(requested), requested);

  const rejected = rejectChatRunCancellation(requested, "运行已经结束");
  assert.equal(rejected.cancelRequested, false);
  assert.equal(rejected.cancelErrorMessage, "运行已经结束");
});

test("maps structured provider errors to executable UI guidance", () => {
  const error: ProviderError = {
    category: "authentication",
    providerCode: "invalid_api_key",
    safeMessage: "invalid credentials",
    retryable: false,
    retryAfterMs: null,
    providerStatus: 401,
  };
  const presentation = presentProviderError(error);
  assert.equal(presentation.action, "openSettings");
  assert.equal(presentation.actionLabel, "检查密钥");
});

test("restores a failed run projection with partial content", () => {
  const error: ProviderError = {
    category: "network",
    providerCode: "application_interrupted",
    safeMessage: "The previous run was interrupted.",
    retryable: true,
    retryAfterMs: null,
    providerStatus: null,
  };
  const restored = createChatRunStateFromProjection(
    {
      runId: "run-restored",
      conversationId: "conversation-1",
      nodeId: "node-restored",
      providerId: "deepseek",
      modelId: "deepseek-chat",
      state: "failed",
      lastSequence: 8,
      partialContent: "partial answer",
      terminalEvent: { type: "failed", error, partialContentRetained: true },
      updatedAt: "2026-08-18T08:00:00Z",
    },
    { prompt: "question", parentNodeId: null, branchType: "continues" },
  );

  assert.equal(restored.status, "failed");
  assert.equal(restored.content, "partial answer");
  assert.equal(restored.error?.providerCode, "application_interrupted");
  assert.equal(restored.partialContentRetained, true);
  const presentation = presentProviderError(error);
  assert.equal(presentation.title, "上次生成被应用退出中断");
  assert.equal(presentation.actionLabel, "重新生成");
});

test("routes a missing credential to settings without suggesting Mock fallback", () => {
  const presentation = presentProviderError({
    category: "authentication",
    providerCode: "credential_not_found",
    safeMessage: "No credential is configured for this provider.",
    retryable: false,
    retryAfterMs: null,
    providerStatus: null,
  });
  assert.equal(presentation.action, "openSettings");
  assert.equal(presentation.actionLabel, "配置 Key");
  assert.match(presentation.guidance, /不会自动改用 Mock/);
});

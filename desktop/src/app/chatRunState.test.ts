import assert from "node:assert/strict";
import test from "node:test";
import type { ModelRunEvent, ModelRunEventEnvelope, ProviderError } from "../domain/runtime.ts";
import {
  SUPPORTED_RUNTIME_CONTRACT_VERSION,
  createChatRunState,
  presentProviderError,
  reduceModelRunEnvelope,
} from "./chatRunState.ts";

function envelope(sequence: number, event: ModelRunEvent): ModelRunEventEnvelope {
  return {
    contractVersion: SUPPORTED_RUNTIME_CONTRACT_VERSION,
    eventId: `event-${sequence}`,
    runId: "run-1",
    nodeId: "node-1",
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
  state = reduceModelRunEnvelope(
    state,
    envelope(3, { type: "cancelled", reason: "userRequested", partialContentRetained: true }),
  );
  assert.equal(state.status, "cancelled");
  assert.equal(state.content, "partial");
  assert.equal(state.partialContentRetained, true);
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

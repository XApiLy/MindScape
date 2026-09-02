import assert from "node:assert/strict";
import test from "node:test";
import type { ProviderDescriptor } from "../domain/provider.ts";
import {
  buildChatModelOptions,
  chooseModelSelection,
  describeModelCapabilities,
} from "./providerCatalog.ts";

const capabilities = {
  textInput: true,
  imageInput: false,
  toolCalling: false,
  usageReporting: true,
  streaming: true,
  contextWindowTokens: null,
  supportsReasoning: false,
  reasoningControl: "none",
  reasoningModes: [],
  structuredOutput: false,
  generationParameters: {
    maxOutputTokens: "supported",
    temperature: "unsupported",
    topP: "unsupported",
    seed: "unsupported",
  },
  inputModalities: ["text"],
};

const providers: ProviderDescriptor[] = [
  {
    id: "mock",
    displayName: "Mock Provider",
    defaultBaseUrl: null,
    customBaseUrlAllowed: false,
    credentialRequired: false,
    models: { "mock-stream-v1": capabilities },
  },
  {
    id: "deepseek",
    displayName: "DeepSeek",
    defaultBaseUrl: "https://api.deepseek.com/chat/completions",
    customBaseUrlAllowed: false,
    credentialRequired: true,
    models: { "deepseek-v4-flash": capabilities },
  },
];

test("marks real models unavailable without a credential while keeping Mock explicit", () => {
  const options = buildChatModelOptions(providers, { mock: true, deepseek: false });
  assert.deepEqual(
    options.map(({ providerId, available, availabilityLabel }) => ({
      providerId,
      available,
      availabilityLabel,
    })),
    [
      { providerId: "mock", available: true, availabilityLabel: "本地测试" },
      { providerId: "deepseek", available: false, availabilityLabel: "缺少 Key" },
    ],
  );
});

test("does not silently fall back to Mock when the selected real credential disappears", () => {
  const current = { providerId: "deepseek", modelId: "deepseek-v4-flash" };
  assert.deepEqual(
    chooseModelSelection(providers, { mock: true, deepseek: false }, current),
    current,
  );
});

test("prefers an available real model when there is no existing selection", () => {
  assert.deepEqual(
    chooseModelSelection(providers, { mock: true, deepseek: true }, null),
    { providerId: "deepseek", modelId: "deepseek-v4-flash" },
  );
});

test("falls back only when the selected model is no longer registered", () => {
  assert.deepEqual(
    chooseModelSelection(providers, { mock: true, deepseek: false }, {
      providerId: "removed-provider",
      modelId: "removed-model",
    }),
    { providerId: "mock", modelId: "mock-stream-v1" },
  );
});

test("describes provider capability snapshots without inventing unsupported controls", () => {
  const deepseekCapabilities = {
    ...capabilities,
    supportsReasoning: true,
    reasoningModes: ["high", "max"] as const,
    structuredOutput: true,
    generationParameters: {
      maxOutputTokens: "supported" as const,
      temperature: "nonReasoningOnly" as const,
      topP: "nonReasoningOnly" as const,
      seed: "unsupported" as const,
    },
  };
  assert.deepEqual(
    describeModelCapabilities(deepseekCapabilities).map(({ label, tone }) => ({ label, tone })),
    [
      { label: "思考：high / max", tone: "supported" },
      { label: "结构化输出可用", tone: "supported" },
      { label: "工具闭环未接入", tone: "unavailable" },
      { label: "流式输出可用", tone: "supported" },
      { label: "temperature（仅非思考）", tone: "limited" },
      { label: "top_p（仅非思考）", tone: "limited" },
      { label: "seed 未声明", tone: "unavailable" },
    ],
  );
});

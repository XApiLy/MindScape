import assert from "node:assert/strict";
import test from "node:test";
import type { ProviderDescriptor } from "../domain/provider.ts";
import {
  buildChatModelOptions,
  chooseModelSelection,
} from "./providerCatalog.ts";

const capabilities = {
  textInput: true,
  imageInput: false,
  toolCalling: false,
  usageReporting: true,
  streaming: true,
  contextWindowTokens: null,
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

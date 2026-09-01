import assert from "node:assert/strict";
import test from "node:test";
import type { ModelCapabilities } from "../domain/index.ts";
import {
  composeEffectiveRunProfile,
  contextPolicyForComposer,
  createDefaultRunProfileDraft,
} from "./runProfileControls.ts";

const deepseekCapabilities: ModelCapabilities = {
  textInput: true,
  imageInput: false,
  toolCalling: false,
  usageReporting: true,
  streaming: true,
  contextWindowTokens: 64_000,
  supportsReasoning: true,
  reasoningControl: "effort",
  reasoningModes: ["disabled", "high", "max"],
  structuredOutput: true,
  generationParameters: {
    maxOutputTokens: "supported",
    temperature: "nonReasoningOnly",
    topP: "nonReasoningOnly",
    seed: "unsupported",
  },
  inputModalities: ["text"],
};

test("builds a frozen profile from explicit controls and catalog facts", () => {
  const draft = { ...createDefaultRunProfileDraft(), reasoningMode: "deep" as const, maxOutputTokens: 2_048 };
  const result = composeEffectiveRunProfile({
    selection: { providerId: "deepseek", modelId: "deepseek-v4-flash" },
    capabilities: deepseekCapabilities,
    draft,
    contextPolicy: "branchFromNode",
    isMock: false,
  });
  assert.deepEqual(result.issues, []);
  assert.equal(result.profile.reasoningMode, "deep");
  assert.equal(result.profile.generationParameters.maxOutputTokens, 2_048);
  assert.equal(result.profile.contextPolicy, "branchFromNode");
  assert.equal(result.profile.toolPermission, "disabled");
  assert.deepEqual(result.profile.capabilitySnapshot.unsupportedParameters, ["seed"]);
});

test("reports incompatible sampling without silently deleting the chosen values", () => {
  const draft = {
    ...createDefaultRunProfileDraft(),
    reasoningMode: "standard" as const,
    temperature: 0.2,
    topP: 0.8,
  };
  const result = composeEffectiveRunProfile({
    selection: { providerId: "deepseek", modelId: "deepseek-v4-flash" },
    capabilities: deepseekCapabilities,
    draft,
    contextPolicy: "continueCurrent",
    isMock: false,
  });
  assert.deepEqual(result.issues.map(({ code }) => code), ["temperature_reasoning_conflict", "top_p_reasoning_conflict"]);
  assert.equal(result.profile.generationParameters.temperature, 0.2);
  assert.equal(result.profile.generationParameters.topP, 0.8);
});

test("maps custom effort and imported continuation explicitly", () => {
  const result = composeEffectiveRunProfile({
    selection: { providerId: "deepseek", modelId: "deepseek-v4-flash" },
    capabilities: deepseekCapabilities,
    draft: { ...createDefaultRunProfileDraft(), reasoningMode: "custom", customReasoningEffort: "max" },
    contextPolicy: contextPolicyForComposer("importedFrom"),
    isMock: false,
  });
  assert.deepEqual(result.issues, []);
  assert.deepEqual(result.profile.generationParameters.vendorParameters, { reasoning_effort: "max" });
  assert.equal(result.profile.contextPolicy, "continueImportedRaw");
});

test("blocks reasoning when a model does not declare it", () => {
  const capabilities: ModelCapabilities = {
    ...deepseekCapabilities,
    supportsReasoning: false,
    reasoningControl: "none",
    reasoningModes: [],
  };
  const result = composeEffectiveRunProfile({
    selection: { providerId: "mock", modelId: "mock-stream-v1" },
    capabilities,
    draft: { ...createDefaultRunProfileDraft(), reasoningMode: "deep" },
    contextPolicy: "continueCurrent",
    isMock: true,
  });
  assert.deepEqual(result.issues.map(({ code }) => code), ["reasoning_unsupported", "deep_reasoning_unsupported"]);
});

import type {
  CapabilityRequirement,
  EffectiveRunProfile,
  FocusContextPolicy,
  ModelCapabilities,
  ModelSelection,
  ParameterSupport,
  ReasoningMode,
} from "../domain";

export const EFFECTIVE_RUN_PROFILE_CONTRACT_VERSION = "mindscape.effective-run-profile.v1";
export const PROVIDER_CATALOG_VERSION = "provider-catalog-v1";

export type RunProfileDraft = {
  reasoningMode: ReasoningMode;
  customReasoningEffort: "high" | "max";
  temperature: number | null;
  topP: number | null;
  maxOutputTokens: number;
  responseFormat: "text" | "json_object";
  timeoutMs: number;
};

export type RunProfileIssue = {
  code: string;
  message: string;
};

export function createDefaultRunProfileDraft(): RunProfileDraft {
  return {
    reasoningMode: "off",
    customReasoningEffort: "high",
    temperature: null,
    topP: null,
    maxOutputTokens: 8_192,
    responseFormat: "text",
    timeoutMs: 120_000,
  };
}

function parameterIssue(
  name: string,
  value: number | null,
  support: ParameterSupport,
  reasoningMode: ReasoningMode,
): RunProfileIssue | null {
  if (value === null) return null;
  if (support === "unsupported") {
    return { code: `unsupported_${name}`, message: `${name} 未在当前模型能力目录中声明。` };
  }
  if (support === "nonReasoningOnly" && reasoningMode !== "off") {
    return { code: `${name}_reasoning_conflict`, message: `${name} 仅能在关闭思考模式时使用；当前值不会被静默删除。` };
  }
  return null;
}

function supportedCapabilities(capabilities: ModelCapabilities): CapabilityRequirement[] {
  const supported: CapabilityRequirement[] = [];
  if (capabilities.textInput) supported.push("textInput");
  if (capabilities.imageInput) supported.push("imageInput");
  if (capabilities.toolCalling) supported.push("toolCalling");
  if (capabilities.usageReporting) supported.push("usageReporting");
  return supported;
}

function unsupportedParameters(capabilities: ModelCapabilities): string[] {
  return Object.entries(capabilities.generationParameters)
    .filter(([, support]) => support === "unsupported")
    .map(([parameter]) => parameter);
}

export function composeEffectiveRunProfile(input: {
  selection: ModelSelection;
  capabilities: ModelCapabilities;
  draft: RunProfileDraft;
  contextPolicy: FocusContextPolicy;
  isMock: boolean;
}): { profile: EffectiveRunProfile; issues: RunProfileIssue[] } {
  const { capabilities, draft, selection } = input;
  const issues: RunProfileIssue[] = [];
  if (draft.reasoningMode !== "off" && !capabilities.supportsReasoning) {
    issues.push({ code: "reasoning_unsupported", message: "当前模型没有声明思考模式支持。" });
  }
  if (draft.reasoningMode === "standard" && !capabilities.reasoningModes.includes("high")) {
    issues.push({ code: "standard_reasoning_unsupported", message: "当前模型没有声明 Standard / high 思考档位。" });
  }
  if (draft.reasoningMode === "deep" && !capabilities.reasoningModes.includes("max")) {
    issues.push({ code: "deep_reasoning_unsupported", message: "当前模型没有声明 Deep / max 思考档位。" });
  }
  if (draft.reasoningMode === "custom" && !capabilities.reasoningModes.includes(draft.customReasoningEffort)) {
    issues.push({ code: "custom_reasoning_unsupported", message: `当前模型没有声明 ${draft.customReasoningEffort} 自定义思考档位。` });
  }
  const temperatureIssue = parameterIssue("temperature", draft.temperature, capabilities.generationParameters.temperature, draft.reasoningMode);
  const topPIssue = parameterIssue("top_p", draft.topP, capabilities.generationParameters.topP, draft.reasoningMode);
  if (temperatureIssue) issues.push(temperatureIssue);
  if (topPIssue) issues.push(topPIssue);
  if (!Number.isInteger(draft.maxOutputTokens) || draft.maxOutputTokens <= 0) {
    issues.push({ code: "invalid_max_output_tokens", message: "最大输出 Token 必须是正整数。" });
  }
  if (!Number.isInteger(draft.timeoutMs) || draft.timeoutMs < 1_000) {
    issues.push({ code: "invalid_timeout", message: "超时必须至少为 1000 ms。" });
  }
  if (draft.responseFormat === "json_object" && !capabilities.structuredOutput) {
    issues.push({ code: "structured_output_unsupported", message: "当前模型没有声明结构化输出能力。" });
  }

  const allowedCapabilities = supportedCapabilities(capabilities)
    .filter((capability) => capability === "textInput" || capability === "usageReporting");
  const vendorParameters = draft.reasoningMode === "custom"
    ? { reasoning_effort: draft.customReasoningEffort }
    : {};
  const profile: EffectiveRunProfile = {
    contractVersion: EFFECTIVE_RUN_PROFILE_CONTRACT_VERSION,
    providerId: selection.providerId,
    modelId: selection.modelId,
    reasoningMode: draft.reasoningMode,
    reasoningBudget: null,
    generationParameters: {
      temperature: draft.temperature,
      topP: draft.topP,
      maxOutputTokens: draft.maxOutputTokens,
      responseFormat: draft.responseFormat === "json_object" ? "json_object" : null,
      seed: null,
      vendorParameters,
    },
    contextPolicy: input.contextPolicy,
    allowedCapabilities,
    toolPermission: "disabled",
    budgetEnvelope: {
      maxInputTokens: null,
      maxReasoningTokens: null,
      maxOutputTokens: draft.maxOutputTokens,
      maxCostMicrounits: input.isMock ? 0 : null,
      timeoutMs: draft.timeoutMs,
    },
    valueOrigins: {
      providerId: "user",
      modelId: "user",
      reasoningMode: "user",
      generationParameters: "user",
      contextPolicy: "conversation",
      toolPermission: "providerConstraint",
      budgetEnvelope: "user",
      capabilitySnapshot: "providerConstraint",
    },
    capabilitySnapshot: {
      catalogVersion: PROVIDER_CATALOG_VERSION,
      contextWindowTokens: capabilities.contextWindowTokens,
      supportedCapabilities: supportedCapabilities(capabilities),
      unsupportedParameters: unsupportedParameters(capabilities),
    },
  };
  return { profile, issues };
}

export function contextPolicyForComposer(parentBranchType: string | null): FocusContextPolicy {
  if (parentBranchType === "importedFrom") return "continueImportedRaw";
  if (parentBranchType) return "branchFromNode";
  return "continueCurrent";
}

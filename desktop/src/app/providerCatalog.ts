import type { ModelCapabilities, ModelSelection, ProviderDescriptor } from "../domain";

export type ProviderCredentialStatus = Record<string, boolean>;

export type ChatModelOption = ModelSelection & {
  providerLabel: string;
  modelLabel: string;
  available: boolean;
  availabilityLabel: string;
  isMock: boolean;
  capabilities: ModelCapabilities;
};

export type ModelCapabilityBadge = {
  label: string;
  tone: "supported" | "limited" | "unavailable";
};

function parameterBadge(label: string, support: ModelCapabilities["generationParameters"][keyof ModelCapabilities["generationParameters"]]): ModelCapabilityBadge {
  if (support === "supported") return { label: `${label} 可用`, tone: "supported" };
  if (support === "nonReasoningOnly") return { label: `${label}（仅非思考）`, tone: "limited" };
  return { label: `${label} 未声明`, tone: "unavailable" };
}

export function describeModelCapabilities(capabilities: ModelCapabilities): ModelCapabilityBadge[] {
  const reasoning = capabilities.supportsReasoning && capabilities.reasoningModes.length > 0
    ? { label: `思考：${capabilities.reasoningModes.join(" / ")}`, tone: "supported" as const }
    : { label: "思考模式未声明", tone: "unavailable" as const };
  return [
    reasoning,
    capabilities.structuredOutput
      ? { label: "结构化输出可用", tone: "supported" as const }
      : { label: "结构化输出未声明", tone: "unavailable" as const },
    capabilities.toolCalling
      ? { label: "工具调用已声明", tone: "supported" as const }
      : { label: "工具闭环未接入", tone: "unavailable" as const },
    capabilities.streaming
      ? { label: "流式输出可用", tone: "supported" as const }
      : { label: "流式输出未声明", tone: "unavailable" as const },
    parameterBadge("temperature", capabilities.generationParameters.temperature),
    parameterBadge("top_p", capabilities.generationParameters.topP),
    parameterBadge("seed", capabilities.generationParameters.seed),
  ];
}

export function hasUsableCredential(
  provider: ProviderDescriptor,
  credentialStatus: ProviderCredentialStatus,
) {
  return !provider.credentialRequired || Boolean(credentialStatus[provider.id]);
}

export function buildChatModelOptions(
  providers: ProviderDescriptor[],
  credentialStatus: ProviderCredentialStatus,
): ChatModelOption[] {
  return providers.flatMap((provider) => {
    const available = hasUsableCredential(provider, credentialStatus);
    return Object.keys(provider.models).sort().map((modelId) => ({
      providerId: provider.id,
      modelId,
      providerLabel: provider.displayName,
      modelLabel: modelId,
      available,
      availabilityLabel: provider.id === "mock"
        ? "本地测试"
        : available
          ? "真实 API 可用"
          : "缺少 Key",
      isMock: provider.id === "mock",
      capabilities: provider.models[modelId],
    }));
  });
}

export function chooseModelSelection(
  providers: ProviderDescriptor[],
  credentialStatus: ProviderCredentialStatus,
  current: ModelSelection | null,
): ModelSelection | null {
  if (current) {
    const currentProvider = providers.find((provider) => provider.id === current.providerId);
    if (currentProvider?.models[current.modelId]) {
      // Preserve an unavailable real selection so the UI can explain the missing Key.
      // Falling back to Mock here would silently change execution semantics.
      return current;
    }
  }

  const candidates = providers.flatMap((provider) =>
    Object.keys(provider.models).map((modelId) => ({ provider, modelId })),
  );
  const available = candidates.filter(({ provider }) =>
    hasUsableCredential(provider, credentialStatus),
  );
  const preferred = available.find(({ provider }) => provider.id !== "mock") ?? available[0];
  return preferred ? { providerId: preferred.provider.id, modelId: preferred.modelId } : null;
}

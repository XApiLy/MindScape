import type { ModelSelection, ProviderDescriptor } from "../domain";

export type ProviderCredentialStatus = Record<string, boolean>;

export type ChatModelOption = ModelSelection & {
  providerLabel: string;
  modelLabel: string;
  available: boolean;
  availabilityLabel: string;
  isMock: boolean;
};

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

export type ModelCapabilities = {
  textInput: boolean;
  imageInput: boolean;
  toolCalling: boolean;
  usageReporting: boolean;
  streaming: boolean;
  contextWindowTokens: number | null;
};

export type ProviderDescriptor = {
  id: string;
  displayName: string;
  defaultBaseUrl: string | null;
  customBaseUrlAllowed: boolean;
  credentialRequired: boolean;
  models: Record<string, ModelCapabilities>;
};

export type ProviderConnectionTestResult = {
  providerId: string;
  authenticated: boolean;
  availableModels: string[];
  checkedAt: string;
};

export type ModelSelection = {
  providerId: string;
  modelId: string;
};

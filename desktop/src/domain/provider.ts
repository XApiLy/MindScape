export type ReasoningControl = "none" | "effort" | "tokenBudget" | "vendorSpecific";
export type ProviderReasoningMode = "disabled" | "high" | "max";
export type ParameterSupport = "unsupported" | "supported" | "nonReasoningOnly";
export type InputModality = "text" | "image" | "file" | "audio";

export type GenerationParameterCapabilities = {
  maxOutputTokens: ParameterSupport;
  temperature: ParameterSupport;
  topP: ParameterSupport;
  seed: ParameterSupport;
};

export type ModelCapabilities = {
  textInput: boolean;
  imageInput: boolean;
  toolCalling: boolean;
  usageReporting: boolean;
  streaming: boolean;
  contextWindowTokens: number | null;
  supportsReasoning: boolean;
  reasoningControl: ReasoningControl;
  reasoningModes: ProviderReasoningMode[];
  structuredOutput: boolean;
  generationParameters: GenerationParameterCapabilities;
  inputModalities: InputModality[];
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

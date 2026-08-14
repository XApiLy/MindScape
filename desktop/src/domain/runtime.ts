import type { ContextSnapshot } from "./context";

export type CapabilityRequirement =
  | "textInput"
  | "imageInput"
  | "toolCalling"
  | "usageReporting";

export type ModelRunBudget = {
  maxOutputTokens: number | null;
  maxCostMicrounits: number | null;
  timeoutMs: number;
};

export type ModelRunRequest = {
  contractVersion: string;
  runId: string;
  conversationId: string;
  nodeId: string;
  contextSnapshot: ContextSnapshot;
  providerId: string;
  modelId: string;
  capabilities: CapabilityRequirement[];
  budget: ModelRunBudget;
  idempotencyKey: string;
  createdAt: string;
};

export type ModelUsage = {
  inputTokens: number | null;
  outputTokens: number | null;
  cachedInputTokens: number | null;
  costMicrounits: number | null;
};

export type ProviderErrorCategory =
  | "authentication"
  | "rateLimit"
  | "insufficientBalance"
  | "modelUnavailable"
  | "invalidRequest"
  | "network"
  | "timeout"
  | "contentPolicy"
  | "cancelled"
  | "unknown";

export type ProviderError = {
  category: ProviderErrorCategory;
  providerCode: string | null;
  safeMessage: string;
  retryable: boolean;
  retryAfterMs: number | null;
  providerStatus: number | null;
};

export type ModelRunEvent =
  | { type: "started" }
  | { type: "text_delta"; delta: string }
  | { type: "usage_updated"; usage: ModelUsage }
  | {
      type: "completed";
      finishReason: "stop" | "length" | "contentPolicy" | "toolCall" | "unknown";
      usage: ModelUsage;
    }
  | {
      type: "cancelled";
      reason: "userRequested" | "timeout" | "applicationShutdown" | "superseded";
      partialContentRetained: boolean;
    }
  | {
      type: "failed";
      error: ProviderError;
      partialContentRetained: boolean;
    };

export type ModelRunEventEnvelope = {
  contractVersion: string;
  eventId: string;
  runId: string;
  nodeId: string;
  sequence: number;
  occurredAt: string;
  event: ModelRunEvent;
};

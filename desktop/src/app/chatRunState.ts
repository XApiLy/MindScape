import type { BranchType } from "../domain/common.ts";
import {
  APPLICATION_INTERRUPTED_PROVIDER_CODE,
  type EffectiveRunProfile,
  type ModelRunEventEnvelope,
  type ModelRunProjection,
  type ModelUsage,
  type ProviderError,
  type ProviderErrorCategory,
} from "../domain/runtime.ts";

export const SUPPORTED_RUNTIME_CONTRACT_VERSION = "mindscape.runtime.v1";

export type ChatRunStatus = "starting" | "streaming" | "completed" | "cancelled" | "failed";

export type ChatRunState = {
  id: string;
  nodeId: string;
  providerId: string;
  modelId: string;
  prompt: string;
  parentNodeId: string | null;
  branchType: BranchType;
  content: string;
  status: ChatRunStatus;
  usage: ModelUsage | null;
  error: ProviderError | null;
  errorMessage: string | null;
  finishReason: "stop" | "length" | "contentPolicy" | "toolCall" | "unknown" | null;
  partialContentRetained: boolean;
  lastSequence: number;
  protocolWarning: string | null;
  cancelRequested: boolean;
  cancelErrorMessage: string | null;
  effectiveRunProfile?: EffectiveRunProfile;
};

export type ProviderErrorPresentation = {
  title: string;
  guidance: string;
  action: "openSettings" | "retry" | "chooseModel" | "editRequest" | "none";
  actionLabel: string | null;
};

export function createChatRunState(input: {
  runId: string;
  nodeId: string;
  providerId: string;
  modelId: string;
  prompt: string;
  parentNodeId: string | null;
  branchType: BranchType;
  effectiveRunProfile?: EffectiveRunProfile;
}): ChatRunState {
  return {
    id: input.runId,
    nodeId: input.nodeId,
    providerId: input.providerId,
    modelId: input.modelId,
    prompt: input.prompt,
    parentNodeId: input.parentNodeId,
    branchType: input.branchType,
    content: "",
    status: "starting",
    usage: null,
    error: null,
    errorMessage: null,
    finishReason: null,
    partialContentRetained: false,
    lastSequence: 0,
    protocolWarning: null,
    cancelRequested: false,
    cancelErrorMessage: null,
    ...(input.effectiveRunProfile ? { effectiveRunProfile: input.effectiveRunProfile } : {}),
  };
}

export function createChatRunStateFromProjection(
  projection: ModelRunProjection,
  input: {
    prompt: string;
    parentNodeId: string | null;
    branchType: BranchType;
  },
): ChatRunState {
  const terminal = projection.terminalEvent;
  const completed = terminal?.type === "completed" ? terminal : null;
  const cancelled = terminal?.type === "cancelled" ? terminal : null;
  const failed = terminal?.type === "failed" ? terminal : null;
  const status: ChatRunStatus =
    projection.state === "pending" ? "starting" : projection.state;

  return {
    id: projection.runId,
    nodeId: projection.nodeId,
    providerId: projection.providerId,
    modelId: projection.modelId,
    prompt: input.prompt,
    parentNodeId: input.parentNodeId,
    branchType: input.branchType,
    content: projection.partialContent,
    status,
    usage: completed?.usage ?? null,
    error: failed?.error ?? null,
    errorMessage: failed?.error.safeMessage ?? null,
    finishReason: completed?.finishReason ?? null,
    partialContentRetained:
      cancelled?.partialContentRetained ?? failed?.partialContentRetained ?? false,
    lastSequence: projection.lastSequence,
    protocolWarning: null,
    cancelRequested: false,
    cancelErrorMessage: null,
  };
}

export function requestChatRunCancellation(state: ChatRunState): ChatRunState {
  if (isTerminal(state.status) || state.cancelRequested) return state;
  return { ...state, cancelRequested: true, cancelErrorMessage: null };
}

export function rejectChatRunCancellation(
  state: ChatRunState,
  message: string,
): ChatRunState {
  if (isTerminal(state.status)) return state;
  return { ...state, cancelRequested: false, cancelErrorMessage: message };
}

function isTerminal(status: ChatRunStatus) {
  return status === "completed" || status === "cancelled" || status === "failed";
}

export function reduceModelRunEnvelope(
  state: ChatRunState,
  envelope: ModelRunEventEnvelope,
): ChatRunState {
  if (envelope.runId !== state.id || envelope.nodeId !== state.nodeId) {
    return {
      ...state,
      protocolWarning: "收到不属于当前运行的事件，已忽略。",
    };
  }

  if (envelope.contractVersion !== SUPPORTED_RUNTIME_CONTRACT_VERSION) {
    return {
      ...state,
      status: "failed",
      error: {
        category: "invalidRequest",
        providerCode: "runtime_contract_mismatch",
        safeMessage: "模型运行协议版本不兼容，请更新 MindScape。",
        retryable: false,
        retryAfterMs: null,
        providerStatus: null,
      },
      errorMessage: "模型运行协议版本不兼容，请更新 MindScape。",
      protocolWarning: `不支持的运行协议：${envelope.contractVersion}`,
    };
  }

  if (envelope.sequence <= state.lastSequence) return state;

  const sequenceWarning =
    envelope.sequence > state.lastSequence + 1
      ? `运行事件序号不连续：期望 ${state.lastSequence + 1}，收到 ${envelope.sequence}。`
      : state.protocolWarning;

  if (isTerminal(state.status)) {
    return {
      ...state,
      protocolWarning: `终态 ${state.status} 后收到额外事件 ${envelope.event.type}，已忽略。`,
    };
  }

  const common = {
    ...state,
    lastSequence: envelope.sequence,
    protocolWarning: sequenceWarning,
  };

  switch (envelope.event.type) {
    case "started":
      return { ...common, status: "streaming" };
    case "text_delta":
      return {
        ...common,
        content: state.content + envelope.event.delta,
        status: "streaming",
      };
    case "usage_updated":
      return { ...common, usage: envelope.event.usage };
    case "completed":
      return {
        ...common,
        status: "completed",
        cancelRequested: false,
        cancelErrorMessage: null,
        usage: envelope.event.usage,
        finishReason: envelope.event.finishReason,
      };
    case "cancelled":
      return {
        ...common,
        status: "cancelled",
        cancelRequested: false,
        cancelErrorMessage: null,
        partialContentRetained: envelope.event.partialContentRetained,
      };
    case "failed":
      return {
        ...common,
        status: "failed",
        cancelRequested: false,
        cancelErrorMessage: null,
        error: envelope.event.error,
        errorMessage: envelope.event.error.safeMessage,
        partialContentRetained: envelope.event.partialContentRetained,
      };
  }
}

const errorCopy: Record<ProviderErrorCategory, Omit<ProviderErrorPresentation, "actionLabel"> & { actionLabel: string | null }> = {
  authentication: {
    title: "API Key 无效或已失效",
    guidance: "请检查密钥是否完整、是否属于当前厂商，并重新测试连接。",
    action: "openSettings",
    actionLabel: "检查密钥",
  },
  rateLimit: {
    title: "请求过于频繁",
    guidance: "厂商正在限流。请稍后重试，或切换其他可用模型。",
    action: "retry",
    actionLabel: "重试",
  },
  insufficientBalance: {
    title: "模型账户余额不足",
    guidance: "请前往厂商控制台补充额度，或选择其他已配置模型。",
    action: "openSettings",
    actionLabel: "查看配置",
  },
  modelUnavailable: {
    title: "当前模型不可用",
    guidance: "模型可能已下线、无权限或暂时维护，请选择其他可用模型。",
    action: "chooseModel",
    actionLabel: "选择模型",
  },
  invalidRequest: {
    title: "本次请求无法发送",
    guidance: "请检查输入和模型能力要求；MindScape 不会静默降级发送。",
    action: "editRequest",
    actionLabel: "修改输入",
  },
  network: {
    title: "无法连接模型服务",
    guidance: "请检查网络、代理和 Base URL，恢复连接后再试。",
    action: "retry",
    actionLabel: "重试",
  },
  timeout: {
    title: "模型响应超时",
    guidance: "服务在限定时间内没有完成响应，已保留收到的内容。",
    action: "retry",
    actionLabel: "重新生成",
  },
  contentPolicy: {
    title: "厂商内容策略拒绝了请求",
    guidance: "请调整问题表达或删除不兼容内容；MindScape 不会把拒绝伪装成回答。",
    action: "editRequest",
    actionLabel: "修改输入",
  },
  cancelled: {
    title: "运行已取消",
    guidance: "本次生成已停止，已收到的内容会按照运行记录保留。",
    action: "retry",
    actionLabel: "重新生成",
  },
  unknown: {
    title: "模型运行失败",
    guidance: "发生了尚未归类的错误。可以重试；若持续出现，请复制脱敏诊断信息。",
    action: "retry",
    actionLabel: "重试",
  },
};

export function presentProviderError(error: ProviderError): ProviderErrorPresentation {
  if (error.providerCode === APPLICATION_INTERRUPTED_PROVIDER_CODE) {
    return {
      title: "上次生成被应用退出中断",
      guidance: "MindScape 已恢复持久化的部分内容。本次运行没有被标记为完成，你可以明确选择重新生成。",
      action: "retry",
      actionLabel: "重新生成",
    };
  }
  if (error.category === "authentication" && error.providerCode === "credential_not_found") {
    return {
      title: "尚未配置 API Key",
      guidance: "请在模型设置中安全保存该 Provider 的 Key，然后重新发送；MindScape 不会自动改用 Mock。",
      action: "openSettings",
      actionLabel: "配置 Key",
    };
  }
  const presentation = errorCopy[error.category];
  if (error.category === "rateLimit" && error.retryAfterMs) {
    return {
      ...presentation,
      guidance: `厂商正在限流，约 ${Math.ceil(error.retryAfterMs / 1000)} 秒后可重试。`,
    };
  }
  return presentation;
}

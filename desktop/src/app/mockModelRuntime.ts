import type { ModelRunEvent, ModelRunEventEnvelope } from "../domain";
import { SUPPORTED_RUNTIME_CONTRACT_VERSION } from "./chatRunState";
import { chunkMockResponse } from "./mockStreamChunking";

export type MockRunInput = {
  runId: string;
  nodeId: string;
  prompt: string;
  parentTitle?: string;
  signal: AbortSignal;
  onEvent: (envelope: ModelRunEventEnvelope) => void;
};

const wait = (milliseconds: number, signal: AbortSignal) =>
  new Promise<void>((resolve, reject) => {
    const timer = window.setTimeout(resolve, milliseconds);
    signal.addEventListener(
      "abort",
      () => {
        window.clearTimeout(timer);
        reject(new DOMException("The operation was aborted", "AbortError"));
      },
      { once: true },
    );
  });

function createMockResponse(prompt: string, parentTitle?: string) {
  const source = parentTitle ? `我会沿着“${parentTitle}”的上下文继续。` : "这是一个新的根会话。";
  return [
    "这是本地模拟 Provider 返回的流式内容，用于验证 Chat 前端状态，不会调用真实模型或产生费用。\n\n",
    `${source}\n\n`,
    `你本轮提出的问题是：${prompt}\n\n`,
    "当前前端已经能够消费统一的 started、text_delta 和 completed 事件。",
    "正式 Provider 接入后，这里将保持相同的渲染路径，不解析任何厂商原始 SSE。",
  ].join("");
}

export async function runMockModel(input: MockRunInput) {
  const response = createMockResponse(input.prompt, input.parentTitle);
  const chunks = chunkMockResponse(response);
  let sequence = 0;

  const emit = (event: ModelRunEvent) => {
    sequence += 1;
    input.onEvent({
      contractVersion: SUPPORTED_RUNTIME_CONTRACT_VERSION,
      eventId: `mock-event-${input.runId}-${sequence}`,
      runId: input.runId,
      nodeId: input.nodeId,
      sequence,
      occurredAt: new Date().toISOString(),
      event,
    });
  };

  emit({ type: "started" });

  try {
    for (const delta of chunks) {
      await wait(34, input.signal);
      emit({ type: "text_delta", delta });
    }
    emit({
      type: "completed",
      finishReason: "stop",
      usage: {
        inputTokens: null,
        outputTokens: null,
        cachedInputTokens: null,
        costMicrounits: 0,
      },
    });
  } catch (error) {
    if (input.signal.aborted) {
      emit({ type: "cancelled", reason: "userRequested", partialContentRetained: true });
      return;
    }

    emit({
      type: "failed",
      partialContentRetained: true,
      error: {
        category: "unknown",
        providerCode: null,
        safeMessage: error instanceof Error ? error.message : "模拟运行发生未知错误",
        retryable: true,
        retryAfterMs: null,
        providerStatus: null,
      },
    });
  }
}

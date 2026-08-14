import type { ChatMessage, ProviderConfig } from "../types/workspace";

type StreamOptions = {
  signal?: AbortSignal;
  onChunk: (chunk: string) => void;
};

const ensureOk = async (response: Response) => {
  if (response.ok) return;
  const body = await response.text();
  throw new Error(body || `${response.status} ${response.statusText}`);
};

const consumeSse = async (
  response: Response,
  extract: (payload: Record<string, unknown>) => string,
  onChunk: (chunk: string) => void,
) => {
  if (!response.body) throw new Error("供应商没有返回流式响应");
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const events = buffer.split("\n\n");
    buffer = events.pop() ?? "";
    for (const event of events) {
      const dataLines = event
        .split("\n")
        .filter((line) => line.startsWith("data:"))
        .map((line) => line.slice(5).trim());
      for (const data of dataLines) {
        if (!data || data === "[DONE]") continue;
        try {
          const chunk = extract(JSON.parse(data) as Record<string, unknown>);
          if (chunk) onChunk(chunk);
        } catch {
          // Providers may interleave heartbeat events that are not JSON.
        }
      }
    }
  }
};

const streamOpenAICompatible = async (
  config: ProviderConfig,
  messages: ChatMessage[],
  options: StreamOptions,
) => {
  const response = await fetch(`${config.baseUrl.replace(/\/$/, "")}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${config.apiKey}`,
    },
    body: JSON.stringify({
      model: config.model,
      stream: true,
      messages: messages.map(({ role, content }) => ({ role, content })),
    }),
    signal: options.signal,
  });
  await ensureOk(response);
  await consumeSse(
    response,
    (payload) => {
      const choices = payload.choices as Array<{ delta?: { content?: string } }> | undefined;
      return choices?.[0]?.delta?.content ?? "";
    },
    options.onChunk,
  );
};

const streamAnthropic = async (
  config: ProviderConfig,
  messages: ChatMessage[],
  options: StreamOptions,
) => {
  const system = messages.filter((message) => message.role === "system").map((message) => message.content).join("\n");
  const response = await fetch(`${config.baseUrl.replace(/\/$/, "")}/messages`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": config.apiKey,
      "anthropic-version": "2023-06-01",
      "anthropic-dangerous-direct-browser-access": "true",
    },
    body: JSON.stringify({
      model: config.model,
      max_tokens: 4096,
      stream: true,
      ...(system ? { system } : {}),
      messages: messages
        .filter((message) => message.role !== "system")
        .map(({ role, content }) => ({ role, content })),
    }),
    signal: options.signal,
  });
  await ensureOk(response);
  await consumeSse(
    response,
    (payload) => {
      const delta = payload.delta as { text?: string } | undefined;
      return payload.type === "content_block_delta" ? delta?.text ?? "" : "";
    },
    options.onChunk,
  );
};

const streamGemini = async (
  config: ProviderConfig,
  messages: ChatMessage[],
  options: StreamOptions,
) => {
  const endpoint = `${config.baseUrl.replace(/\/$/, "")}/models/${encodeURIComponent(config.model)}:streamGenerateContent?alt=sse&key=${encodeURIComponent(config.apiKey)}`;
  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      contents: messages
        .filter((message) => message.role !== "system")
        .map((message) => ({
          role: message.role === "assistant" ? "model" : "user",
          parts: [{ text: message.content }],
        })),
      systemInstruction: {
        parts: messages.filter((message) => message.role === "system").map((message) => ({ text: message.content })),
      },
    }),
    signal: options.signal,
  });
  await ensureOk(response);
  await consumeSse(
    response,
    (payload) => {
      const candidates = payload.candidates as Array<{ content?: { parts?: Array<{ text?: string }> } }> | undefined;
      return candidates?.[0]?.content?.parts?.map((part) => part.text ?? "").join("") ?? "";
    },
    options.onChunk,
  );
};

export const streamProviderChat = async (
  config: ProviderConfig,
  messages: ChatMessage[],
  options: StreamOptions,
) => {
  if (!config.apiKey) throw new Error("请先在模型设置中填写 API Key");
  if (config.kind === "anthropic") return streamAnthropic(config, messages, options);
  if (config.kind === "gemini") return streamGemini(config, messages, options);
  return streamOpenAICompatible(config, messages, options);
};

export const demoResponse = (prompt: string) =>
  `这是 MindScape 的本地演示回答。你提出的是：**${prompt}**\n\n当前画布已经保留了来源节点和会话关系。配置任意模型供应商的 API Key 后，这里会切换为真实流式回答。\n\n你可以继续：\n\n- 从当前卡片**深入探索**\n- 创建一个**平行视角**\n- 选择**换个角度**重新提问\n- 点击卡片右上角进入聚焦阅读`;

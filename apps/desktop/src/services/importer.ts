import type { ChatMessage, ImportedConversation } from "../types/workspace";

const messageId = () => `message-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

const normalizeRole = (value: unknown): ChatMessage["role"] => {
  const role = String(value ?? "").toLowerCase();
  if (["assistant", "ai", "claude", "model"].includes(role)) return "assistant";
  if (["system", "developer"].includes(role)) return "system";
  return "user";
};

const contentToText = (content: unknown): string => {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content
      .map((item) => {
        if (typeof item === "string") return item;
        if (item && typeof item === "object" && "text" in item) return String(item.text ?? "");
        return "";
      })
      .filter(Boolean)
      .join("\n");
  }
  if (content && typeof content === "object" && "text" in content) {
    return String((content as { text?: unknown }).text ?? "");
  }
  return content == null ? "" : JSON.stringify(content);
};

const normalizeMessage = (input: unknown): ChatMessage | null => {
  if (!input || typeof input !== "object") return null;
  const record = input as Record<string, unknown>;
  const content = contentToText(record.content ?? record.text ?? record.message);
  if (!content.trim()) return null;
  return {
    id: String(record.id ?? messageId()),
    role: normalizeRole(record.role ?? record.sender ?? record.author),
    content,
    createdAt: String(record.createdAt ?? record.created_at ?? record.timestamp ?? new Date().toISOString()),
  };
};

const findMessageArray = (value: unknown): unknown[] => {
  if (Array.isArray(value)) return value;
  if (!value || typeof value !== "object") return [];
  const record = value as Record<string, unknown>;
  for (const key of ["messages", "conversation", "items", "chat", "turns"]) {
    if (Array.isArray(record[key])) return record[key] as unknown[];
  }
  return [];
};

const parseTextTurns = (text: string): ChatMessage[] => {
  const marker = /^(用户|User|Human|助手|Assistant|AI|Claude|System)\s*[:：]\s*/i;
  const lines = text.split(/\r?\n/);
  const messages: ChatMessage[] = [];
  let current: ChatMessage | null = null;

  for (const line of lines) {
    const match = line.match(marker);
    if (match) {
      if (current?.content.trim()) messages.push(current);
      current = {
        id: messageId(),
        role: normalizeRole(match[1]),
        content: line.slice(match[0].length),
        createdAt: new Date().toISOString(),
      };
    } else if (current) {
      current.content += `${current.content ? "\n" : ""}${line}`;
    }
  }
  if (current?.content.trim()) messages.push(current);
  if (messages.length) return messages;

  return [
    {
      id: messageId(),
      role: "user",
      content: text,
      createdAt: new Date().toISOString(),
    },
  ];
};

export const importConversationFile = async (file: File): Promise<ImportedConversation> => {
  const text = await file.text();
  const extension = file.name.split(".").pop()?.toLowerCase();
  let rawMessages: unknown[] = [];
  const warnings: string[] = [];

  try {
    if (extension === "json") {
      rawMessages = findMessageArray(JSON.parse(text));
    } else if (extension === "jsonl") {
      rawMessages = text
        .split(/\r?\n/)
        .filter(Boolean)
        .map((line) => JSON.parse(line));
    }
  } catch (error) {
    warnings.push(`结构化解析失败，已按纯文本导入：${error instanceof Error ? error.message : "未知错误"}`);
  }

  const messages = rawMessages.length
    ? rawMessages.map(normalizeMessage).filter((message): message is ChatMessage => Boolean(message))
    : parseTextTurns(text);

  if (!messages.length) warnings.push("没有识别到有效消息");

  return {
    title: file.name.replace(/\.[^.]+$/, ""),
    source: extension ? extension.toUpperCase() : "TEXT",
    messages,
    warnings,
  };
};

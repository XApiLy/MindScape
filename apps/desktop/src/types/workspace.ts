import type { Edge, Node } from "@xyflow/react";

export type BranchKind = "main" | "deep" | "parallel" | "alternate";

export type ProviderKind =
  | "openai"
  | "anthropic"
  | "gemini"
  | "deepseek"
  | "openrouter"
  | "custom";

export type ProviderConfig = {
  id: string;
  kind: ProviderKind;
  name: string;
  model: string;
  baseUrl: string;
  apiKey: string;
  enabled: boolean;
};

export type ChatMessage = {
  id: string;
  role: "system" | "user" | "assistant";
  content: string;
  createdAt: string;
};

export type ConversationNodeData = Record<string, unknown> & {
  title: string;
  prompt: string;
  content: string;
  model: string;
  createdAt: string;
  tags: string[];
  branchKind: BranchKind;
  status: "ready" | "thinking" | "error";
  reasoningLabel?: string;
  imported?: boolean;
};

export type ConversationNode = Node<ConversationNodeData, "conversation">;
export type ConversationEdge = Edge;

export type ProjectItem = {
  id: string;
  title: string;
  count: number;
  conversations: { id: string; title: string; updatedAt: string }[];
};

export type AnalysisLevel = "raw" | "quick" | "detailed";

export type ImportedConversation = {
  title: string;
  source: string;
  messages: ChatMessage[];
  warnings: string[];
};

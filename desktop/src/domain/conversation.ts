import type { BranchType, MessageRole, RunState } from "./common";
import type { ContentBlock } from "./content";

export type Workspace = {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
};

export type Conversation = {
  id: string;
  workspaceId: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type ConversationSummary = Conversation & {
  nodeCount: number;
};

export type Message = {
  id: string;
  conversationId: string;
  nodeId: string;
  role: MessageRole;
  contentBlocks: ContentBlock[];
  createdAt: string;
};

export type ConversationNode = {
  id: string;
  conversationId: string;
  parentNodeId: string | null;
  branchType: BranchType;
  title: string;
  userMessage: Message;
  assistantMessage: Message | null;
  providerId: string | null;
  modelId: string | null;
  contextSnapshotId: string;
  runState: RunState;
  createdAt: string;
  updatedAt: string;
  revision: number;
};

export type ConversationEdge = {
  id: string;
  conversationId: string;
  sourceNodeId: string;
  targetNodeId: string;
  relation: BranchType;
  createdAt: string;
};

export type CanvasNodePosition = {
  nodeId: string;
  x: number;
  y: number;
};

export type ConversationGraph = {
  conversation: Conversation;
  nodes: ConversationNode[];
  edges: ConversationEdge[];
  positions: CanvasNodePosition[];
};

export type KernelBootstrap = {
  schemaVersion: number;
  databasePath: string;
  workspace: Workspace;
  conversations: ConversationSummary[];
};

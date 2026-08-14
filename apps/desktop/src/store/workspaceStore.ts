import { applyEdgeChanges, applyNodeChanges, type EdgeChange, type NodeChange } from "@xyflow/react";
import { create } from "zustand";
import { persist } from "zustand/middleware";
import { demoEdges, demoNodes, demoProjects } from "../data/demo";
import type {
  AnalysisLevel,
  BranchKind,
  ChatMessage,
  ConversationEdge,
  ConversationNode,
  ImportedConversation,
  ProjectItem,
  ProviderConfig,
} from "../types/workspace";

type WorkspaceState = {
  nodes: ConversationNode[];
  edges: ConversationEdge[];
  projects: ProjectItem[];
  selectedNodeId: string | null;
  focusedNodeId: string | null;
  activeConversationId: string;
  canvasMode: "immersive" | "grid";
  providerConfigs: ProviderConfig[];
  activeProviderId: string;
  onNodesChange: (changes: NodeChange<ConversationNode>[]) => void;
  onEdgesChange: (changes: EdgeChange<ConversationEdge>[]) => void;
  selectNode: (id: string | null) => void;
  focusNode: (id: string | null) => void;
  setCanvasMode: (mode: "immersive" | "grid") => void;
  setActiveConversation: (id: string) => void;
  setActiveProvider: (id: string) => void;
  upsertProvider: (config: ProviderConfig) => void;
  branchFrom: (id: string, kind: BranchKind) => string | null;
  addPromptNode: (prompt: string, model: string, parentId?: string | null) => string;
  updateNodeContent: (id: string, content: string, status?: "ready" | "thinking" | "error") => void;
  importConversation: (conversation: ImportedConversation, level: AnalysisLevel) => void;
  resetDemo: () => void;
};

const defaultProviders: ProviderConfig[] = [
  {
    id: "demo-gemini",
    kind: "gemini",
    name: "Gemini",
    model: "gemini-2.5-flash",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta",
    apiKey: "",
    enabled: true,
  },
  {
    id: "demo-anthropic",
    kind: "anthropic",
    name: "Anthropic",
    model: "claude-sonnet-4-20250514",
    baseUrl: "https://api.anthropic.com/v1",
    apiKey: "",
    enabled: true,
  },
  {
    id: "demo-openai",
    kind: "openai",
    name: "OpenAI-compatible",
    model: "gpt-4.1-mini",
    baseUrl: "https://api.openai.com/v1",
    apiKey: "",
    enabled: true,
  },
  {
    id: "demo-deepseek",
    kind: "deepseek",
    name: "DeepSeek",
    model: "deepseek-chat",
    baseUrl: "https://api.deepseek.com/v1",
    apiKey: "",
    enabled: true,
  },
];

const newId = (prefix: string) => `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;

const branchOffset: Record<BranchKind, { x: number; y: number }> = {
  main: { x: 460, y: 0 },
  deep: { x: 430, y: -100 },
  parallel: { x: 440, y: 250 },
  alternate: { x: 40, y: 430 },
};

export const useWorkspaceStore = create<WorkspaceState>()(
  persist(
    (set, get) => ({
      nodes: demoNodes,
      edges: demoEdges,
      projects: demoProjects,
      selectedNodeId: "node-root",
      focusedNodeId: null,
      activeConversationId: "conv-core",
      canvasMode: "immersive",
      providerConfigs: defaultProviders,
      activeProviderId: "demo-gemini",
      onNodesChange: (changes) => set({ nodes: applyNodeChanges(changes, get().nodes) }),
      onEdgesChange: (changes) => set({ edges: applyEdgeChanges(changes, get().edges) }),
      selectNode: (id) => set({ selectedNodeId: id }),
      focusNode: (id) => set({ focusedNodeId: id }),
      setCanvasMode: (mode) => set({ canvasMode: mode }),
      setActiveConversation: (id) => set({ activeConversationId: id }),
      setActiveProvider: (id) => set({ activeProviderId: id }),
      upsertProvider: (config) =>
        set((state) => ({
          providerConfigs: state.providerConfigs.some((item) => item.id === config.id)
            ? state.providerConfigs.map((item) => (item.id === config.id ? config : item))
            : [...state.providerConfigs, config],
        })),
      branchFrom: (id, kind) => {
        const source = get().nodes.find((node) => node.id === id);
        if (!source) return null;
        const nodeId = newId("node");
        const offset = branchOffset[kind];
        const labels: Record<BranchKind, string> = {
          main: "继续主线",
          deep: "深入探索",
          parallel: "平行发散",
          alternate: "换个角度",
        };
        const nextNode: ConversationNode = {
          id: nodeId,
          type: "conversation",
          position: { x: source.position.x + offset.x, y: source.position.y + offset.y },
          data: {
            title: `${labels[kind]}：${source.data.title}`,
            prompt: `基于“${source.data.title}”${labels[kind]}。`,
            content: "在这里继续提问，新的回答将保留与来源卡片的上下文关系。",
            model: source.data.model,
            createdAt: new Date().toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }),
            tags: [labels[kind]],
            branchKind: kind,
            status: "ready",
            reasoningLabel: "等待新的问题",
          },
        };
        set((state) => ({
          nodes: [...state.nodes, nextNode],
          edges: [
            ...state.edges,
            {
              id: newId("edge"),
              source: id,
              target: nodeId,
              type: "smoothstep",
              style: {
                stroke: kind === "deep" ? "#79b79b" : kind === "parallel" ? "#8f86b3" : "#b8a57f",
                strokeWidth: 1.5,
              },
            },
          ],
          selectedNodeId: nodeId,
        }));
        return nodeId;
      },
      addPromptNode: (prompt, model, parentId) => {
        const currentNodes = get().nodes;
        const parent = currentNodes.find((node) => node.id === parentId) ?? currentNodes[currentNodes.length - 1];
        const nodeId = newId("node");
        const nextNode: ConversationNode = {
          id: nodeId,
          type: "conversation",
          position: parent
            ? { x: parent.position.x + 450, y: parent.position.y + 60 }
            : { x: 120, y: 120 },
          data: {
            title: prompt.length > 28 ? `${prompt.slice(0, 28)}…` : prompt,
            prompt,
            content: "正在组织上下文并请求模型…",
            model,
            createdAt: new Date().toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }),
            tags: ["新会话"],
            branchKind: "main",
            status: "thinking",
            reasoningLabel: "正在准备上下文",
          },
        };
        set((state) => ({
          nodes: [...state.nodes, nextNode],
          edges: parent
            ? [
                ...state.edges,
                {
                  id: newId("edge"),
                  source: parent.id,
                  target: nodeId,
                  type: "smoothstep",
                  style: { stroke: "#79b79b", strokeWidth: 1.5 },
                },
              ]
            : state.edges,
          selectedNodeId: nodeId,
        }));
        return nodeId;
      },
      updateNodeContent: (id, content, status = "ready") =>
        set((state) => ({
          nodes: state.nodes.map((node) =>
            node.id === id
              ? {
                  ...node,
                  data: {
                    ...node.data,
                    content,
                    status,
                    reasoningLabel: status === "ready" ? "回答完成 · 可展开工作轨迹" : node.data.reasoningLabel,
                  },
                }
              : node,
          ),
        })),
      importConversation: (conversation, level) => {
        const assistantMessages = conversation.messages.filter((message) => message.role === "assistant");
        const latest = conversation.messages.slice(-8);
        const content = latest
          .map((message) => `**${message.role === "user" ? "用户" : "AI"}**\n\n${message.content}`)
          .join("\n\n---\n\n");
        const rootId = newId("import");
        const importNode: ConversationNode = {
          id: rootId,
          type: "conversation",
          position: { x: 160, y: 140 },
          data: {
            title: conversation.title,
            prompt: `从 ${conversation.source} 导入 ${conversation.messages.length} 条消息`,
            content: content || "导入成功，但没有识别到可显示的消息。",
            model: assistantMessages[assistantMessages.length - 1]?.content ? conversation.source : "Imported",
            createdAt: new Date().toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }),
            tags: ["已导入", level === "raw" ? "原样继续" : level === "quick" ? "快速识别" : "详细分析"],
            branchKind: "main",
            status: "ready",
            reasoningLabel:
              level === "raw"
                ? "仅保留原文，未进行语义分析"
                : `已创建${level === "quick" ? "快速" : "详细"}接续轨`,
            imported: true,
          },
        };
        set({
          nodes: [importNode],
          edges: [],
          selectedNodeId: rootId,
          focusedNodeId: null,
        });
      },
      resetDemo: () => set({ nodes: demoNodes, edges: demoEdges, selectedNodeId: "node-root" }),
    }),
    {
      name: "mindscape-workspace-v1",
      partialize: (state) => ({
        nodes: state.nodes,
        edges: state.edges,
        projects: state.projects,
        selectedNodeId: state.selectedNodeId,
        activeConversationId: state.activeConversationId,
        canvasMode: state.canvasMode,
        activeProviderId: state.activeProviderId,
        providerConfigs: state.providerConfigs.map((provider) => ({ ...provider, apiKey: "" })),
      }),
    },
  ),
);

export const toMessagesForNode = (node: ConversationNode): ChatMessage[] => [
  { id: `${node.id}-user`, role: "user", content: node.data.prompt, createdAt: node.data.createdAt },
  { id: `${node.id}-assistant`, role: "assistant", content: node.data.content, createdAt: node.data.createdAt },
];

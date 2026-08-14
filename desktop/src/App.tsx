import { useEffect, useRef, useState } from "react";
import {
  createChatRunState,
  reduceModelRunEnvelope,
  type ChatRunState,
} from "./app/chatRunState";
import { runMockModel } from "./app/mockModelRuntime";
import { kernelClient } from "./app/kernelClient";
import { ChatWorkspace } from "./components/ChatWorkspace";
import { ContextDialog } from "./components/ContextDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import type { CanvasPoint } from "./canvas/graphProjection";
import type {
  BranchType,
  Conversation,
  ConversationEdge,
  ConversationGraph,
  ConversationNode,
  ConversationSummary,
  ContextSnapshot,
  KernelBootstrap,
  Message,
  ModelRunRequest,
  Workspace,
} from "./domain";

type RuntimeMode = "tauri" | "preview";

const previewWorkspace: Workspace = {
  id: "workspace-preview",
  name: "UI 预览工作区",
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

function newPreviewConversation(title: string): Conversation {
  const timestamp = new Date().toISOString();
  return {
    id: `conversation-preview-${Date.now()}`,
    workspaceId: previewWorkspace.id,
    title,
    createdAt: timestamp,
    updatedAt: timestamp,
    revision: 1,
  };
}

function asSummary(conversation: Conversation, nodeCount = 0): ConversationSummary {
  return { ...conversation, nodeCount };
}

function createPreviewNode(
  graph: ConversationGraph,
  prompt: string,
  content: string,
  parentNodeId: string | null,
  branchType: BranchType,
): { node: ConversationNode; edge: ConversationEdge | null } {
  const timestamp = new Date().toISOString();
  const nodeId = `node-preview-${Date.now()}`;
  const userMessage: Message = {
    id: `message-user-${Date.now()}`,
    conversationId: graph.conversation.id,
    nodeId,
    role: "user",
    contentBlocks: [{ type: "text", text: prompt }],
    createdAt: timestamp,
  };
  const assistantMessage: Message = {
    id: `message-assistant-${Date.now()}`,
    conversationId: graph.conversation.id,
    nodeId,
    role: "assistant",
    contentBlocks: [{ type: "text", text: content }],
    createdAt: timestamp,
  };
  const node: ConversationNode = {
    id: nodeId,
    conversationId: graph.conversation.id,
    parentNodeId,
    branchType,
    title: prompt.length > 30 ? `${prompt.slice(0, 30)}…` : prompt,
    userMessage,
    assistantMessage,
    providerId: "mock",
    modelId: "mock-stream-v1",
    contextSnapshotId: `context-preview-${Date.now()}`,
    runState: "completed",
    createdAt: timestamp,
    updatedAt: timestamp,
    revision: 1,
  };
  const edge = parentNodeId
    ? {
        id: `edge-preview-${Date.now()}`,
        conversationId: graph.conversation.id,
        sourceNodeId: parentNodeId,
        targetNodeId: nodeId,
        relation: branchType,
        createdAt: timestamp,
      }
    : null;
  return { node, edge };
}

export function App() {
  const [mode, setMode] = useState<RuntimeMode>("preview");
  const [booting, setBooting] = useState(true);
  const [workspace, setWorkspace] = useState<Workspace>(previewWorkspace);
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [selectedConversationId, setSelectedConversationId] = useState<string | null>(null);
  const [graph, setGraph] = useState<ConversationGraph | null>(null);
  const [graphLoading, setGraphLoading] = useState(false);
  const [previewGraphs, setPreviewGraphs] = useState<Record<string, ConversationGraph>>({});
  const [selectedParentId, setSelectedParentId] = useState<string | null>(null);
  const [selectedBranchType, setSelectedBranchType] = useState<BranchType>("continues");
  const [viewMode, setViewMode] = useState<"canvas" | "chat">("canvas");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [contextOpen, setContextOpen] = useState(false);
  const [contextLoading, setContextLoading] = useState(false);
  const [contextSnapshot, setContextSnapshot] = useState<ContextSnapshot | null>(null);
  const [contextError, setContextError] = useState<string | null>(null);
  const [run, setRun] = useState<ChatRunState | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);

  useEffect(() => {
    let active = true;
    void kernelClient.bootstrap().then(
      (data: KernelBootstrap) => {
        if (!active) return;
        setMode("tauri");
        setWorkspace(data.workspace);
        setConversations(data.conversations);
        setSelectedConversationId(data.conversations[0]?.id ?? null);
        setBooting(false);
      },
      () => {
        if (!active) return;
        const welcome = newPreviewConversation("欢迎来到 MindScape");
        const welcomeGraph: ConversationGraph = {
          conversation: welcome,
          nodes: [],
          edges: [],
          positions: [],
        };
        setMode("preview");
        setWorkspace(previewWorkspace);
        setConversations([asSummary(welcome)]);
        setPreviewGraphs({ [welcome.id]: welcomeGraph });
        setSelectedConversationId(welcome.id);
        setBooting(false);
      },
    );
    return () => {
      active = false;
      abortControllerRef.current?.abort();
    };
  }, []);

  useEffect(() => {
    if (booting || !selectedConversationId) {
      setGraph(null);
      setSelectedParentId(null);
      setSelectedBranchType("continues");
      return;
    }

    if (mode === "preview") {
      const nextGraph = previewGraphs[selectedConversationId] ?? null;
      setGraph(nextGraph);
      setSelectedParentId(nextGraph?.nodes.at(-1)?.id ?? null);
      setSelectedBranchType("continues");
      return;
    }

    let active = true;
    setGraphLoading(true);
    void kernelClient.loadConversationGraph(selectedConversationId).then(
      (nextGraph) => {
        if (!active) return;
        setGraph(nextGraph);
        setSelectedParentId(nextGraph.nodes.at(-1)?.id ?? null);
        setSelectedBranchType("continues");
        setGraphLoading(false);
      },
      (error: unknown) => {
        if (!active) return;
        setNotice(error instanceof Error ? error.message : String(error));
        setGraphLoading(false);
      },
    );
    return () => {
      active = false;
    };
  }, [booting, mode, previewGraphs, selectedConversationId]);

  const createConversation = async () => {
    const title = `新会话 ${conversations.length + 1}`;
    setNotice(null);
    if (mode === "preview") {
      const conversation = newPreviewConversation(title);
      const nextGraph: ConversationGraph = { conversation, nodes: [], edges: [], positions: [] };
      setPreviewGraphs((current) => ({ ...current, [conversation.id]: nextGraph }));
      setConversations((current) => [asSummary(conversation), ...current]);
      setSelectedConversationId(conversation.id);
      return;
    }

    try {
      const conversation = await kernelClient.createConversation({ workspaceId: workspace.id, title });
      setConversations((current) => [asSummary(conversation), ...current]);
      setSelectedConversationId(conversation.id);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  const persistMockCompletion = async (
    prompt: string,
    content: string,
    parentNodeId: string | null,
    branchType: BranchType,
  ) => {
    if (!graph) return;
    if (mode === "preview") {
      const { node, edge } = createPreviewNode(graph, prompt, content, parentNodeId, branchType);
      const nextGraph: ConversationGraph = {
        ...graph,
        nodes: [...graph.nodes, node],
        edges: edge ? [...graph.edges, edge] : graph.edges,
      };
      setPreviewGraphs((current) => ({ ...current, [graph.conversation.id]: nextGraph }));
      setConversations((current) =>
        current.map((conversation) =>
          conversation.id === graph.conversation.id
            ? { ...conversation, nodeCount: nextGraph.nodes.length, updatedAt: node.updatedAt }
            : conversation,
        ),
      );
      setSelectedParentId(node.id);
      setSelectedBranchType("continues");
      return;
    }

    const pendingNode = await kernelClient.appendTurn({
      conversationId: graph.conversation.id,
      parentNodeId,
      branchType,
      title: prompt.length > 30 ? `${prompt.slice(0, 30)}…` : prompt,
      prompt,
      providerId: "mock",
      modelId: "mock-stream-v1",
    });
    await kernelClient.completeTurn({
      nodeId: pendingNode.id,
      content,
      providerId: "mock",
      modelId: "mock-stream-v1",
    });
    const nextGraph = await kernelClient.loadConversationGraph(graph.conversation.id);
    setGraph(nextGraph);
    setSelectedParentId(nextGraph.nodes.at(-1)?.id ?? null);
    setSelectedBranchType("continues");
    setConversations((current) =>
      current.map((conversation) =>
        conversation.id === nextGraph.conversation.id
          ? { ...conversation, ...nextGraph.conversation, nodeCount: nextGraph.nodes.length }
          : conversation,
      ),
    );
  };

  const sendPrompt = async (prompt: string) => {
    if (!graph || abortControllerRef.current) return;
    setNotice(null);
    const controller = new AbortController();
    const runId = `run-mock-${Date.now()}`;
    const transientNodeId = `node-${runId}`;
    const parentNodeId = selectedParentId;
    const branchType = parentNodeId ? selectedBranchType : "continues";
    const parentTitle = graph.nodes.find((node) => node.id === parentNodeId)?.title;
    let completedContent = "";
    let terminalEvent: "completed" | "cancelled" | "failed" | null = null;
    abortControllerRef.current = controller;

    if (mode === "tauri") {
      try {
        const pendingNode = await kernelClient.appendTurn({
          conversationId: graph.conversation.id,
          parentNodeId,
          branchType,
          title: prompt.length > 30 ? `${prompt.slice(0, 30)}…` : prompt,
          prompt,
          providerId: "mock",
          modelId: "mock-stream-v1",
        });
        const snapshot = await kernelClient.getContextSnapshot(pendingNode.contextSnapshotId);
        const request: ModelRunRequest = {
          contractVersion: "mindscape.runtime.v1",
          runId,
          conversationId: graph.conversation.id,
          nodeId: pendingNode.id,
          contextSnapshot: snapshot,
          providerId: "mock",
          modelId: "mock-stream-v1",
          capabilities: ["textInput", "usageReporting"],
          budget: {
            maxOutputTokens: 1024,
            maxCostMicrounits: 0,
            timeoutMs: 30_000,
          },
          idempotencyKey: runId,
          createdAt: new Date().toISOString(),
        };
        setRun(createChatRunState({
          runId,
          nodeId: pendingNode.id,
          prompt,
          parentNodeId,
          branchType,
        }));
        await kernelClient.runModel(request, (envelope) => {
          if (envelope.event.type === "text_delta") completedContent += envelope.event.delta;
          if (
            envelope.event.type === "completed" ||
            envelope.event.type === "cancelled" ||
            envelope.event.type === "failed"
          ) {
            terminalEvent = envelope.event.type;
          }
          setRun((current) =>
            current?.id === runId ? reduceModelRunEnvelope(current, envelope) : current,
          );
        });
        abortControllerRef.current = null;
        if (terminalEvent !== "completed" || !completedContent) return;
        await kernelClient.completeTurn({
          nodeId: pendingNode.id,
          content: completedContent,
          providerId: "mock",
          modelId: "mock-stream-v1",
        });
        const nextGraph = await kernelClient.loadConversationGraph(graph.conversation.id);
        setGraph(nextGraph);
        setSelectedParentId(pendingNode.id);
        setSelectedBranchType("continues");
        setConversations((current) => current.map((conversation) =>
          conversation.id === nextGraph.conversation.id
            ? { ...conversation, ...nextGraph.conversation, nodeCount: nextGraph.nodes.length }
            : conversation,
        ));
        window.setTimeout(() => setRun((current) => current?.id === runId ? null : current), 550);
      } catch (error) {
        abortControllerRef.current = null;
        const safeMessage = error instanceof Error ? error.message : String(error);
        setNotice(safeMessage);
      }
      return;
    }

    setRun(createChatRunState({
      runId,
      nodeId: transientNodeId,
      prompt,
      parentNodeId,
      branchType,
    }));

    await runMockModel({
      runId,
      nodeId: transientNodeId,
      prompt,
      parentTitle,
      signal: controller.signal,
      onEvent: (envelope) => {
        if (envelope.event.type === "text_delta") {
          completedContent += envelope.event.delta;
        }
        setRun((current) =>
          current?.id === runId ? reduceModelRunEnvelope(current, envelope) : current,
        );
      },
    });

    abortControllerRef.current = null;
    if (controller.signal.aborted || !completedContent) return;
    try {
      await persistMockCompletion(prompt, completedContent, parentNodeId, branchType);
      window.setTimeout(() => {
        setRun((current) => current?.id === runId ? null : current);
      }, 550);
    } catch (error) {
      const safeMessage = error instanceof Error ? error.message : String(error);
      setRun((current) =>
        current?.id === runId
          ? {
              ...current,
              status: "failed",
              error: {
                category: "unknown",
                providerCode: "persistence_failed",
                safeMessage,
                retryable: true,
                retryAfterMs: null,
                providerStatus: null,
              },
              errorMessage: safeMessage,
              partialContentRetained: true,
            }
          : current,
      );
    }
  };

  const cancelRun = () => {
    if (mode === "tauri" && run) {
      void kernelClient.cancelModelRun(run.id);
      return;
    }
    abortControllerRef.current?.abort();
    abortControllerRef.current = null;
  };

  const retryRun = () => {
    if (!run) return;
    const prompt = run.prompt;
    setRun(null);
    void sendPrompt(prompt);
  };

  const inspectContext = async (node: ConversationNode) => {
    setContextOpen(true);
    setContextSnapshot(null);
    setContextError(null);
    if (mode === "preview") {
      setContextError("浏览器预览不会伪造上下文快照；请在 Tauri 本地内核中查看真实冻结数据。");
      return;
    }
    setContextLoading(true);
    try {
      setContextSnapshot(await kernelClient.getContextSnapshot(node.contextSnapshotId));
    } catch (error) {
      setContextError(error instanceof Error ? error.message : String(error));
    } finally {
      setContextLoading(false);
    }
  };

  const moveNode = async (nodeId: string, position: CanvasPoint) => {
    if (!graph) return;
    const nextPositions = [
      ...graph.positions.filter((item) => item.nodeId !== nodeId),
      { nodeId, x: position.x, y: position.y },
    ];
    const nextGraph = { ...graph, positions: nextPositions };
    setGraph(nextGraph);

    if (mode === "preview") {
      setPreviewGraphs((current) => ({ ...current, [graph.conversation.id]: nextGraph }));
      return;
    }

    try {
      await kernelClient.updateNodePosition({
        conversationId: graph.conversation.id,
        nodeId,
        x: position.x,
        y: position.y,
      });
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="app-shell">
      <WorkspaceSidebar
        open={sidebarOpen}
        workspace={workspace}
        conversations={conversations}
        selectedConversationId={selectedConversationId}
        onToggle={() => setSidebarOpen((value) => !value)}
        onCreateConversation={() => void createConversation()}
        onSelectConversation={(conversationId) => {
          if (abortControllerRef.current) return;
          setRun(null);
          setSelectedConversationId(conversationId);
        }}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <ChatWorkspace
        graph={graph}
        loading={booting || graphLoading}
        sidebarOpen={sidebarOpen}
        runtimeLabel={mode === "tauri" ? "本地内核 · 模拟模型" : "浏览器预览 · 不保存"}
        selectedParentId={selectedParentId}
        selectedBranchType={selectedBranchType}
        viewMode={viewMode}
        run={run}
        onToggleSidebar={() => setSidebarOpen(true)}
        onSelectParent={(nodeId) => {
          setSelectedParentId(nodeId);
          setSelectedBranchType("continues");
        }}
        onSelectBranch={(nodeId, branchType) => {
          setSelectedParentId(nodeId);
          setSelectedBranchType(branchType);
        }}
        onChangeViewMode={setViewMode}
        onMoveNode={(nodeId, position) => void moveNode(nodeId, position)}
        onClearParent={() => {
          setSelectedParentId(null);
          setSelectedBranchType("continues");
        }}
        onCreateConversation={() => void createConversation()}
        onSend={(prompt) => void sendPrompt(prompt)}
        onCancel={cancelRun}
        onRetry={retryRun}
        onInspectContext={(node) => void inspectContext(node)}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      {notice ? (
        <div className="notice-toast" role="alert">
          <span>{notice}</span>
          <button type="button" onClick={() => setNotice(null)} aria-label="关闭错误提示">×</button>
        </div>
      ) : null}
      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <ContextDialog
        open={contextOpen}
        loading={contextLoading}
        snapshot={contextSnapshot}
        error={contextError}
        onClose={() => setContextOpen(false)}
      />
    </div>
  );
}

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  createChatRunState,
  createChatRunStateFromProjection,
  rejectChatRunCancellation,
  requestChatRunCancellation,
  reduceModelRunEnvelope,
  type ChatRunState,
} from "./app/chatRunState";
import { commandErrorMessage as safeErrorMessage } from "./app/commandErrorPresentation";
import { runMockModel } from "./app/mockModelRuntime";
import { kernelClient } from "./app/kernelClient";
import {
  buildChatModelOptions,
  chooseModelSelection,
} from "./app/providerCatalog";
import { ChatWorkspace } from "./components/ChatWorkspace";
import { ContextDialog } from "./components/ContextDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import {
  CanvasViewportPersistence,
  loadCanvasViewport,
} from "./canvas/canvasViewportPersistence";
import type { CanvasPoint, CanvasViewport } from "./canvas/graphProjection";
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
  ModelSelection,
  ModelRunProjection,
  ProviderDescriptor,
  Workspace,
} from "./domain";

type RuntimeMode = "tauri" | "preview";

const previewWorkspace: Workspace = {
  id: "workspace-preview",
  name: "UI 预览工作区",
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

const previewProvider: ProviderDescriptor = {
  id: "mock",
  displayName: "Mock Provider",
  defaultBaseUrl: null,
  customBaseUrlAllowed: false,
  credentialRequired: false,
  models: {
    "mock-stream-v1": {
      textInput: true,
      imageInput: false,
      toolCalling: false,
      usageReporting: true,
      streaming: true,
      contextWindowTokens: 16_384,
    },
  },
};

const DEFAULT_PROVIDER_ACCOUNT = "default";
const DEFAULT_MAX_OUTPUT_TOKENS = 8_192;

function messageToPrompt(message: Message) {
  return message.contentBlocks
    .map((block) => {
      if (block.type === "text") return block.text;
      if (block.type === "code") return block.code;
      if (block.type === "link") return block.label ?? block.url;
      return "";
    })
    .filter(Boolean)
    .join("\n");
}

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
  const [modelRuns, setModelRuns] = useState<ModelRunProjection[]>([]);
  const [canvasViewport, setCanvasViewport] = useState<CanvasViewport | null>(null);
  const [runSubmitting, setRunSubmitting] = useState(false);
  const [providers, setProviders] = useState<ProviderDescriptor[]>([previewProvider]);
  const [providerCredentials, setProviderCredentials] = useState<Record<string, boolean>>({ mock: true });
  const [providersLoading, setProvidersLoading] = useState(false);
  const [providersError, setProvidersError] = useState<string | null>(null);
  // Resolve the initial selection after Tauri reports provider credentials. A
  // hard-coded Mock selection would be treated as an explicit user choice and
  // prevent an available real Provider from becoming the default.
  const [selectedModel, setSelectedModel] = useState<ModelSelection | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const runInFlightRef = useRef(false);
  const viewportPersistenceRef = useRef<CanvasViewportPersistence | null>(null);

  const refreshProviders = useCallback(async () => {
    setProvidersLoading(true);
    setProvidersError(null);
    try {
      const descriptors = await kernelClient.listProviders();
      const credentialEntries = await Promise.all(
        descriptors.map(async (provider) => [
          provider.id,
          provider.credentialRequired
            ? await kernelClient.hasProviderCredential({
                providerId: provider.id,
                accountId: DEFAULT_PROVIDER_ACCOUNT,
              })
            : true,
        ] as const),
      );
      const credentialStatus = Object.fromEntries(credentialEntries);
      setProviders(descriptors);
      setProviderCredentials(credentialStatus);
      setSelectedModel((current) => chooseModelSelection(descriptors, credentialStatus, current));
    } catch (error) {
      setProvidersError(safeErrorMessage(error));
    } finally {
      setProvidersLoading(false);
    }
  }, []);

  const modelOptions = useMemo(
    () => buildChatModelOptions(providers, providerCredentials),
    [providerCredentials, providers],
  );

  useEffect(() => {
    if (mode !== "tauri") {
      viewportPersistenceRef.current = null;
      return;
    }
    const persistence = new CanvasViewportPersistence(
      (input) => kernelClient.saveCanvasViewport(input),
      {
        onError: (error) => {
          setNotice(`画布视口暂未保存：${safeErrorMessage(error)}`);
        },
      },
    );
    viewportPersistenceRef.current = persistence;
    return () => {
      void persistence.flushAll();
      if (viewportPersistenceRef.current === persistence) {
        viewportPersistenceRef.current = null;
      }
    };
  }, [mode]);

  useEffect(() => {
    const persistence = viewportPersistenceRef.current;
    const conversationId = selectedConversationId;
    return () => {
      if (persistence && conversationId) void persistence.flush(conversationId);
    };
  }, [selectedConversationId]);

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
        void refreshProviders();
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
        setProviders([previewProvider]);
        setProviderCredentials({ mock: true });
        setSelectedModel({ providerId: "mock", modelId: "mock-stream-v1" });
        setSelectedConversationId(welcome.id);
        setBooting(false);
      },
    );
    return () => {
      active = false;
      abortControllerRef.current?.abort();
      runInFlightRef.current = false;
    };
  }, [refreshProviders]);

  useEffect(() => {
    if (booting || !selectedConversationId) {
      setGraph(null);
      setModelRuns([]);
      setCanvasViewport(null);
      setSelectedParentId(null);
      setSelectedBranchType("continues");
      return;
    }

    if (mode === "preview") {
      const nextGraph = previewGraphs[selectedConversationId] ?? null;
      setGraph(nextGraph);
      setModelRuns([]);
      setCanvasViewport(null);
      setSelectedParentId(nextGraph?.nodes.at(-1)?.id ?? null);
      setSelectedBranchType("continues");
      return;
    }

    let active = true;
    setGraphLoading(true);
    setCanvasViewport(null);
    const viewportRequest = loadCanvasViewport(
      kernelClient.getCanvasViewport,
      selectedConversationId,
    ).then(
      (viewport) => ({ viewport, error: null }),
      (error: unknown) => ({ viewport: null, error }),
    );
    void Promise.all([
      kernelClient.loadConversationGraph(selectedConversationId),
      kernelClient.listModelRuns(selectedConversationId),
      viewportRequest,
    ]).then(
      ([nextGraph, modelRuns, viewportResult]) => {
        if (!active) return;
        setGraph(nextGraph);
        setModelRuns(modelRuns);
        setCanvasViewport(viewportResult.viewport);
        if (viewportResult.error) {
          setNotice(`会话已恢复，但画布视口读取失败：${safeErrorMessage(viewportResult.error)}`);
        }
        setSelectedParentId(nextGraph.nodes.at(-1)?.id ?? null);
        setSelectedBranchType("continues");
        if (!runInFlightRef.current) {
          const latestRecoverable = modelRuns
            .filter((modelRun) => modelRun.state !== "completed")
            .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))[0];
          if (latestRecoverable) {
            const node = nextGraph.nodes.find((candidate) => candidate.id === latestRecoverable.nodeId);
            setRun(createChatRunStateFromProjection(latestRecoverable, {
              prompt: node ? messageToPrompt(node.userMessage) : "恢复的模型运行",
              parentNodeId: node?.parentNodeId ?? null,
              branchType: node?.branchType ?? "continues",
            }));
          } else {
            setRun(null);
          }
        }
        setGraphLoading(false);
      },
      (error: unknown) => {
        if (!active) return;
        setNotice(safeErrorMessage(error));
        setGraphLoading(false);
      },
    );
    return () => {
      active = false;
    };
  }, [booting, mode, previewGraphs, selectedConversationId]);

  const createConversation = async () => {
    if (runInFlightRef.current) {
      setNotice("请先停止当前模型运行，再切换或创建会话。");
      return;
    }
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

  const refreshProviderSettings = async () => {
    if (mode === "preview") {
      setProviders([previewProvider]);
      setProviderCredentials({ mock: true });
      setProvidersError(null);
      return;
    }
    await refreshProviders();
  };

  const saveProviderCredential = async (providerId: string, secret: string) => {
    if (mode !== "tauri") throw new Error("浏览器预览不能访问操作系统安全凭据。");
    try {
      await kernelClient.setProviderCredential({
        providerId,
        accountId: DEFAULT_PROVIDER_ACCOUNT,
        secret,
      });
      await refreshProviders();
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
  };

  const deleteProviderCredential = async (providerId: string) => {
    if (mode !== "tauri") throw new Error("浏览器预览不能访问操作系统安全凭据。");
    try {
      await kernelClient.deleteProviderCredential({
        providerId,
        accountId: DEFAULT_PROVIDER_ACCOUNT,
      });
      await refreshProviders();
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
  };

  const testProviderConnection = async (providerId: string) => {
    if (mode !== "tauri") throw new Error("浏览器预览不能测试真实 Provider 连接。");
    try {
      return await kernelClient.testProviderConnection(providerId);
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
  };

  const persistMockCompletion = async (
    prompt: string,
    content: string,
    parentNodeId: string | null,
    branchType: BranchType,
  ) => {
    if (!graph || mode !== "preview") return;
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
  };

  const sendPrompt = async (prompt: string, modelOverride?: ModelSelection) => {
    if (!graph || runInFlightRef.current) return;
    const selection = modelOverride ?? selectedModel;
    const selectedOption = modelOptions.find(
      (option) => option.providerId === selection?.providerId && option.modelId === selection.modelId,
    );
    const selectedCapabilities = providers
      .find((provider) => provider.id === selection?.providerId)
      ?.models[selection?.modelId ?? ""];
    if (!selection || !selectedOption?.available) {
      setNotice("请先在模型设置中选择一个可用模型；真实 Provider 需要先安全保存 API Key。");
      setSettingsOpen(true);
      return;
    }
    setNotice(null);
    const parentNodeId = selectedParentId;
    const branchType = parentNodeId ? selectedBranchType : "continues";
    const parentTitle = graph.nodes.find((node) => node.id === parentNodeId)?.title;
    runInFlightRef.current = true;
    setRunSubmitting(true);

    if (mode === "tauri") {
      try {
        const idempotencyKey = `chat-${crypto.randomUUID()}`;
        const projection = await kernelClient.startModelRun({
          conversationId: graph.conversation.id,
          parentNodeId,
          branchType,
          title: prompt.length > 30 ? `${prompt.slice(0, 30)}…` : prompt,
          prompt,
          providerId: selection.providerId,
          modelId: selection.modelId,
          capabilities: selectedCapabilities?.usageReporting
            ? ["textInput", "usageReporting"]
            : ["textInput"],
          budget: {
            maxOutputTokens: DEFAULT_MAX_OUTPUT_TOKENS,
            maxCostMicrounits: selectedOption.isMock ? 0 : null,
            timeoutMs: 120_000,
          },
          idempotencyKey,
        }, (envelope) => {
          setRun((current) =>
            reduceModelRunEnvelope(
              current?.id === envelope.runId
                ? current
                : createChatRunState({
                    runId: envelope.runId,
                    nodeId: envelope.nodeId,
                    providerId: selection.providerId,
                    modelId: selection.modelId,
                    prompt,
                    parentNodeId,
                    branchType,
                  }),
              envelope,
            ),
          );
        });
        const nextGraph = await kernelClient.loadConversationGraph(graph.conversation.id);
        setGraph(nextGraph);
        setModelRuns((current) => [
          ...current.filter((modelRun) => modelRun.runId !== projection.runId),
          projection,
        ]);
        setSelectedParentId(projection.nodeId);
        setSelectedBranchType("continues");
        setConversations((current) => current.map((conversation) =>
          conversation.id === nextGraph.conversation.id
            ? { ...conversation, ...nextGraph.conversation, nodeCount: nextGraph.nodes.length }
            : conversation,
        ));
        if (projection.state === "completed") {
          setRun(null);
        } else {
          setRun(createChatRunStateFromProjection(projection, { prompt, parentNodeId, branchType }));
        }
      } catch (error) {
        setNotice(safeErrorMessage(error));
        try {
          const [nextGraph, nextModelRuns] = await Promise.all([
            kernelClient.loadConversationGraph(graph.conversation.id),
            kernelClient.listModelRuns(graph.conversation.id),
          ]);
          setGraph(nextGraph);
          setModelRuns(nextModelRuns);
          setConversations((current) => current.map((conversation) =>
            conversation.id === nextGraph.conversation.id
              ? { ...conversation, ...nextGraph.conversation, nodeCount: nextGraph.nodes.length }
              : conversation,
          ));
        } catch {
          // The original structured command error remains the actionable user message.
        }
      } finally {
        runInFlightRef.current = false;
        setRunSubmitting(false);
      }
      return;
    }

    const controller = new AbortController();
    const runId = `run-mock-${Date.now()}`;
    const transientNodeId = `node-${runId}`;
    let completedContent = "";
    abortControllerRef.current = controller;
    setRun(createChatRunState({
      runId,
      nodeId: transientNodeId,
      providerId: selection.providerId,
      modelId: selection.modelId,
      prompt,
      parentNodeId,
      branchType,
    }));

    try {
      await runMockModel({
        runId,
        nodeId: transientNodeId,
        prompt,
        parentTitle,
        signal: controller.signal,
        onEvent: (envelope) => {
          if (envelope.event.type === "text_delta") completedContent += envelope.event.delta;
          setRun((current) =>
            current?.id === runId ? reduceModelRunEnvelope(current, envelope) : current,
          );
        },
      });
      if (controller.signal.aborted || !completedContent) return;
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
    } finally {
      abortControllerRef.current = null;
      runInFlightRef.current = false;
      setRunSubmitting(false);
    }
  };

  const cancelRun = () => {
    if (!run || run.cancelRequested || (run.status !== "starting" && run.status !== "streaming")) {
      return;
    }

    const runId = run.id;
    setNotice(null);
    setRun((current) => current?.id === runId ? requestChatRunCancellation(current) : current);

    if (mode === "tauri") {
      void kernelClient.cancelModelRun(runId).then((cancelled) => {
        if (cancelled) return;
        const message = "当前运行已经结束或无法取消，请刷新运行状态。";
        setRun((current) => current?.id === runId
          ? rejectChatRunCancellation(current, message)
          : current);
        setNotice(message);
      }, (error) => {
        const message = safeErrorMessage(error);
        setRun((current) => current?.id === runId
          ? rejectChatRunCancellation(current, message)
          : current);
        setNotice(message);
      });
      return;
    }
    abortControllerRef.current?.abort();
    abortControllerRef.current = null;
  };

  const retryRun = () => {
    if (!run) return;
    const prompt = run.prompt;
    const retryModel = { providerId: run.providerId, modelId: run.modelId };
    setSelectedModel(retryModel);
    setRun(null);
    void sendPrompt(prompt, retryModel);
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

  const activeModelOption = modelOptions.find(
    (option) =>
      option.providerId === selectedModel?.providerId && option.modelId === selectedModel.modelId,
  );

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
          if (runInFlightRef.current) return;
          setRun(null);
          setSelectedConversationId(conversationId);
        }}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <ChatWorkspace
        graph={graph}
        modelRuns={modelRuns}
        initialCanvasViewport={canvasViewport}
        loading={booting || graphLoading}
        sidebarOpen={sidebarOpen}
        runtimeLabel={
          mode === "preview"
            ? "浏览器预览 · 不保存"
            : !activeModelOption
              ? "本地内核 · 未选择模型"
              : activeModelOption.isMock
              ? "本地内核 · 本地测试模型"
              : activeModelOption.available
                ? "本地内核 · 真实 API"
                : "本地内核 · 真实 API 缺少 Key"
        }
        selectedParentId={selectedParentId}
        selectedBranchType={selectedBranchType}
        viewMode={viewMode}
        run={run}
        runSubmitting={runSubmitting}
        modelOptions={modelOptions}
        selectedModel={selectedModel}
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
        onCanvasViewportChange={(conversationId, viewport) => {
          if (mode === "tauri") {
            viewportPersistenceRef.current?.schedule(conversationId, viewport);
          }
        }}
        onClearParent={() => {
          setSelectedParentId(null);
          setSelectedBranchType("continues");
        }}
        onCreateConversation={() => void createConversation()}
        onSend={(prompt) => void sendPrompt(prompt)}
        onCancel={cancelRun}
        onRetry={retryRun}
        onSelectModel={setSelectedModel}
        onInspectContext={(node) => void inspectContext(node)}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      {notice ? (
        <div className="notice-toast" role="alert">
          <span>{notice}</span>
          <button type="button" onClick={() => setNotice(null)} aria-label="关闭错误提示">×</button>
        </div>
      ) : null}
      <SettingsDialog
        open={settingsOpen}
        loading={providersLoading}
        error={providersError}
        providers={providers}
        credentialStatus={providerCredentials}
        selectedModel={selectedModel}
        onClose={() => setSettingsOpen(false)}
        onRefresh={refreshProviderSettings}
        onSelectModel={setSelectedModel}
        onSaveCredential={saveProviderCredential}
        onDeleteCredential={deleteProviderCredential}
        onTestConnection={testProviderConnection}
      />
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

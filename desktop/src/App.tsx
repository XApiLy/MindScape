import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
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
  loadMarkdownProjections,
  prependMarkdownProjectionRevision,
} from "./app/markdownProjectionLoader";
import {
  buildChatModelOptions,
  chooseModelSelection,
} from "./app/providerCatalog";
import {
  DEFAULT_READING_PREFERENCES,
  loadReadingPreferences,
  normalizeReadingPreferences,
  resolveReadingParagraphSpacingPx,
  saveReadingPreferences,
  type ReadingPreferences,
} from "./app/readingPreferences";
import { ChatWorkspace } from "./components/ChatWorkspace";
import { ContextDialog } from "./components/ContextDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import {
  CanvasViewportPersistence,
  loadCanvasViewport,
} from "./canvas/canvasViewportPersistence";
import {
  projectFocusFrameQuery,
  projectKnowledgeRetrieval,
  upsertFocusFrameQueryByNodeId,
  type CanvasFocusFrameQueryProjection,
  type CanvasKnowledgeRetrievalProjection,
} from "./canvas/canvasM2Projection";
import type { CanvasPoint, CanvasViewport } from "./canvas/graphProjection";
import type {
  BranchType,
  Conversation,
  ConversationEdge,
  ConversationGraph,
  ConversationNode,
  ConversationSummary,
  ContextSnapshot,
  EffectiveRunProfile,
  FocusFrame,
  FocusFrameLifecycleCommandInput,
  FocusPromotionCandidateSet,
  FocusPromotionDecisionCommandInput,
  FocusPromotionDecisionProjection,
  GenericImportCommandResult,
  ImportBundleQueryProjection,
  ImportSource,
  KnowledgeEntity,
  KnowledgeRelation,
  KnowledgeRetrievalProjection,
  KernelBootstrap,
  MarkdownEditCommandResult,
  MarkdownProjection,
  Message,
  ModelSelection,
  ModelRunProjection,
  ProviderDescriptor,
  RawImportContentProjection,
  SemanticModelPackStatus,
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
      supportsReasoning: false,
      reasoningControl: "none",
      reasoningModes: [],
      structuredOutput: false,
      generationParameters: {
        maxOutputTokens: "supported",
        temperature: "unsupported",
        topP: "unsupported",
        seed: "unsupported",
      },
      inputModalities: ["text"],
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
  const [focusFrameQueryByNodeId, setFocusFrameQueryByNodeId] = useState<ReadonlyMap<string, CanvasFocusFrameQueryProjection>>(new Map());
  const [focusFrameQueryError, setFocusFrameQueryError] = useState<string | null>(null);
  const [importSources, setImportSources] = useState<ImportSource[]>([]);
  const [importSourcesLoading, setImportSourcesLoading] = useState(false);
  const [knowledgeEntities, setKnowledgeEntities] = useState<KnowledgeEntity[]>([]);
  const [knowledgeRelations, setKnowledgeRelations] = useState<KnowledgeRelation[]>([]);
  const [knowledgeLoading, setKnowledgeLoading] = useState(false);
  const [knowledgeError, setKnowledgeError] = useState<string | null>(null);
  const [markdownProjectionsByEntityId, setMarkdownProjectionsByEntityId] = useState<ReadonlyMap<string, readonly MarkdownProjection[]>>(new Map());
  const [markdownProjectionErrorsByEntityId, setMarkdownProjectionErrorsByEntityId] = useState<ReadonlyMap<string, string>>(new Map());
  const [markdownProjectionsLoading, setMarkdownProjectionsLoading] = useState(false);
  const [knowledgeRetrievalByNodeId, setKnowledgeRetrievalByNodeId] = useState<ReadonlyMap<string, CanvasKnowledgeRetrievalProjection>>(new Map());
  const [knowledgeRetrievalLoadingNodeId, setKnowledgeRetrievalLoadingNodeId] = useState<string | null>(null);
  const [knowledgeRetrievalErrorByNodeId, setKnowledgeRetrievalErrorByNodeId] = useState<ReadonlyMap<string, string>>(new Map());
  const [canvasViewport, setCanvasViewport] = useState<CanvasViewport | null>(null);
  const [runSubmitting, setRunSubmitting] = useState(false);
  const [providers, setProviders] = useState<ProviderDescriptor[]>([previewProvider]);
  const [providerCredentials, setProviderCredentials] = useState<Record<string, boolean>>({ mock: true });
  const [providersLoading, setProvidersLoading] = useState(false);
  const [providersError, setProvidersError] = useState<string | null>(null);
  const [persistedReadingPreferences, setPersistedReadingPreferences] = useState<ReadingPreferences>(
    DEFAULT_READING_PREFERENCES,
  );
  const [sessionReadingPreferences, setSessionReadingPreferences] = useState<ReadingPreferences | null>(null);
  // Resolve the initial selection after Tauri reports provider credentials. A
  // hard-coded Mock selection would be treated as an explicit user choice and
  // prevent an available real Provider from becoming the default.
  const [selectedModel, setSelectedModel] = useState<ModelSelection | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const runInFlightRef = useRef(false);
  const viewportPersistenceRef = useRef<CanvasViewportPersistence | null>(null);
  const readingPreferences = sessionReadingPreferences ?? persistedReadingPreferences;

  useEffect(() => {
    const restored = loadReadingPreferences(window.localStorage, workspace.id);
    setPersistedReadingPreferences(restored);
    setSessionReadingPreferences(null);
  }, [workspace.id]);

  const updateReadingPreferences = useCallback((
    nextPreferences: ReadingPreferences,
    scope: "workspace" | "session",
  ) => {
    const normalized = normalizeReadingPreferences(nextPreferences);
    if (scope === "session") {
      setSessionReadingPreferences(normalized);
      return;
    }
    if (!saveReadingPreferences(window.localStorage, workspace.id, normalized)) {
      setNotice("阅读偏好未能写入本地工作区，本次仍会继续使用当前预览。");
      setSessionReadingPreferences(normalized);
      return;
    }
    setPersistedReadingPreferences(normalized);
    setSessionReadingPreferences(null);
  }, [workspace.id]);

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
      setFocusFrameQueryByNodeId(new Map());
      setFocusFrameQueryError(null);
      setImportSources([]);
      setImportSourcesLoading(false);
      setKnowledgeEntities([]);
      setKnowledgeRelations([]);
      setKnowledgeLoading(false);
      setKnowledgeError(null);
      setMarkdownProjectionsByEntityId(new Map());
      setMarkdownProjectionErrorsByEntityId(new Map());
      setMarkdownProjectionsLoading(false);
      setKnowledgeRetrievalByNodeId(new Map());
      setKnowledgeRetrievalLoadingNodeId(null);
      setKnowledgeRetrievalErrorByNodeId(new Map());
      setCanvasViewport(null);
      setSelectedParentId(null);
      setSelectedBranchType("continues");
      return;
    }

    if (mode === "preview") {
      const nextGraph = previewGraphs[selectedConversationId] ?? null;
      setGraph(nextGraph);
      setModelRuns([]);
      setFocusFrameQueryByNodeId(new Map());
      setFocusFrameQueryError(null);
      setImportSources([]);
      setImportSourcesLoading(false);
      setKnowledgeEntities([]);
      setKnowledgeRelations([]);
      setKnowledgeLoading(false);
      setKnowledgeError(null);
      setMarkdownProjectionsByEntityId(new Map());
      setMarkdownProjectionErrorsByEntityId(new Map());
      setMarkdownProjectionsLoading(false);
      setKnowledgeRetrievalByNodeId(new Map());
      setKnowledgeRetrievalLoadingNodeId(null);
      setKnowledgeRetrievalErrorByNodeId(new Map());
      setCanvasViewport(null);
      setSelectedParentId(nextGraph?.nodes.at(-1)?.id ?? null);
      setSelectedBranchType("continues");
      return;
    }

    let active = true;
    setGraphLoading(true);
    setFocusFrameQueryError(null);
    setImportSourcesLoading(true);
    setKnowledgeLoading(true);
    setKnowledgeError(null);
    setMarkdownProjectionsByEntityId(new Map());
    setMarkdownProjectionErrorsByEntityId(new Map());
    setMarkdownProjectionsLoading(true);
    setKnowledgeRetrievalByNodeId(new Map());
    setKnowledgeRetrievalLoadingNodeId(null);
    setKnowledgeRetrievalErrorByNodeId(new Map());
    setCanvasViewport(null);
    const viewportRequest = loadCanvasViewport(
      kernelClient.getCanvasViewport,
      selectedConversationId,
    ).then(
      (viewport) => ({ viewport, error: null }),
      (error: unknown) => ({ viewport: null, error }),
    );
    const focusFrameRequest = kernelClient.listFocusFrames(selectedConversationId).then(
      (queries) => ({ queries, error: null as unknown }),
      (error: unknown) => ({ queries: [], error }),
    );
    const importSourcesRequest = kernelClient.listImportSources(selectedConversationId).then(
      (sources) => ({ sources, error: null as unknown }),
      (error: unknown) => ({ sources: [], error }),
    );
    const knowledgeEntitiesRequest = kernelClient.listKnowledgeEntities(selectedConversationId).then(
      (entities) => ({ entities, error: null as unknown }),
      (error: unknown) => ({ entities: [], error }),
    );
    const knowledgeRelationsRequest = kernelClient.listKnowledgeRelations(selectedConversationId).then(
      (relations) => ({ relations, error: null as unknown }),
      (error: unknown) => ({ relations: [], error }),
    );
    const markdownProjectionsRequest = knowledgeEntitiesRequest.then((result) => (
      result.error
        ? loadMarkdownProjections([], kernelClient.listMarkdownProjections)
        : loadMarkdownProjections(
            result.entities.map((entity) => entity.id),
            kernelClient.listMarkdownProjections,
          )
    ));
    void Promise.all([
      kernelClient.loadConversationGraph(selectedConversationId),
      kernelClient.listModelRuns(selectedConversationId),
      viewportRequest,
      focusFrameRequest,
      importSourcesRequest,
      knowledgeEntitiesRequest,
      knowledgeRelationsRequest,
      markdownProjectionsRequest,
    ]).then(
      ([nextGraph, modelRuns, viewportResult, focusFrameResult, importSourcesResult, knowledgeEntitiesResult, knowledgeRelationsResult, markdownProjectionsResult]) => {
        if (!active) return;
        setGraph(nextGraph);
        setModelRuns(modelRuns);
        let indexedFocusFrames = new Map<string, CanvasFocusFrameQueryProjection>();
        for (const query of focusFrameResult.queries) {
          const projected = projectFocusFrameQuery(query);
          if (projected) {
            indexedFocusFrames = upsertFocusFrameQueryByNodeId(indexedFocusFrames, projected);
          }
        }
        setFocusFrameQueryByNodeId(indexedFocusFrames);
        setFocusFrameQueryError(focusFrameResult.error ? safeErrorMessage(focusFrameResult.error) : null);
        setImportSources(importSourcesResult.sources);
        setImportSourcesLoading(false);
        setKnowledgeEntities(knowledgeEntitiesResult.entities);
        setKnowledgeRelations(knowledgeRelationsResult.relations);
        setKnowledgeLoading(false);
        setMarkdownProjectionsByEntityId(markdownProjectionsResult.projectionsByEntityId);
        setMarkdownProjectionErrorsByEntityId(new Map(
          [...markdownProjectionsResult.errorsByEntityId].map(([entityId, error]) => [
            entityId,
            safeErrorMessage(error),
          ]),
        ));
        setMarkdownProjectionsLoading(false);
        const nextKnowledgeError = knowledgeEntitiesResult.error ?? knowledgeRelationsResult.error;
        setKnowledgeError(nextKnowledgeError ? safeErrorMessage(nextKnowledgeError) : null);
        setCanvasViewport(viewportResult.viewport);
        if (viewportResult.error) {
          setNotice(`会话已恢复，但画布视口读取失败：${safeErrorMessage(viewportResult.error)}`);
        }
        if (focusFrameResult.error) {
          setNotice(`会话已恢复，但 FocusFrame 状态读取失败：${safeErrorMessage(focusFrameResult.error)}`);
        }
        if (importSourcesResult.error) {
          setNotice(`会话已恢复，但导入来源读取失败：${safeErrorMessage(importSourcesResult.error)}`);
        }
        if (nextKnowledgeError) {
          setNotice(`会话已恢复，但知识对象读取失败：${safeErrorMessage(nextKnowledgeError)}`);
        } else if (markdownProjectionsResult.errorsByEntityId.size) {
          setNotice(`会话已恢复，但 ${markdownProjectionsResult.errorsByEntityId.size} 个知识实体的 Markdown 投影读取失败。`);
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
        setImportSourcesLoading(false);
        setKnowledgeLoading(false);
        setMarkdownProjectionsLoading(false);
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

  const getSemanticModelStatus = useCallback(async (): Promise<SemanticModelPackStatus> => {
    if (mode !== "tauri") {
      return { state: "missing", modelVersion: "preview", missingFiles: [] };
    }
    return kernelClient.getSemanticModelPackStatus();
  }, [mode]);

  const installSemanticModel = useCallback(async () => {
    if (mode !== "tauri") throw new Error("浏览器预览不能安装本地语义模型。");
    try {
      return await kernelClient.installSemanticModelPack();
    } catch (installError) {
      throw new Error(safeErrorMessage(installError));
    }
  }, [mode]);

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

  const sendPrompt = async (
    prompt: string,
    modelOverride?: ModelSelection,
    effectiveRunProfile?: EffectiveRunProfile,
  ) => {
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
            maxOutputTokens: effectiveRunProfile?.budgetEnvelope.maxOutputTokens ?? DEFAULT_MAX_OUTPUT_TOKENS,
            maxCostMicrounits: effectiveRunProfile?.budgetEnvelope.maxCostMicrounits ?? (selectedOption.isMock ? 0 : null),
            timeoutMs: effectiveRunProfile?.budgetEnvelope.timeoutMs ?? 120_000,
          },
          effectiveRunProfile: effectiveRunProfile ?? null,
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
                    effectiveRunProfile,
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
      effectiveRunProfile,
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
    void sendPrompt(prompt, retryModel, run.effectiveRunProfile);
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

  const importGenericFile = async (originalFileName: string, payload: number[]): Promise<GenericImportCommandResult> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会调用本地导入命令。");
    }
    if (!graph) {
      throw new Error("请先选择一个本地会话，再导入来源。");
    }
    const result = await kernelClient.importGenericFile({
      conversationId: graph.conversation.id,
      originalFileName,
      payload,
    });
    setImportSources((current) => [
      result.source,
      ...current.filter((source) => source.id !== result.source.id),
    ]);
    setNotice(result.duplicate ? "该来源内容已存在，本次复用原文指纹。" : null);
    return result;
  };

  const loadImportBundle = async (sourceId: string): Promise<ImportBundleQueryProjection> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会读取本地导入记录。");
    }
    return kernelClient.getImportBundle(sourceId);
  };

  const loadRawImportContent = async (sourceId: string): Promise<RawImportContentProjection> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会读取本地导入原文。");
    }
    try {
      return await kernelClient.getRawImportContent(sourceId);
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
  };

  const reloadKnowledge = async () => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会读取本地知识对象。");
    }
    if (!graph) {
      throw new Error("请先选择一个本地会话。");
    }
    setKnowledgeLoading(true);
    setKnowledgeError(null);
    setMarkdownProjectionsLoading(true);
    setMarkdownProjectionErrorsByEntityId(new Map());
    const entitiesRequest = kernelClient.listKnowledgeEntities(graph.conversation.id).then(
      (entities) => ({ entities, error: null as unknown }),
      (error: unknown) => ({ entities: [], error }),
    );
    const markdownProjectionsRequest = entitiesRequest.then((result) => (
      result.error
        ? loadMarkdownProjections([], kernelClient.listMarkdownProjections)
        : loadMarkdownProjections(
            result.entities.map((entity) => entity.id),
            kernelClient.listMarkdownProjections,
          )
    ));
    const [entitiesResult, relationsResult, markdownProjectionsResult] = await Promise.all([
      entitiesRequest,
      kernelClient.listKnowledgeRelations(graph.conversation.id).then(
        (relations) => ({ relations, error: null as unknown }),
        (error: unknown) => ({ relations: [], error }),
      ),
      markdownProjectionsRequest,
    ]);
    setKnowledgeEntities((current) => entitiesResult.error ? current : entitiesResult.entities);
    setKnowledgeRelations((current) => relationsResult.error ? current : relationsResult.relations);
    if (!entitiesResult.error) {
      setMarkdownProjectionsByEntityId(markdownProjectionsResult.projectionsByEntityId);
      setMarkdownProjectionErrorsByEntityId(new Map(
        [...markdownProjectionsResult.errorsByEntityId].map(([entityId, error]) => [
          entityId,
          safeErrorMessage(error),
        ]),
      ));
    }
    const nextError = entitiesResult.error ?? relationsResult.error;
    setKnowledgeError(nextError ? safeErrorMessage(nextError) : null);
    setKnowledgeLoading(false);
    setMarkdownProjectionsLoading(false);
    if (nextError) {
      const message = safeErrorMessage(nextError);
      setNotice(`知识对象读取失败：${message}`);
      throw new Error(message);
    }
    if (markdownProjectionsResult.errorsByEntityId.size) {
      setNotice(`知识对象已刷新，但 ${markdownProjectionsResult.errorsByEntityId.size} 个实体的 Markdown 投影读取失败。`);
    }
  };

  const importMarkdownEntityEdit = async (
    entityId: string,
  ): Promise<MarkdownEditCommandResult> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会读取 Vault Markdown 编辑。");
    }
    if (!graph) {
      throw new Error("请先选择一个本地会话。");
    }

    try {
      const result = await kernelClient.importMarkdownEntityEdit(
        graph.conversation.id,
        entityId,
      );
      setMarkdownProjectionsByEntityId((current) => {
        const next = new Map(current);
        next.set(
          entityId,
          prependMarkdownProjectionRevision(current.get(entityId), result.projection),
        );
        return next;
      });
      setMarkdownProjectionErrorsByEntityId((current) => {
        const next = new Map(current);
        next.delete(entityId);
        return next;
      });

      if (!result.changed) {
        setNotice("Vault Markdown 内容未变化，没有创建空修订。");
        return result;
      }

      setKnowledgeRetrievalByNodeId(new Map());
      setKnowledgeRetrievalLoadingNodeId(null);
      setKnowledgeRetrievalErrorByNodeId(new Map());
      try {
        setKnowledgeEntities(await kernelClient.listKnowledgeEntities(graph.conversation.id));
        setKnowledgeError(null);
        setNotice(`已同步 Vault 编辑：实体 revision ${result.projection.entityRevision}，投影 revision ${result.projection.projectionRevision}。`);
      } catch (error) {
        const message = safeErrorMessage(error);
        setKnowledgeError(message);
        setNotice(`Vault 编辑已同步，但实体清单刷新失败：${message}`);
      }
      return result;
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
  };

  const retrieveKnowledgeForNode = async (
    node: ConversationNode,
    query: string,
  ): Promise<CanvasKnowledgeRetrievalProjection> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会调用本地知识检索。");
    }
    if (!graph || graph.conversation.id !== node.conversationId) {
      throw new Error("当前节点不属于已选本地会话。");
    }
    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      throw new Error("检索问题不能为空。");
    }
    setKnowledgeRetrievalLoadingNodeId(node.id);
    setKnowledgeRetrievalErrorByNodeId((current) => {
      const next = new Map(current);
      next.delete(node.id);
      return next;
    });
    try {
      const retrieval: KnowledgeRetrievalProjection = await kernelClient.retrieveKnowledge(
        graph.conversation.id,
        trimmedQuery,
      );
      const projected = projectKnowledgeRetrieval(retrieval);
      setKnowledgeRetrievalByNodeId((current) => {
        const next = new Map(current);
        next.set(node.id, projected);
        return next;
      });
      return projected;
    } catch (error) {
      const message = safeErrorMessage(error);
      setKnowledgeRetrievalErrorByNodeId((current) => {
        const next = new Map(current);
        next.set(node.id, message);
        return next;
      });
      throw new Error(message);
    } finally {
      setKnowledgeRetrievalLoadingNodeId((current) => current === node.id ? null : current);
    }
  };

  const updateFocusFrameProjection = (query: ReturnType<typeof projectFocusFrameQuery>) => {
    if (!query) return;
    setFocusFrameQueryByNodeId((current) => upsertFocusFrameQueryByNodeId(current, query));
  };

  const reloadFocusFrames = async () => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会读取本地 FocusFrame。");
    }
    if (!graph) {
      throw new Error("请先选择一个本地会话。");
    }
    setFocusFrameQueryError(null);
    try {
      const queries = await kernelClient.listFocusFrames(graph.conversation.id);
      let indexedFocusFrames = new Map<string, CanvasFocusFrameQueryProjection>();
      for (const query of queries) {
        const projected = projectFocusFrameQuery(query);
        if (projected) indexedFocusFrames = upsertFocusFrameQueryByNodeId(indexedFocusFrames, projected);
      }
      setFocusFrameQueryByNodeId(indexedFocusFrames);
    } catch (error) {
      const message = safeErrorMessage(error);
      setFocusFrameQueryError(message);
      throw new Error(message);
    }
  };

  const loadFocusPromotionCandidates = useCallback(async (
    focusFrameId: string,
    expectedMemoryVersion: number,
  ): Promise<FocusPromotionCandidateSet | null> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会读取本地回流候选。");
    }
    try {
      return await kernelClient.getFocusPromotionCandidates(
        focusFrameId,
        expectedMemoryVersion,
      );
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
  }, [mode]);

  const decideFocusPromotion = async (
    input: FocusPromotionDecisionCommandInput,
  ): Promise<FocusPromotionDecisionProjection> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会提交本地回流决定。");
    }
    let projection: FocusPromotionDecisionProjection;
    try {
      projection = await kernelClient.decideFocusPromotion(input);
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
    setKnowledgeRetrievalByNodeId(new Map());
    setKnowledgeRetrievalLoadingNodeId(null);
    setKnowledgeRetrievalErrorByNodeId(new Map());
    const refreshResults = await Promise.allSettled([reloadKnowledge(), reloadFocusFrames()]);
    if (refreshResults.some((result) => result.status === "rejected")) {
      setNotice("回流决定已保存，但部分界面状态刷新失败；请使用刷新入口恢复，不要重复提交决定。");
    }
    return projection;
  };

  const loadFocusPromotionDecisions = useCallback(async (
    focusFrameId: string,
  ): Promise<FocusPromotionDecisionProjection[]> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会读取本地回流决定。");
    }
    try {
      return await kernelClient.listFocusPromotionDecisions(focusFrameId);
    } catch (error) {
      throw new Error(safeErrorMessage(error));
    }
  }, [mode]);

  const createFocusFrameForNode = async (
    node: ConversationNode,
    objective: string,
  ): Promise<CanvasFocusFrameQueryProjection> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会创建本地 FocusFrame。");
    }
    if (!graph || graph.conversation.id !== node.conversationId) {
      throw new Error("当前节点不属于已选本地会话。");
    }
    const trimmedObjective = objective.trim();
    if (!trimmedObjective) {
      throw new Error("FocusFrame 目标不能为空。");
    }
    const frame: FocusFrame = {
      contractVersion: "mindscape.focus.v1",
      id: `focus-${crypto.randomUUID()}`,
      conversationId: graph.conversation.id,
      parentNodeId: node.id,
      objective: trimmedObjective,
      activeWorkItem: null,
      contextPolicy: "branchFromNode",
      memoryScope: {
        branchKind: "task",
        // branchFromNode must explicitly seed the task scope with the
        // selected domain node; the kernel then expands that reference to
        // the corresponding conversation turn. Keep it out of includeRefs
        // because FocusFrame reference sets are mutually exclusive.
        inheritRefs: [node.id],
        localRefs: [],
        excludeRefs: [],
        promoteRefs: [],
      },
      includeRefs: [],
      excludeRefs: [],
      memoryVersion: 1,
      createdAt: new Date().toISOString(),
    };
    const created = await kernelClient.createFocusFrame(frame);
    const query = await kernelClient.getFocusFrameQuery(created.frame.id);
    const projected = projectFocusFrameQuery(query);
    if (!projected) throw new Error("FocusFrame 查询结果无法投影。");
    setFocusFrameQueryByNodeId((current) => upsertFocusFrameQueryByNodeId(current, projected));
    setNotice("已为当前节点创建 FocusFrame，等待内核编译上下文。");
    return projected;
  };

  const transitionFocusFrame = async (
    action: "close" | "reopen",
    query: CanvasFocusFrameQueryProjection,
  ): Promise<CanvasFocusFrameQueryProjection> => {
    if (mode !== "tauri") {
      throw new Error("浏览器预览不会改变本地 FocusFrame 生命周期。");
    }
    const input: FocusFrameLifecycleCommandInput = {
      focusFrameId: query.lifecycle.focusFrame.id,
      expectedRevision: query.lifecycle.revision,
      updatedAt: new Date().toISOString(),
    };
    const next = action === "close"
      ? await kernelClient.closeFocusFrame(input)
      : await kernelClient.reopenFocusFrame(input);
    const projected = projectFocusFrameQuery(next);
    if (!projected) throw new Error("FocusFrame 生命周期结果无法投影。");
    updateFocusFrameProjection(projected);
    setNotice(action === "close" ? "FocusFrame 已关闭。" : "FocusFrame 已重新打开。");
    return projected;
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
  const readingLayoutWidth = Math.max(620, Math.round(readingPreferences.readingWidthCh * 11.4));
  const readingDialogWidth = Math.max(600, readingLayoutWidth - 60);
  const readingParagraphSpacingPx = resolveReadingParagraphSpacingPx(
    readingPreferences.fontSizePx,
    readingPreferences.paragraphSpacingEm,
  );
  const readingStyle = {
    "--reading-font-size": `${readingPreferences.fontSizePx}px`,
    "--reading-line-height": readingPreferences.lineHeightValue,
    "--reading-max-width": `${readingPreferences.readingWidthCh}ch`,
    "--reading-layout-width": `${readingLayoutWidth}px`,
    "--reading-dialog-width": `${readingDialogWidth}px`,
    "--reading-letter-spacing": `${readingPreferences.letterSpacingEm}em`,
    // Resolve the em preference against the body size once. If the raw em token
    // reaches a heading, it scales against that heading's larger font size and
    // makes the same preference produce inconsistent visual gaps.
    "--reading-paragraph-spacing": `${readingParagraphSpacingPx}px`,
  } as CSSProperties;

  return (
    <div
      className="app-shell"
      data-reading-font={readingPreferences.font}
      data-reading-size={readingPreferences.fontSize}
      data-reading-line-height={readingPreferences.lineHeight}
      data-reading-width={readingPreferences.width}
      style={readingStyle}
    >
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
        importSources={importSources}
        importSourcesLoading={importSourcesLoading}
        knowledgeEntities={knowledgeEntities}
        knowledgeRelations={knowledgeRelations}
        knowledgeLoading={knowledgeLoading}
        knowledgeError={knowledgeError}
        markdownProjectionsByEntityId={markdownProjectionsByEntityId}
        markdownProjectionErrorsByEntityId={markdownProjectionErrorsByEntityId}
        markdownProjectionsLoading={markdownProjectionsLoading}
        onImportMarkdownEntityEdit={mode === "tauri" ? importMarkdownEntityEdit : undefined}
        onReloadKnowledge={mode === "tauri" ? reloadKnowledge : undefined}
        knowledgeRetrievalByNodeId={knowledgeRetrievalByNodeId}
        knowledgeRetrievalLoadingNodeId={knowledgeRetrievalLoadingNodeId}
        knowledgeRetrievalErrorByNodeId={knowledgeRetrievalErrorByNodeId}
        onRetrieveKnowledge={mode === "tauri" ? retrieveKnowledgeForNode : undefined}
        focusFrameQueryByNodeId={focusFrameQueryByNodeId}
        focusFrameQueryError={focusFrameQueryError}
        onLoadFocusPromotionCandidates={mode === "tauri" ? loadFocusPromotionCandidates : undefined}
        onDecideFocusPromotion={mode === "tauri" ? decideFocusPromotion : undefined}
        onLoadFocusPromotionDecisions={mode === "tauri" ? loadFocusPromotionDecisions : undefined}
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
        onSend={(prompt, effectiveRunProfile) => void sendPrompt(prompt, undefined, effectiveRunProfile)}
        onCancel={cancelRun}
        onRetry={retryRun}
        onSelectModel={setSelectedModel}
        onInspectContext={(node) => void inspectContext(node)}
        onOpenSettings={() => setSettingsOpen(true)}
        onImportGenericFile={mode === "tauri" ? importGenericFile : undefined}
        onLoadImportBundle={mode === "tauri" ? loadImportBundle : undefined}
        onLoadRawImportContent={mode === "tauri" ? loadRawImportContent : undefined}
        onCreateFocusFrame={mode === "tauri" ? createFocusFrameForNode : undefined}
        onTransitionFocusFrame={mode === "tauri" ? transitionFocusFrame : undefined}
        onReloadFocusFrames={mode === "tauri" ? reloadFocusFrames : undefined}
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
        readingPreferences={readingPreferences}
        readingPreferencesSessionOnly={sessionReadingPreferences !== null}
        onClose={() => setSettingsOpen(false)}
        onRefresh={refreshProviderSettings}
        onSelectModel={setSelectedModel}
        onSaveCredential={saveProviderCredential}
        onDeleteCredential={deleteProviderCredential}
        onTestConnection={testProviderConnection}
        onGetSemanticModelStatus={getSemanticModelStatus}
        onInstallSemanticModel={installSemanticModel}
        onReadingPreferencesChange={updateReadingPreferences}
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

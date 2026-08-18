import { useEffect, useRef, useState } from "react";
import {
  Bot,
  ChevronDown,
  CircleStop,
  Copy,
  CornerDownRight,
  FileUp,
  LayoutGrid,
  List,
  LoaderCircle,
  MessageSquarePlus,
  PanelLeft,
  RotateCcw,
  Send,
  Settings2,
  Sparkles,
  X,
} from "lucide-react";
import { presentProviderError, type ChatRunState } from "../app/chatRunState";
import type { ChatModelOption } from "../app/providerCatalog";
import type {
  BranchType,
  ContentBlock,
  ConversationGraph,
  ConversationNode,
  ModelRunProjection,
  ModelSelection,
} from "../domain";
import type { CanvasPoint, CanvasViewport } from "../canvas/graphProjection";
import { ConversationCanvas } from "./ConversationCanvas";

type ChatWorkspaceProps = {
  graph: ConversationGraph | null;
  modelRuns: readonly ModelRunProjection[];
  initialCanvasViewport: CanvasViewport | null;
  loading: boolean;
  sidebarOpen: boolean;
  runtimeLabel: string;
  selectedParentId: string | null;
  selectedBranchType: BranchType;
  viewMode: "canvas" | "chat";
  run: ChatRunState | null;
  runSubmitting: boolean;
  modelOptions: ChatModelOption[];
  selectedModel: ModelSelection | null;
  onToggleSidebar: () => void;
  onSelectParent: (nodeId: string) => void;
  onSelectBranch: (nodeId: string, branchType: BranchType) => void;
  onChangeViewMode: (mode: "canvas" | "chat") => void;
  onMoveNode: (nodeId: string, position: CanvasPoint) => void;
  onCanvasViewportChange: (conversationId: string, viewport: CanvasViewport) => void;
  onClearParent: () => void;
  onCreateConversation: () => void;
  onSend: (prompt: string) => void;
  onCancel: () => void;
  onRetry: () => void;
  onSelectModel: (selection: ModelSelection) => void;
  onInspectContext: (node: ConversationNode) => void;
  onOpenSettings: () => void;
};

function blocksToPlainText(blocks: ContentBlock[]) {
  return blocks
    .map((block) => {
      if (block.type === "text") return block.text;
      if (block.type === "code") return block.code;
      if (block.type === "link") return block.label ?? block.url;
      if (block.type === "attachmentRef") return `[附件：${block.displayName}]`;
      if (block.type === "toolCallRef") return `[工具调用：${block.toolRunId}]`;
      if (block.type === "toolResultRef") return `[工具结果：${block.toolRunId}]`;
      return `[暂不支持的内容：${block.originalType}]`;
    })
    .join("\n");
}

function NodeCard({
  node,
  selected,
  onContinue,
  onInspectContext,
}: {
  node: ConversationNode;
  selected: boolean;
  onContinue: () => void;
  onInspectContext: () => void;
}) {
  const [copied, setCopied] = useState(false);

  const copyAnswer = async () => {
    if (!node.assistantMessage) return;
    await navigator.clipboard.writeText(blocksToPlainText(node.assistantMessage.contentBlocks));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  return (
    <article className={`turn-card${selected ? " is-selected" : ""}`}>
      <div className="turn-question">
        <span className="turn-avatar is-user">你</span>
        <div>
          <span className="turn-meta">用户问题</span>
          <p>{blocksToPlainText(node.userMessage.contentBlocks)}</p>
        </div>
      </div>

      <div className="turn-answer">
        <span className="turn-avatar is-assistant"><Sparkles aria-hidden="true" /></span>
        <div className="answer-content">
          <div className="answer-heading">
            <span>{node.modelId ?? "等待模型"}</span>
            <small>{node.runState}</small>
          </div>
          {node.assistantMessage ? (
            <p className="answer-text">{blocksToPlainText(node.assistantMessage.contentBlocks)}</p>
          ) : (
            <p className="pending-answer"><LoaderCircle aria-hidden="true" />等待运行恢复</p>
          )}
          <div className="turn-actions">
            <button type="button" onClick={onContinue}>
              <CornerDownRight aria-hidden="true" />从这里继续
            </button>
            <button type="button" onClick={onInspectContext}>
              <Sparkles aria-hidden="true" />本轮上下文
            </button>
            <button type="button" onClick={copyAnswer} disabled={!node.assistantMessage}>
              <Copy aria-hidden="true" />{copied ? "已复制" : "复制"}
            </button>
          </div>
        </div>
      </div>
    </article>
  );
}

function ActiveRunCard({
  run,
  onCancel,
  onRetry,
  onOpenSettings,
}: {
  run: ChatRunState;
  onCancel: () => void;
  onRetry: () => void;
  onOpenSettings: () => void;
}) {
  const running = run.status === "starting" || run.status === "streaming";
  const errorPresentation = run.error ? presentProviderError(run.error) : null;
  const settingsAction =
    errorPresentation?.action === "openSettings" || errorPresentation?.action === "chooseModel";
  const retryAction =
    errorPresentation?.action === "retry" || (run.status === "failed" && run.error?.retryable);
  const statusLabel = run.cancelRequested
    ? "正在停止"
    : {
        starting: "正在启动",
        streaming: "正在生成",
        completed: "已完成",
        cancelled: "已停止",
        failed: "运行失败",
      }[run.status];
  return (
    <article className="turn-card is-running" aria-live="polite">
      <div className="turn-question">
        <span className="turn-avatar is-user">你</span>
        <div>
          <span className="turn-meta">刚刚发送</span>
          <p>{run.prompt}</p>
        </div>
      </div>
      <div className="turn-answer">
        <span className="turn-avatar is-assistant"><Bot aria-hidden="true" /></span>
        <div className="answer-content">
          <div className="answer-heading">
            <span>{run.modelId} · {run.providerId === "mock" ? "本地测试" : run.providerId}</span>
            <small>{statusLabel}</small>
          </div>
          {run.content ? <p className="answer-text is-streaming">{run.content}</p> : null}
          {run.status === "starting" ? <p className="pending-answer"><LoaderCircle aria-hidden="true" />正在准备上下文与运行请求</p> : null}
          {run.status === "cancelled" ? (
            <p className="run-message">
              已停止。{run.partialContentRetained ? "已收到的内容会保留在本次运行记录中。" : "本次没有保留部分内容。"}
            </p>
          ) : null}
          {run.status === "failed" ? (
            <div className="provider-error-panel" role="alert">
              <strong>{errorPresentation?.title ?? "模型运行失败"}</strong>
              <p>{errorPresentation?.guidance ?? "请重试；若持续出现，请检查模型配置。"}</p>
              {run.errorMessage ? <small>诊断信息：{run.errorMessage}</small> : null}
            </div>
          ) : null}
          {run.cancelErrorMessage ? (
            <p className="protocol-warning" role="alert">停止请求未生效：{run.cancelErrorMessage}</p>
          ) : null}
          {run.protocolWarning ? <p className="protocol-warning">事件协议提示：{run.protocolWarning}</p> : null}
          <div className="turn-actions">
            {running ? (
              <button type="button" onClick={onCancel} disabled={run.cancelRequested}>
                {run.cancelRequested ? <LoaderCircle className="spin" aria-hidden="true" /> : <CircleStop aria-hidden="true" />}
                {run.cancelRequested ? "正在停止" : "停止生成"}
              </button>
            ) : null}
            {run.status === "cancelled" || retryAction ? (
              <button type="button" onClick={onRetry}><RotateCcw aria-hidden="true" />重试</button>
            ) : null}
            {run.status === "failed" && settingsAction ? (
              <button type="button" onClick={onOpenSettings}>
                <Settings2 aria-hidden="true" />{errorPresentation?.actionLabel ?? "打开设置"}
              </button>
            ) : null}
          </div>
        </div>
      </div>
    </article>
  );
}

function Composer({
  parent,
  branchType,
  disabled,
  submitting,
  cancelRequested,
  modelOptions,
  selectedModel,
  onClearParent,
  onSelectModel,
  onOpenSettings,
  onSend,
  onCancel,
}: {
  parent: ConversationNode | null;
  branchType: BranchType;
  disabled: boolean;
  submitting: boolean;
  cancelRequested: boolean;
  modelOptions: ChatModelOption[];
  selectedModel: ModelSelection | null;
  onClearParent: () => void;
  onSelectModel: (selection: ModelSelection) => void;
  onOpenSettings: () => void;
  onSend: (prompt: string) => void;
  onCancel: () => void;
}) {
  const [prompt, setPrompt] = useState("");
  const [modelOpen, setModelOpen] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "auto";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 132)}px`;
  }, [prompt]);

  const submit = () => {
    const value = prompt.trim();
    if (!value || disabled) return;
    onSend(value);
    setPrompt("");
  };

  const branchLabel = {
    continues: "继续",
    deepens: "深入",
    diverges: "发散",
    reframes: "换角度",
    importedFrom: "导入来源",
  }[branchType];
  const selectedOption = modelOptions.find(
    (option) =>
      option.providerId === selectedModel?.providerId && option.modelId === selectedModel.modelId,
  );

  return (
    <div className="composer-wrap">
      {parent ? (
        <div className="continue-chip">
          <CornerDownRight aria-hidden="true" />
          <span>从“{parent.title}”{branchLabel}</span>
          <button type="button" onClick={onClearParent} aria-label="取消从当前节点继续"><X aria-hidden="true" /></button>
        </div>
      ) : (
        <div className="continue-chip is-root"><MessageSquarePlus aria-hidden="true" />创建新的根节点</div>
      )}

      <div className="composer">
        <div className="model-select-wrap">
          <button
            className="model-select"
            type="button"
            onClick={() => setModelOpen((value) => !value)}
            aria-expanded={modelOpen}
          >
            <Bot aria-hidden="true" />
            <span>
              <strong>{selectedOption?.modelLabel ?? "选择可用模型"}</strong>
              <small>
                {selectedOption
                  ? `${selectedOption.providerLabel} · ${selectedOption.availabilityLabel}`
                  : "需要先配置 Provider"}
              </small>
            </span>
            <ChevronDown aria-hidden="true" />
          </button>
          {modelOpen ? (
            <div className="model-menu">
              <span className="menu-label">选择下一次运行使用的模型</span>
              {modelOptions.map((model) => (
                <button
                  key={`${model.providerId}:${model.modelId}`}
                  type="button"
                  disabled={!model.available}
                  onClick={() => {
                    onSelectModel({ providerId: model.providerId, modelId: model.modelId });
                    setModelOpen(false);
                  }}
                >
                  <span><strong>{model.modelLabel}</strong><small>{model.providerLabel}</small></span>
                  <small>{model.availabilityLabel}</small>
                </button>
              ))}
              {modelOptions.length === 0 ? (
                <button type="button" onClick={onOpenSettings}>
                  <span><strong>尚无已注册模型</strong><small>打开设置检查 Provider</small></span>
                </button>
              ) : null}
            </div>
          ) : null}
        </div>

        <textarea
          ref={textareaRef}
          value={prompt}
          rows={1}
          disabled={disabled}
          placeholder={
            cancelRequested
              ? "正在停止当前生成…"
              : submitting
                ? "模型正在生成，可点击右侧停止按钮…"
                : disabled
                  ? "等待当前运行结束…"
                  : "输入现在想探索的问题…"
          }
          onChange={(event) => setPrompt(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              submit();
            }
          }}
        />
        <button
          className={`send-button${disabled ? " is-stop" : ""}`}
          type="button"
          onClick={disabled ? onCancel : submit}
          disabled={disabled ? cancelRequested : !selectedOption?.available || !prompt.trim()}
          aria-label={disabled ? (cancelRequested ? "正在停止生成" : "停止生成") : "发送消息"}
        >
          {disabled
            ? cancelRequested
              ? <LoaderCircle className="spin" aria-hidden="true" />
              : <CircleStop aria-hidden="true" />
            : <Send aria-hidden="true" />}
        </button>
      </div>
      <div className="composer-hint">
        Enter 发送 · Shift + Enter 换行 · Mock 与真实 API 会明确区分，模型选择只影响下一次运行
      </div>
    </div>
  );
}

export function ChatWorkspace({
  graph,
  modelRuns,
  initialCanvasViewport,
  loading,
  sidebarOpen,
  runtimeLabel,
  selectedParentId,
  selectedBranchType,
  viewMode,
  run,
  runSubmitting,
  modelOptions,
  selectedModel,
  onToggleSidebar,
  onSelectParent,
  onSelectBranch,
  onChangeViewMode,
  onMoveNode,
  onCanvasViewportChange,
  onClearParent,
  onCreateConversation,
  onSend,
  onCancel,
  onRetry,
  onSelectModel,
  onInspectContext,
  onOpenSettings,
}: ChatWorkspaceProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const canvasViewportsRef = useRef(new Map<string, CanvasViewport>());
  const parent = graph?.nodes.find((node) => node.id === selectedParentId) ?? null;
  const running = run
    ? run.status === "starting" || run.status === "streaming"
    : runSubmitting;

  useEffect(() => {
    if (!run) return;
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [run?.content, run?.status]);

  return (
    <section className="chat-workspace">
      <header className="workspace-topbar">
        <div className="topbar-title">
          {!sidebarOpen ? (
            <button className="icon-button" type="button" onClick={onToggleSidebar} aria-label="展开会话侧栏"><PanelLeft aria-hidden="true" /></button>
          ) : null}
          <div>
            <span className="eyebrow">CONVERSATION</span>
            <h1>{graph?.conversation.title ?? "MindScape 工作区"}</h1>
          </div>
        </div>
        <div className="topbar-actions">
          <div className="workspace-mode-switch" aria-label="工作区视图">
            <button
              className={viewMode === "canvas" ? "is-active" : ""}
              type="button"
              onClick={() => onChangeViewMode("canvas")}
              aria-pressed={viewMode === "canvas"}
            >
              <LayoutGrid aria-hidden="true" /><span>画布</span>
            </button>
            <button
              className={viewMode === "chat" ? "is-active" : ""}
              type="button"
              onClick={() => onChangeViewMode("chat")}
              aria-pressed={viewMode === "chat"}
            >
              <List aria-hidden="true" /><span>阅读</span>
            </button>
          </div>
          <span className="runtime-pill"><span aria-hidden="true" />{runtimeLabel}</span>
          <button className="icon-button" type="button" onClick={onOpenSettings} aria-label="打开模型设置"><Settings2 aria-hidden="true" /></button>
        </div>
      </header>

      <div className="chat-stage">
        {viewMode === "chat" ? <div className="turn-scroll" ref={scrollRef}>
          {loading ? <div className="center-state"><LoaderCircle className="spin" aria-hidden="true" /><p>正在读取本地会话图…</p></div> : null}
          {!loading && !graph ? (
            <div className="welcome-state">
              <span className="welcome-mark" aria-hidden="true"><Sparkles /></span>
              <span className="eyebrow">START WITH INTENT</span>
              <h2>今天想继续探索什么？</h2>
              <p>直接开始一个问题，或导入已有 AI 会话。无需先选择复杂模式。</p>
              <div className="welcome-actions">
                <button className="primary-action" type="button" onClick={onCreateConversation}><MessageSquarePlus aria-hidden="true" />开始提问</button>
                <button className="secondary-action" type="button" disabled title="等待导入模块接入"><FileUp aria-hidden="true" />导入 AI 会话 <small>待接入</small></button>
              </div>
            </div>
          ) : null}
          {!loading && graph && graph.nodes.length === 0 ? (
            <div className="conversation-empty">
              <span className="eyebrow">EMPTY CONVERSATION</span>
              <h2>从第一个问题开始</h2>
              <p>回答会成为会话图中的根节点，之后可以从任意节点继续。</p>
            </div>
          ) : null}
          {graph?.nodes.map((node) => (
            <NodeCard
              key={node.id}
              node={node}
              selected={node.id === selectedParentId}
              onContinue={() => onSelectParent(node.id)}
              onInspectContext={() => onInspectContext(node)}
            />
          ))}
          {run ? (
            <ActiveRunCard
              run={run}
              onCancel={onCancel}
              onRetry={onRetry}
              onOpenSettings={onOpenSettings}
            />
          ) : null}
        </div> : null}
        {viewMode === "canvas" && !loading && graph ? (
          <ConversationCanvas
            graph={graph}
            modelRuns={modelRuns}
            selectedNodeId={selectedParentId}
            run={run}
            onSelectNode={onSelectParent}
            onSelectBranch={onSelectBranch}
            onInspectContext={onInspectContext}
            onMoveNode={onMoveNode}
            initialViewport={
              canvasViewportsRef.current.get(graph.conversation.id) ??
              initialCanvasViewport ??
              undefined
            }
            onViewportChange={(viewport) => {
              canvasViewportsRef.current.set(graph.conversation.id, viewport);
              onCanvasViewportChange(graph.conversation.id, viewport);
            }}
          />
        ) : null}
        {viewMode === "canvas" && loading ? (
          <div className="center-state"><LoaderCircle className="spin" aria-hidden="true" /><p>正在读取本地会话图…</p></div>
        ) : null}
        {viewMode === "canvas" && !loading && !graph ? (
          <div className="welcome-state">
            <span className="welcome-mark" aria-hidden="true"><Sparkles /></span>
            <span className="eyebrow">START WITH INTENT</span>
            <h2>今天想继续探索什么？</h2>
            <p>先创建一段会话，然后从画布上的任意节点继续、深入、发散或换一个角度。</p>
            <div className="welcome-actions">
              <button className="primary-action" type="button" onClick={onCreateConversation}><MessageSquarePlus aria-hidden="true" />开始提问</button>
              <button className="secondary-action" type="button" disabled title="等待导入模块接入"><FileUp aria-hidden="true" />导入 AI 会话 <small>待接入</small></button>
            </div>
          </div>
        ) : null}
        {graph ? (
          <Composer
            parent={parent}
            branchType={selectedBranchType}
            disabled={Boolean(running)}
            submitting={runSubmitting}
            cancelRequested={run?.cancelRequested ?? false}
            modelOptions={modelOptions}
            selectedModel={selectedModel}
            onClearParent={onClearParent}
            onSelectModel={onSelectModel}
            onOpenSettings={onOpenSettings}
            onSend={onSend}
            onCancel={onCancel}
          />
        ) : null}
      </div>
    </section>
  );
}

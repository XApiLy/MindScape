import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  SlidersHorizontal,
  Sparkles,
  X,
} from "lucide-react";
import { presentProviderError, type ChatRunState } from "../app/chatRunState";
import { shouldFollowLatest } from "../app/chatScrollFollow";
import { projectChatTimeline } from "../app/chatTimeline";
import { buildImportKnowledgeProposalTargets } from "../app/importKnowledgeProposalTargets";
import { blocksToMarkdown } from "../app/markdownContent";
import { presentMissingNodeAnswer } from "../app/nodeAnswerPresentation";
import type { ChatModelOption } from "../app/providerCatalog";
import {
  composeEffectiveRunProfile,
  contextPolicyForComposer,
  createDefaultRunProfileDraft,
} from "../app/runProfileControls";
import type {
  BranchType,
  ContentBlock,
  ConversationGraph,
  ConversationNode,
  EffectiveRunProfile,
  FocusPromotionCandidateSet,
  FocusPromotionCandidateGenerationCommandInput,
  FocusPromotionCandidateGenerationProjection,
  FocusPromotionDecisionCommandInput,
  FocusPromotionDecisionProjection,
  GenericImportCommandResult,
  ImportBundleQueryProjection,
  ImportKnowledgeProposalBatchProjection,
  ImportKnowledgeProposalDiscoveryProjection,
  ImportKnowledgeProposalDiscoveryQuery,
  ImportKnowledgeProposalRequestInput,
  ImportKnowledgeProposalReviewCommandInput,
  ImportKnowledgeProposalReviewProjection,
  ImportSource,
  KnowledgeEntity,
  KnowledgeRelation,
  MarkdownEditCommandResult,
  MarkdownProjection,
  ModelRunProjection,
  ModelSelection,
  RawImportContentProjection,
} from "../domain";
import type { CanvasPoint, CanvasViewport } from "../canvas/graphProjection";
import type {
  CanvasFocusFrameQueryProjection,
  CanvasKnowledgeRetrievalProjection,
} from "../canvas/canvasM2Projection";
import { ConversationCanvas } from "./ConversationCanvas";
import { renderedMarkdownText, SafeMarkdown } from "./SafeMarkdown";

const ImportIntakeDialog = lazy(() => import("./ImportIntakeDialog").then((module) => ({
  default: module.ImportIntakeDialog,
})));

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
  onSend: (prompt: string, effectiveRunProfile: EffectiveRunProfile) => void;
  onCancel: () => void;
  onRetry: () => void;
  onSelectModel: (selection: ModelSelection) => void;
  onInspectContext: (node: ConversationNode) => void;
  onOpenSettings: () => void;
  onImportGenericFile?: (
    originalFileName: string,
    payload: number[],
  ) => Promise<GenericImportCommandResult>;
  importSources: readonly ImportSource[];
  importSourcesLoading: boolean;
  knowledgeEntities: readonly KnowledgeEntity[];
  knowledgeRelations: readonly KnowledgeRelation[];
  knowledgeLoading: boolean;
  knowledgeError: string | null;
  markdownProjectionsByEntityId: ReadonlyMap<string, readonly MarkdownProjection[]>;
  markdownProjectionErrorsByEntityId: ReadonlyMap<string, string>;
  markdownProjectionsLoading: boolean;
  onImportMarkdownEntityEdit?: (entityId: string) => Promise<MarkdownEditCommandResult>;
  onReloadKnowledge?: () => Promise<void>;
  knowledgeRetrievalByNodeId: ReadonlyMap<string, CanvasKnowledgeRetrievalProjection>;
  knowledgeRetrievalLoadingNodeId: string | null;
  knowledgeRetrievalErrorByNodeId: ReadonlyMap<string, string>;
  onRetrieveKnowledge?: (
    node: ConversationNode,
    query: string,
  ) => Promise<CanvasKnowledgeRetrievalProjection>;
  onLoadImportBundle?: (sourceId: string) => Promise<ImportBundleQueryProjection>;
  onLoadRawImportContent?: (sourceId: string) => Promise<RawImportContentProjection>;
  onRequestImportKnowledgeProposals?: (
    input: ImportKnowledgeProposalRequestInput,
  ) => Promise<ImportKnowledgeProposalBatchProjection>;
  onDiscoverImportKnowledgeProposals?: (
    query: ImportKnowledgeProposalDiscoveryQuery,
  ) => Promise<ImportKnowledgeProposalDiscoveryProjection>;
  onReviewImportKnowledgeProposal?: (
    input: ImportKnowledgeProposalReviewCommandInput,
  ) => Promise<ImportKnowledgeProposalReviewProjection>;
  onListImportKnowledgeProposalReviews?: (
    requestId: string,
  ) => Promise<ImportKnowledgeProposalReviewProjection[]>;
  onCreateFocusFrame?: (
    node: ConversationNode,
    objective: string,
  ) => Promise<CanvasFocusFrameQueryProjection>;
  onTransitionFocusFrame?: (
    action: "close" | "reopen",
    query: CanvasFocusFrameQueryProjection,
  ) => Promise<CanvasFocusFrameQueryProjection>;
  onGenerateFocusPromotionCandidates?: (
    input: FocusPromotionCandidateGenerationCommandInput,
  ) => Promise<FocusPromotionCandidateGenerationProjection>;
  focusFrameQueryByNodeId: ReadonlyMap<string, CanvasFocusFrameQueryProjection>;
  focusFrameQueryError: string | null;
  onLoadFocusPromotionCandidates?: (
    focusFrameId: string,
    expectedMemoryVersion: number,
  ) => Promise<FocusPromotionCandidateSet | null>;
  onDecideFocusPromotion?: (
    input: FocusPromotionDecisionCommandInput,
  ) => Promise<FocusPromotionDecisionProjection>;
  onLoadFocusPromotionDecisions?: (
    focusFrameId: string,
  ) => Promise<FocusPromotionDecisionProjection[]>;
  onReloadFocusFrames?: () => Promise<void>;
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
  const [copiedMode, setCopiedMode] = useState<"readable" | "markdown" | null>(null);
  const answerSurfaceRef = useRef<HTMLDivElement>(null);
  const missingAnswer = presentMissingNodeAnswer(node.runState);
  const answerMarkdown = node.assistantMessage
    ? blocksToMarkdown(node.assistantMessage.contentBlocks)
    : "";

  const markCopied = (mode: "readable" | "markdown") => {
    setCopiedMode(mode);
    window.setTimeout(() => setCopiedMode(null), 1200);
  };

  const copyReadableAnswer = async () => {
    const readableText = renderedMarkdownText(answerSurfaceRef.current);
    if (!readableText) return;
    await navigator.clipboard.writeText(readableText);
    markCopied("readable");
  };

  const copyMarkdownAnswer = async () => {
    if (!answerMarkdown) return;
    await navigator.clipboard.writeText(answerMarkdown);
    markCopied("markdown");
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
          {/*
            UI-HANDOFF-06
            位置：阅读视图已落库回答正文
            数据：Message.contentBlocks 只经 blocksToMarkdown 派生；renderer DOM 不写回领域对象
            状态：completed/cancelled/failed 继续由节点运行态表达，部分回答仍可安全阅读
            安全边界：不执行 raw HTML，不加载远程图片，不允许危险 URL；复制动作明确复制 raw Markdown
            可替换范围：员工06可调整排版、密度和视觉，不可改变 raw/rendered 分层与安全边界
          */}
          {node.assistantMessage ? (
            <SafeMarkdown markdown={answerMarkdown} className="answer-text" contentRef={answerSurfaceRef} />
          ) : (
            <p className="pending-answer">
              {missingAnswer.showSpinner ? <LoaderCircle aria-hidden="true" /> : null}
              {missingAnswer.message}
            </p>
          )}
          <div className="turn-actions">
            <button type="button" onClick={onContinue}>
              <CornerDownRight aria-hidden="true" />从这里继续
            </button>
            <button type="button" onClick={onInspectContext}>
              <Sparkles aria-hidden="true" />本轮上下文
            </button>
            <button type="button" onClick={copyReadableAnswer} disabled={!answerMarkdown}>
              <Copy aria-hidden="true" />{copiedMode === "readable" ? "已复制正文" : "复制正文"}
            </button>
            <button type="button" onClick={copyMarkdownAnswer} disabled={!answerMarkdown}>
              <Copy aria-hidden="true" />{copiedMode === "markdown" ? "已复制 Markdown" : "复制 Markdown"}
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
          {/*
            UI-HANDOFF-06
            位置：阅读视图实时运行正文
            数据：run.content 是不可变事件流的当前 raw Markdown 投影；仅交给共享安全 renderer
            状态：starting/streaming 使用渐进可读渲染；cancelled/failed 保留既有终态与部分内容
            可替换范围：员工06可调整流式视觉，不可引入 HTML 执行、远程图片或每 token 动效
          */}
          {run.content ? (
            <SafeMarkdown markdown={run.content} streaming={running} className="answer-text" />
          ) : null}
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
  onSend: (prompt: string, effectiveRunProfile: EffectiveRunProfile) => void;
  onCancel: () => void;
}) {
  const [prompt, setPrompt] = useState("");
  const [modelOpen, setModelOpen] = useState(false);
  const [profileOpen, setProfileOpen] = useState(false);
  const [profileDraft, setProfileDraft] = useState(createDefaultRunProfileDraft);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "auto";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 132)}px`;
  }, [prompt]);

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
  const contextPolicy = contextPolicyForComposer(parent?.branchType ?? null);
  const runProfile = useMemo(() => selectedOption
    ? composeEffectiveRunProfile({
        selection: { providerId: selectedOption.providerId, modelId: selectedOption.modelId },
        capabilities: selectedOption.capabilities,
        draft: profileDraft,
        contextPolicy,
        isMock: selectedOption.isMock,
      })
    : null, [contextPolicy, profileDraft, selectedOption]);
  const reasoningLabel = {
    off: "思考关闭",
    standard: "Standard / high",
    deep: "Deep / max",
    custom: `Custom / ${profileDraft.customReasoningEffort}`,
  }[profileDraft.reasoningMode];
  const contextPolicyLabel = {
    continueCurrent: "继续当前问题",
    focusNew: "聚焦新问题",
    branchFromNode: "从节点分支",
    continueImportedRaw: "原样续接",
  }[contextPolicy];

  const submit = () => {
    const value = prompt.trim();
    if (!value || disabled || !runProfile || runProfile.issues.length > 0) return;
    onSend(value, runProfile.profile);
    setPrompt("");
  };

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

      {/*
        UI-HANDOFF-06
        位置：Chat 编辑区上方的“下一次运行档案”入口与展开面板
        用途：让用户在发送前确认 reasoning、生成参数、上下文策略和禁用的工具权限
        数据/IPC：本地 profileDraft + providerCatalog 能力快照；提交时随 startModelRun.effectiveRunProfile 进入真实 IPC
        状态：正常显示当前档案；加载由父级发送状态体现；空白/错误由 profile issues 阻止发送并显示告警；停止/恢复沿用现有运行卡片和重试状态
        交互约束：按钮、表单和折叠面板保持键盘可达、aria-expanded/aria-pressed、冲突时禁用发送，不回显或接触 Key
        可替换范围：员工06可替换面板布局、视觉、动效和字段分组
        不可改变：EffectiveRunProfile 字段、能力门禁、测试选择器、IPC 名称、错误语义和工具禁用边界
      */}
      <div className="run-profile-control">
        <button
          className="run-profile-trigger"
          type="button"
          onClick={() => setProfileOpen((value) => !value)}
          aria-expanded={profileOpen}
          aria-controls="run-profile-panel"
        >
          <SlidersHorizontal aria-hidden="true" />
          <span><strong>下一次运行档案</strong><small>{reasoningLabel} · {profileDraft.maxOutputTokens} tokens · {contextPolicyLabel}</small></span>
          <ChevronDown aria-hidden="true" />
        </button>
        {runProfile?.issues.length ? <span className="run-profile-alert" role="alert">{runProfile.issues[0].message}</span> : null}
      </div>

      {profileOpen ? (
        <section className="run-profile-panel" id="run-profile-panel" aria-label="下一次运行档案设置">
          <header><div><span className="eyebrow">EFFECTIVE RUN PROFILE</span><strong>仅影响下一次运行</strong></div><small>旧运行档案不会被当前设置改写</small></header>
          <div className="run-profile-reasoning" role="group" aria-label="思考模式">
            {(["off", "standard", "deep", "custom"] as const).map((mode) => {
              const available = mode === "off"
                || (mode === "standard" && selectedOption?.capabilities.reasoningModes.includes("high"))
                || (mode === "deep" && selectedOption?.capabilities.reasoningModes.includes("max"))
                || (mode === "custom" && Boolean(selectedOption?.capabilities.supportsReasoning));
              const label = { off: "关闭", standard: "Standard", deep: "Deep", custom: "Custom" }[mode];
              return (
                <button
                  type="button"
                  key={mode}
                  className={profileDraft.reasoningMode === mode ? "is-active" : ""}
                  disabled={!available}
                  aria-pressed={profileDraft.reasoningMode === mode}
                  onClick={() => setProfileDraft((current) => ({ ...current, reasoningMode: mode }))}
                >{label}</button>
              );
            })}
          </div>
          {profileDraft.reasoningMode === "custom" ? (
            <label className="run-profile-field"><span>厂商思考档位</span><select value={profileDraft.customReasoningEffort} onChange={(event) => setProfileDraft((current) => ({ ...current, customReasoningEffort: event.target.value as "high" | "max" }))}><option value="high">high</option><option value="max">max</option></select></label>
          ) : null}
          <div className="run-profile-grid">
            <label className="run-profile-field"><span>最大输出 Tokens</span><input type="number" min="1" step="1" value={profileDraft.maxOutputTokens} onChange={(event) => setProfileDraft((current) => ({ ...current, maxOutputTokens: Number(event.target.value) }))} /></label>
            <label className="run-profile-field"><span>超时（秒）</span><input type="number" min="1" step="1" value={profileDraft.timeoutMs / 1000} onChange={(event) => setProfileDraft((current) => ({ ...current, timeoutMs: Number(event.target.value) * 1000 }))} /></label>
            <label className="run-profile-field"><span>temperature（可留空）</span><input type="number" min="0" max="2" step="0.1" value={profileDraft.temperature ?? ""} disabled={selectedOption?.capabilities.generationParameters.temperature === "unsupported"} onChange={(event) => setProfileDraft((current) => ({ ...current, temperature: event.target.value === "" ? null : Number(event.target.value) }))} /></label>
            <label className="run-profile-field"><span>top_p（可留空）</span><input type="number" min="0" max="1" step="0.1" value={profileDraft.topP ?? ""} disabled={selectedOption?.capabilities.generationParameters.topP === "unsupported"} onChange={(event) => setProfileDraft((current) => ({ ...current, topP: event.target.value === "" ? null : Number(event.target.value) }))} /></label>
            <label className="run-profile-field"><span>响应格式</span><select value={profileDraft.responseFormat} onChange={(event) => setProfileDraft((current) => ({ ...current, responseFormat: event.target.value as "text" | "json_object" }))}><option value="text">文本</option><option value="json_object" disabled={!selectedOption?.capabilities.structuredOutput}>JSON object</option></select></label>
          </div>
          <div className="run-profile-facts">
            <span><small>上下文策略</small><strong>{contextPolicyLabel}</strong></span>
            <span><small>工具权限</small><strong>禁用</strong></span>
            <span><small>能力目录</small><strong>provider-catalog-v1</strong></span>
          </div>
          {runProfile?.issues.length ? <ul className="run-profile-issues">{runProfile.issues.map((issue) => <li key={issue.code}>{issue.message}</li>)}</ul> : null}
        </section>
      ) : null}

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
          disabled={disabled
            ? cancelRequested
            : !selectedOption?.available || !prompt.trim() || !runProfile || runProfile.issues.length > 0}
          data-profile-ready={runProfile && runProfile.issues.length === 0 ? "true" : "false"}
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
  onImportGenericFile,
  importSources,
  importSourcesLoading,
  knowledgeEntities,
  knowledgeRelations,
  knowledgeLoading,
  knowledgeError,
  markdownProjectionsByEntityId,
  markdownProjectionErrorsByEntityId,
  markdownProjectionsLoading,
  onImportMarkdownEntityEdit,
  onReloadKnowledge,
  knowledgeRetrievalByNodeId,
  knowledgeRetrievalLoadingNodeId,
  knowledgeRetrievalErrorByNodeId,
  onRetrieveKnowledge,
  onLoadImportBundle,
  onLoadRawImportContent,
  onRequestImportKnowledgeProposals,
  onDiscoverImportKnowledgeProposals,
  onReviewImportKnowledgeProposal,
  onListImportKnowledgeProposalReviews,
  onCreateFocusFrame,
  onTransitionFocusFrame,
  onGenerateFocusPromotionCandidates,
  focusFrameQueryByNodeId,
  focusFrameQueryError,
  onLoadFocusPromotionCandidates,
  onDecideFocusPromotion,
  onLoadFocusPromotionDecisions,
  onReloadFocusFrames,
}: ChatWorkspaceProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const canvasViewportsRef = useRef(new Map<string, CanvasViewport>());
  const [importOpen, setImportOpen] = useState(false);
  const [followLatest, setFollowLatest] = useState(true);
  const parent = graph?.nodes.find((node) => node.id === selectedParentId) ?? null;
  const timeline = projectChatTimeline(graph?.nodes ?? [], run);
  const running = run
    ? run.status === "starting" || run.status === "streaming"
    : runSubmitting;

  const scrollToLatest = useCallback(() => {
    const surface = scrollRef.current;
    if (!surface) return;
    surface.scrollTop = surface.scrollHeight;
    setFollowLatest(true);
  }, []);

  useEffect(() => {
    if (!run?.id) return;
    setFollowLatest(true);
  }, [run?.id]);

  useEffect(() => {
    if (!run || !followLatest || viewMode !== "chat") return;
    const frame = window.requestAnimationFrame(scrollToLatest);
    return () => window.cancelAnimationFrame(frame);
  }, [followLatest, run?.content, run?.status, scrollToLatest, viewMode]);

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
          <button className="icon-button" type="button" onClick={() => setImportOpen(true)} aria-label="导入已有会话"><FileUp aria-hidden="true" /></button>
          <button className="icon-button" type="button" onClick={onOpenSettings} aria-label="打开模型设置"><Settings2 aria-hidden="true" /></button>
        </div>
      </header>

      <div className="chat-stage">
        {viewMode === "chat" ? <div
          className="turn-scroll"
          ref={scrollRef}
          onScroll={(event) => {
            const next = shouldFollowLatest(event.currentTarget);
            setFollowLatest((current) => current === next ? current : next);
          }}
        >
          {loading ? <div className="center-state"><LoaderCircle className="spin" aria-hidden="true" /><p>正在读取本地会话图…</p></div> : null}
          {!loading && !graph ? (
            <div className="welcome-state">
              <span className="welcome-mark" aria-hidden="true"><Sparkles /></span>
              <span className="eyebrow">START WITH INTENT</span>
              <h2>今天想继续探索什么？</h2>
              <p>直接开始一个问题，或导入已有 AI 会话。无需先选择复杂模式。</p>
              <div className="welcome-actions">
                <button className="primary-action" type="button" onClick={onCreateConversation}><MessageSquarePlus aria-hidden="true" />开始提问</button>
                <button className="secondary-action" type="button" onClick={() => setImportOpen(true)}><FileUp aria-hidden="true" />导入 AI 会话</button>
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
          {timeline.map((entry) => entry.kind === "run" ? (
            <ActiveRunCard
              key={`run-${entry.run.nodeId}`}
              run={entry.run}
              onCancel={onCancel}
              onRetry={onRetry}
              onOpenSettings={onOpenSettings}
            />
          ) : (
            <NodeCard
              key={entry.node.id}
              node={entry.node}
              selected={entry.node.id === selectedParentId}
              onContinue={() => onSelectParent(entry.node.id)}
              onInspectContext={() => onInspectContext(entry.node)}
            />
          ))}
        </div> : null}
        {/*
          UI-HANDOFF-06
          位置：Chat 阅读视图流式正文上方的阅读锁定恢复入口
          用途：用户离开底部后停止自动跟随；仅显式点击“定位到最新”恢复
          数据：只读取 scrollTop/clientHeight/scrollHeight，不修改 raw stream、运行终态或会话图
          状态：近底部自动跟随；上滑锁定；流式增长保持当前位置；运行结束沿用当前锚点
          可替换范围：员工06可替换按钮位置、视觉和非必要动效；不可恢复强制下拉或隐藏锁定状态
        */}
        {viewMode === "chat" && run && !followLatest ? (
          <button className="locate-latest-button" type="button" onClick={scrollToLatest}>
            <ChevronDown aria-hidden="true" />定位到最新
          </button>
        ) : null}
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
            focusFrameQueryByNodeId={focusFrameQueryByNodeId}
            focusFrameQueryError={focusFrameQueryError}
            onLoadFocusPromotionCandidates={onLoadFocusPromotionCandidates}
            onDecideFocusPromotion={onDecideFocusPromotion}
            onLoadFocusPromotionDecisions={onLoadFocusPromotionDecisions}
            onCreateFocusFrame={onCreateFocusFrame}
            onTransitionFocusFrame={onTransitionFocusFrame}
            onGenerateFocusPromotionCandidates={onGenerateFocusPromotionCandidates}
            onReloadFocusFrames={onReloadFocusFrames}
            knowledgeEntities={knowledgeEntities}
            knowledgeRelations={knowledgeRelations}
            knowledgeLoading={knowledgeLoading}
            knowledgeError={knowledgeError}
            markdownProjectionsByEntityId={markdownProjectionsByEntityId}
            markdownProjectionErrorsByEntityId={markdownProjectionErrorsByEntityId}
            markdownProjectionsLoading={markdownProjectionsLoading}
            onImportMarkdownEntityEdit={onImportMarkdownEntityEdit}
            onReloadKnowledge={onReloadKnowledge}
            knowledgeRetrievalByNodeId={knowledgeRetrievalByNodeId}
            knowledgeRetrievalLoadingNodeId={knowledgeRetrievalLoadingNodeId}
            knowledgeRetrievalErrorByNodeId={knowledgeRetrievalErrorByNodeId}
            onRetrieveKnowledge={onRetrieveKnowledge}
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
              <button className="secondary-action" type="button" onClick={() => setImportOpen(true)}><FileUp aria-hidden="true" />导入 AI 会话</button>
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
      {importOpen ? (
        <Suspense fallback={(
          <div className="modal-backdrop" role="presentation">
            <section className="import-dialog" role="dialog" aria-modal="true" aria-label="正在打开导入面板">
              <p className="import-history-empty" role="status"><LoaderCircle className="spin" aria-hidden="true" />正在准备本地导入面板…</p>
            </section>
          </div>
        )}>
          <ImportIntakeDialog
            open
            onClose={() => setImportOpen(false)}
            onImportGenericFile={onImportGenericFile}
            importSources={importSources}
            importSourcesLoading={importSourcesLoading}
            onLoadImportBundle={onLoadImportBundle}
            onLoadRawImportContent={onLoadRawImportContent}
            proposalTargetOptions={graph
              ? buildImportKnowledgeProposalTargets(
                  graph.conversation,
                  focusFrameQueryByNodeId.values(),
                )
              : []}
            onRequestImportKnowledgeProposals={onRequestImportKnowledgeProposals}
            onDiscoverImportKnowledgeProposals={onDiscoverImportKnowledgeProposals}
            onReviewImportKnowledgeProposal={onReviewImportKnowledgeProposal}
            onListImportKnowledgeProposalReviews={onListImportKnowledgeProposalReviews}
          />
        </Suspense>
      ) : null}
    </section>
  );
}

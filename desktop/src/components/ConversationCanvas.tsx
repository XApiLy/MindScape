import {
  ArrowDown,
  ArrowUpRight,
  BrainCircuit,
  BookOpen,
  Check,
  Copy,
  Crosshair,
  Eye,
  FileText,
  GitBranch,
  LocateFixed,
  LoaderCircle,
  Maximize2,
  Minus,
  Move,
  Plus,
  RefreshCcw,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent,
} from "react";
import type {
  BranchType,
  ConversationGraph,
  ConversationNode,
  FocusPromotionCandidateSet,
  FocusPromotionCandidateGenerationCommandInput,
  FocusPromotionCandidateGenerationProjection,
  FocusPromotionDecisionAction,
  FocusPromotionDecisionCommandInput,
  FocusPromotionDecisionProjection,
  KnowledgeEntity,
  KnowledgeRelation,
  MarkdownEditCommandResult,
  MarkdownProjection,
  ModelRunProjection,
  ProviderError,
  RunState,
} from "../domain";
import {
  CANVAS_NODE_HEIGHT,
  CANVAS_NODE_WIDTH,
  DEFAULT_CANVAS_VIEWPORT,
  markdownToCanvasPreview,
  nextChildPosition,
  projectConversationGraph,
  type CanvasNodeProjection,
  type CanvasPoint,
  type CanvasViewport,
} from "../canvas/graphProjection";
import {
  centerCanvasViewportOnPoint,
  clampCanvasZoom,
  panCanvasViewport,
  zoomCanvasViewportAtPoint,
} from "../canvas/canvasViewport";
import { projectCanvasBranchTrail } from "../canvas/canvasBranchNavigation";
import { projectBranchMemoryAudit } from "../canvas/branchMemoryAudit";
import type {
  CanvasFocusFrameQueryProjection,
  CanvasKnowledgeRetrievalProjection,
} from "../canvas/canvasM2Projection";
import { summarizeKnowledgeInventory } from "../app/knowledgeInventory";
import { buildFocusPromotionDecisionInput } from "../app/focusPromotionDecision";
import {
  isFocusPromotionSelectionChanged,
  reconcileFocusPromotionSelection,
  selectableFocusPromotionEntities,
} from "../app/focusPromotionSelection";
import { renderedMarkdownText, SafeMarkdown } from "./SafeMarkdown";
import "./conversationCanvas.css";

type CanvasRunView = {
  id: string;
  nodeId: string;
  prompt: string;
  parentNodeId: string | null;
  branchType?: BranchType;
  content: string;
  status: "starting" | "streaming" | "completed" | "cancelled" | "failed";
  providerId: string;
  modelId: string;
  error?: ProviderError | null;
  errorMessage?: string | null;
  partialContentRetained?: boolean;
};

type ConversationCanvasProps = {
  graph: ConversationGraph;
  modelRuns: readonly ModelRunProjection[];
  selectedNodeId: string | null;
  run: CanvasRunView | null;
  onSelectNode: (nodeId: string) => void;
  onSelectBranch: (nodeId: string, branchType: BranchType) => void;
  onInspectContext: (node: ConversationNode) => void;
  onMoveNode: (nodeId: string, position: CanvasPoint) => void;
  initialViewport?: CanvasViewport;
  onViewportChange: (viewport: CanvasViewport) => void;
  focusFrameQueryByNodeId?: ReadonlyMap<string, CanvasFocusFrameQueryProjection | null>;
  focusFrameQueryError?: string | null;
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
  onReloadFocusFrames?: () => Promise<void>;
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
};

type FocusPromotionLoadState =
  | { kind: "unavailable" }
  | { kind: "loading" }
  | { kind: "empty" }
  | { kind: "ready"; candidateSet: FocusPromotionCandidateSet }
  | { kind: "error"; message: string };

type FocusPromotionGenerationState =
  | { kind: "idle" }
  | { kind: "pending"; input: FocusPromotionCandidateGenerationCommandInput }
  | { kind: "success"; projection: FocusPromotionCandidateGenerationProjection }
  | { kind: "error"; input: FocusPromotionCandidateGenerationCommandInput; message: string };

type FocusPromotionDecisionState =
  | { kind: "idle" }
  | { kind: "pending"; input: FocusPromotionDecisionCommandInput }
  | { kind: "error"; input: FocusPromotionDecisionCommandInput; message: string }
  | { kind: "success"; projection: FocusPromotionDecisionProjection };

type FocusPromotionDecisionHistoryState =
  | { kind: "unavailable" }
  | { kind: "loading" }
  | { kind: "ready"; decisions: FocusPromotionDecisionProjection[] }
  | { kind: "error"; message: string };

const FOCUS_PROMOTION_ACTION_LABEL: Record<FocusPromotionDecisionAction, string> = {
  confirm: "已确认（仅分支）",
  promote: "已回流",
  reject: "已否决",
  delete: "已删除",
};

type PointerInteraction =
  | {
      type: "pan";
      pointerId: number;
      startClient: CanvasPoint;
      startViewport: CanvasPoint;
    }
  | {
      type: "node";
      pointerId: number;
      nodeId: string;
      startClient: CanvasPoint;
      startPosition: CanvasPoint;
    };

const BRANCH_META: Record<BranchType, { label: string; shortLabel: string; className: string }> = {
  continues: { label: "普通继续", shortLabel: "继续", className: "is-continues" },
  deepens: { label: "深入", shortLabel: "深入", className: "is-deepens" },
  diverges: { label: "发散", shortLabel: "发散", className: "is-diverges" },
  reframes: { label: "换角度", shortLabel: "换角度", className: "is-reframes" },
  importedFrom: { label: "导入来源", shortLabel: "导入", className: "is-imported" },
};

const RUN_LABEL: Record<RunState, string> = {
  pending: "等待运行",
  streaming: "正在生成",
  completed: "已完成",
  cancelled: "已停止",
  failed: "运行失败",
};

const FOCUS_BRANCH_LABEL = {
  mainline: "主线",
  exploration: "探索分支",
  task: "任务分支",
  retrospective: "复盘分支",
} as const;

const FOCUS_CONTEXT_POLICY_LABEL = {
  continueCurrent: "继续当前问题",
  focusNew: "聚焦新问题",
  branchFromNode: "从节点分支",
  continueImportedRaw: "原样续接导入",
} as const;

function edgePath(source: CanvasNodeProjection, target: CanvasNodeProjection) {
  const startX = source.position.x + CANVAS_NODE_WIDTH;
  const startY = source.position.y + 76;
  const endX = target.position.x;
  const endY = target.position.y + 76;
  const distance = Math.max(96, Math.abs(endX - startX) * 0.48);
  return `M ${startX} ${startY} C ${startX + distance} ${startY}, ${endX - distance} ${endY}, ${endX} ${endY}`;
}

function nodeRunState(run: CanvasRunView): RunState {
  if (run.status === "starting") return "pending";
  return run.status;
}

function formatNodeTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  return new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit" }).format(date);
}

function CanvasNodeCard({
  node,
  selected,
  domainNode,
  onStartDrag,
  onSelect,
  onSelectBranch,
  onOpenReader,
  onInspectContext,
}: {
  node: CanvasNodeProjection;
  selected: boolean;
  domainNode: ConversationNode | null;
  onStartDrag: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onSelect: () => void;
  onSelectBranch: (branchType: BranchType) => void;
  onOpenReader: (trigger: HTMLButtonElement) => void;
  onInspectContext: () => void;
}) {
  const [copied, setCopied] = useState(false);
  const branch = BRANCH_META[node.branchType];
  const isImportedSource = node.origin.kind === "importedSource";

  const copyAnswer = async () => {
    const content = node.answer ?? node.question;
    await navigator.clipboard.writeText(content);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  return (
    <article
      className={`conversation-canvas-node${selected ? " is-selected" : ""}${node.runState === "streaming" ? " is-streaming" : ""}${isImportedSource ? " is-import-source" : ""}`}
      style={{
        width: CANVAS_NODE_WIDTH,
        minHeight: CANVAS_NODE_HEIGHT,
        transform: `translate(${node.position.x}px, ${node.position.y}px)`,
      }}
      data-node-id={node.id}
      aria-label={`${node.title}，${isImportedSource ? "导入原文" : "本地运行"}，${RUN_LABEL[node.runState]}`}
      onClick={onSelect}
    >
      <div className="canvas-node-header" onPointerDown={onStartDrag}>
        <span className="canvas-node-grip" aria-hidden="true"><Move /></span>
        <span className={`canvas-node-relation ${branch.className}`}>{branch.shortLabel}</span>
        <span className="canvas-node-id">{node.id.slice(-8)}</span>
        <span className={`canvas-node-state is-${node.runState}`}>{RUN_LABEL[node.runState]}</span>
      </div>

      <div className="canvas-node-question">
        {isImportedSource ? (
          <span className="canvas-node-origin"><FileText aria-hidden="true" />导入原文 · 非本地生成</span>
        ) : null}
        <span>{isImportedSource ? "原文内容" : "用户问题"}</span>
        <strong>{node.questionPreview}</strong>
      </div>

      <div className="canvas-node-answer">
        <div className="canvas-node-answer-meta">
          <span>
            {isImportedSource ? <FileText aria-hidden="true" /> : <Sparkles aria-hidden="true" />}
            {isImportedSource ? "外部会话原文" : node.modelId ?? "等待模型"}
          </span>
          <time>{formatNodeTime(node.createdAt)}</time>
        </div>
        {node.answerPreview ? <p className="canvas-node-answer-preview">{node.answerPreview}</p> : node.runState === "failed" ? (
          <p className="canvas-node-error">本次运行失败，错误已作为状态保留。</p>
        ) : node.runState === "cancelled" ? (
          <p className="canvas-node-muted">生成已停止，已有内容将按运行策略处理。</p>
        ) : (
          <p className="canvas-node-muted"><span className="canvas-stream-cursor" />正在等待回答内容…</p>
        )}
        {node.answer && node.runError ? (
          <p className="canvas-node-error">{node.runError.safeMessage}</p>
        ) : null}
        {node.answer && node.runState === "cancelled" ? (
          <p className="canvas-node-muted">
            {node.partialContentRetained ? "生成已停止，以上部分内容已保留。" : "生成已停止。"}
          </p>
        ) : null}
      </div>

      <div className="canvas-node-actions">
        <button type="button" onClick={(event) => { event.stopPropagation(); onSelectBranch("deepens"); }}>
          <ArrowUpRight aria-hidden="true" />深入
        </button>
        <button type="button" onClick={(event) => { event.stopPropagation(); onSelectBranch("diverges"); }}>
          <GitBranch aria-hidden="true" />发散
        </button>
        <button type="button" onClick={(event) => { event.stopPropagation(); onSelectBranch("reframes"); }}>
          <ArrowDown aria-hidden="true" />换角度
        </button>
        <button
          type="button"
          onClick={(event) => { event.stopPropagation(); onOpenReader(event.currentTarget); }}
          aria-label="聚焦阅读节点"
          title="聚焦阅读"
        >
          <BookOpen aria-hidden="true" />
        </button>
        <button
          type="button"
          disabled={!domainNode}
          onClick={(event) => { event.stopPropagation(); onInspectContext(); }}
          aria-label="查看本轮上下文"
          title="查看本轮上下文"
        >
          <Eye aria-hidden="true" />
        </button>
        <button
          type="button"
          onClick={(event) => { event.stopPropagation(); void copyAnswer(); }}
          aria-label="复制节点内容"
          title="复制节点内容"
        >
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
        </button>
      </div>
    </article>
  );
}

function MarkdownProjectionFact({
  entityId,
  projections,
  loading,
  error,
  onImportEdit,
}: {
  entityId: string;
  projections: readonly MarkdownProjection[] | undefined;
  loading: boolean;
  error: string | undefined;
  onImportEdit?: (entityId: string) => Promise<MarkdownEditCommandResult>;
}) {
  const latest = projections?.[0];
  const [syncing, setSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<"changed" | "unchanged" | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);

  useEffect(() => {
    setSyncResult(null);
    setSyncError(null);
  }, [entityId]);

  const importEdit = async () => {
    if (!onImportEdit || syncing) return;
    setSyncing(true);
    setSyncError(null);
    try {
      const result = await onImportEdit(entityId);
      setSyncResult(result.changed ? "changed" : "unchanged");
    } catch (importError) {
      setSyncError(importError instanceof Error ? importError.message : String(importError));
    } finally {
      setSyncing(false);
    }
  };

  if (error) {
    return (
      <div className="canvas-reader-markdown-projection is-error" role="status" title={error} aria-label={`${entityId} Markdown 投影读取失败`}>
        <FileText aria-hidden="true" />
        <span>Markdown 投影读取失败，可从画布状态条重试。</span>
      </div>
    );
  }

  if (!latest) {
    return (
      <div className={`canvas-reader-markdown-projection${loading ? " is-loading" : ""}`} role="status" aria-label={`${entityId} Markdown 投影状态`}>
        {loading ? <LoaderCircle className="spin" aria-hidden="true" /> : <FileText aria-hidden="true" />}
        <span>
          {loading
            ? "正在读取 Vault Markdown 投影…"
            : projections
              ? "当前实体暂无 Markdown 投影。"
              : "当前实体不在本次 Markdown 投影查询清单中。"}
        </span>
      </div>
    );
  }

  return (
    <div className="canvas-reader-markdown-projection" aria-label={`${entityId} 最新 Markdown 投影`}>
      <FileText aria-hidden="true" />
      <div>
        <span title={latest.relativePath}>Vault {latest.relativePath}</span>
        <span>
          投影 revision {latest.projectionRevision} · 实体 revision {latest.entityRevision}
          {projections && projections.length > 1 ? ` · ${projections.length} 条修订记录` : ""}
          {loading ? " · 正在刷新" : ""}
        </span>
        <code title={latest.contentHash}>content hash {latest.contentHash}</code>
        {onImportEdit ? (
          <button
            className="canvas-reader-markdown-sync"
            type="button"
            disabled={loading || syncing}
            onClick={() => void importEdit()}
          >
            {syncing ? <LoaderCircle className="spin" aria-hidden="true" /> : <RefreshCcw aria-hidden="true" />}
            {syncing ? "正在同步…" : "同步 Vault 编辑"}
          </button>
        ) : null}
        {syncResult ? (
          <small role="status">
            {syncResult === "changed" ? "已导入新修订，并清除旧检索投影。" : "内容未变化，未创建空修订。"}
          </small>
        ) : null}
        {syncError ? <small className="is-error" role="alert">同步失败：{syncError}</small> : null}
      </div>
    </div>
  );
}

function MemoryReferenceGroup({
  label,
  refs,
  emptyLabel,
}: {
  label: string;
  refs: readonly string[];
  emptyLabel: string;
}) {
  return (
    <details className="canvas-reader-memory-group">
      <summary>
        <span>{label}</span>
        <strong>{refs.length}</strong>
      </summary>
      {refs.length ? (
        <ul>
          {refs.map((reference, index) => (
            <li key={`${reference}:${index}`}><code>{reference}</code></li>
          ))}
        </ul>
      ) : <small>{emptyLabel}</small>}
    </details>
  );
}

export function BranchMemoryAuditPanel({
  query,
  promotionState,
  knowledgeEntitiesById,
  onRetryPromotion,
  onRefreshPromotionSource,
  decisionState,
  decisionHistoryState,
  onDecidePromotion,
}: {
  query: CanvasFocusFrameQueryProjection;
  promotionState: FocusPromotionLoadState;
  knowledgeEntitiesById: ReadonlyMap<string, KnowledgeEntity>;
  onRetryPromotion: () => void;
  onRefreshPromotionSource?: () => void;
  decisionState: FocusPromotionDecisionState;
  decisionHistoryState: FocusPromotionDecisionHistoryState;
  onDecidePromotion?: (action: FocusPromotionDecisionAction, candidateRef: string) => void;
}) {
  const audit = projectBranchMemoryAudit(query);

  return (
    <section className="canvas-reader-knowledge canvas-reader-memory-audit" aria-label="分支记忆作用域审计">
      <div className="canvas-reader-knowledge-title">
        <GitBranch aria-hidden="true" />
        <div>
          <span>分支记忆作用域</span>
          <strong>{FOCUS_BRANCH_LABEL[audit.branchKind]}</strong>
        </div>
      </div>
      <div className="canvas-reader-knowledge-meta">
        <span>memory version {audit.memoryVersion}</span>
        <span>{FOCUS_CONTEXT_POLICY_LABEL[audit.contextPolicy]}</span>
        <span>{query.lifecycle.status === "active" ? "活动" : "已关闭"}</span>
      </div>
      <div className="canvas-reader-memory-groups">
        <MemoryReferenceGroup label="继承声明" refs={audit.declared.inheritRefs} emptyLabel="未声明继承引用" />
        <MemoryReferenceGroup label="分支本地" refs={audit.declared.localRefs} emptyLabel="未声明分支本地引用" />
        <MemoryReferenceGroup label="显式排除" refs={audit.declared.excludeRefs} emptyLabel="未声明排除引用" />
        <MemoryReferenceGroup label="回流声明" refs={audit.declared.promoteRefs} emptyLabel="未声明回流引用" />
      </div>
      <div className="canvas-reader-promotion-candidates" aria-label="分支回流候选">
        <div>
          <strong>回流候选</strong>
          <span>内核原子决策 · 版本门禁</span>
        </div>
        {promotionState.kind === "loading" ? (
          <p className="canvas-node-muted" role="status"><LoaderCircle className="spin" aria-hidden="true" />正在查询回流候选…</p>
        ) : promotionState.kind === "unavailable" ? (
          <p className="canvas-node-muted">当前运行环境未提供本地回流候选查询。</p>
        ) : promotionState.kind === "error" ? (
          <div className="canvas-reader-promotion-error">
            <p className="canvas-node-error" role="alert">回流候选读取失败：{promotionState.message}</p>
            <div className="canvas-reader-promotion-actions">
              <button className="canvas-reader-focus-action" type="button" onClick={onRetryPromotion}>
                <RefreshCcw aria-hidden="true" />重新查询
              </button>
              {onRefreshPromotionSource ? (
                <button className="canvas-reader-focus-action" type="button" onClick={onRefreshPromotionSource}>
                  <RefreshCcw aria-hidden="true" />刷新 FocusFrame
                </button>
              ) : null}
            </div>
          </div>
        ) : promotionState.kind === "empty" ? (
          <p className="canvas-node-muted">
            {query.lifecycle.status === "active"
              ? "当前分支尚未完成；上方 promoteRefs 仅为声明，正式候选由内核保持隐藏。"
              : "内核返回空态：当前为主线，或该 FocusFrame 未声明回流候选。"}
          </p>
        ) : (
          <>
            <div className="canvas-reader-knowledge-meta">
              <span>source {promotionState.candidateSet.focusFrameId}</span>
              <span>{FOCUS_BRANCH_LABEL[promotionState.candidateSet.branchKind]}</span>
              <span>memory version {promotionState.candidateSet.memoryVersion}</span>
              <span>FocusFrame {query.lifecycle.status === "closed" ? "已关闭" : "活动"}</span>
            </div>
            {promotionState.candidateSet.candidateRefs.length ? (
              <ul className="canvas-reader-promotion-list">
                {promotionState.candidateSet.candidateRefs.map((reference, index) => {
                  const entity = knowledgeEntitiesById.get(reference);
                  return (
                    <li key={`${reference}:${index}`}>
                      <div>
                        <strong>{entity?.name ?? reference}</strong>
                        {entity ? <code>{reference}</code> : null}
                        {!entity ? <small>未加载到实体，不能提交决定</small> : null}
                      </div>
                      <div className="canvas-reader-promotion-row-actions" aria-label={`${entity?.name ?? reference} 的回流决定`}>
                        <button
                          type="button"
                          disabled={!entity || !onDecidePromotion || decisionState.kind === "pending"}
                          onClick={() => onDecidePromotion?.("confirm", reference)}
                        >
                          <Check aria-hidden="true" />确认（仅分支）
                        </button>
                        <button
                          type="button"
                          disabled={!entity || !onDecidePromotion || decisionState.kind === "pending"}
                          onClick={() => onDecidePromotion?.("promote", reference)}
                        >
                          <ArrowUpRight aria-hidden="true" />回流当前会话
                        </button>
                        <button
                          type="button"
                          disabled={!entity || !onDecidePromotion || decisionState.kind === "pending"}
                          onClick={() => onDecidePromotion?.("reject", reference)}
                        >
                          <X aria-hidden="true" />否决
                        </button>
                        <button
                          className="is-danger"
                          type="button"
                          disabled={!entity || !onDecidePromotion || decisionState.kind === "pending"}
                          onClick={() => onDecidePromotion?.("delete", reference)}
                        >
                          <Trash2 aria-hidden="true" />删除
                        </button>
                      </div>
                      {decisionState.kind === "pending" && decisionState.input.candidateRef === reference ? (
                        <small className="canvas-reader-promotion-feedback" role="status">
                          <LoaderCircle className="spin" aria-hidden="true" />正在提交原子决定…
                        </small>
                      ) : null}
                      {decisionState.kind === "error" && decisionState.input.candidateRef === reference ? (
                        <small className="canvas-reader-promotion-feedback is-error" role="alert">
                          {decisionState.message}
                        </small>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            ) : (
              <p className="canvas-reader-promotion-success" role="status">
                该 FocusFrame 的回流候选已全部处理；重启后仍以决策记录过滤，不会重新出现。
              </p>
            )}
            <p className="canvas-reader-memory-notice">
              确认只保留在当前分支；“回流当前会话”会创建新的确认实体并保留分支源实体与 EvidenceRef。
            </p>
            {decisionState.kind === "success" ? (
              <p className="canvas-reader-promotion-success" role="status">
                决定已持久化：{decisionState.projection.action} · revision {decisionState.projection.decisionRevision}
              </p>
            ) : null}
            <div className="canvas-reader-promotion-history" aria-label="回流候选决策记录">
              <strong>决策记录</strong>
              {decisionHistoryState.kind === "loading" ? (
                <p className="canvas-node-muted" role="status"><LoaderCircle className="spin" aria-hidden="true" />正在读取持久化决定…</p>
              ) : decisionHistoryState.kind === "unavailable" ? (
                <p className="canvas-node-muted">当前运行环境未提供决策历史查询。</p>
              ) : decisionHistoryState.kind === "error" ? (
                <p className="canvas-node-error" role="alert">决策记录读取失败：{decisionHistoryState.message}</p>
              ) : decisionHistoryState.decisions.length ? (
                <ul>
                  {decisionHistoryState.decisions.map((decision) => (
                    <li key={decision.decisionId}>
                      <div>
                        <strong>{knowledgeEntitiesById.get(decision.candidateRef)?.name ?? decision.candidateRef}</strong>
                        <code>{decision.candidateRef}</code>
                      </div>
                      <div>
                        <span>{FOCUS_PROMOTION_ACTION_LABEL[decision.action]}</span>
                        <small>decision revision {decision.decisionRevision} · {decision.decidedAt}</small>
                        {decision.promotedEntityId ? <code>target {decision.promotedEntityId}</code> : null}
                      </div>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="canvas-node-muted">尚无持久化决定。</p>
              )}
            </div>
          </>
        )}
      </div>
      <div className="canvas-reader-memory-frozen">
        <strong>冻结上下文实际选择</strong>
        {audit.frozen.state === "unavailable" ? (
          <p className="canvas-node-muted">FocusedContext 尚未编译，不能从声明集合推断实际继承结果。</p>
        ) : (
          <>
            <MemoryReferenceGroup label="实际选中" refs={audit.frozen.selectedRefs} emptyLabel="冻结上下文没有选中记忆引用" />
            <details className="canvas-reader-memory-group">
              <summary>
                <span>实际排除</span>
                <strong>{audit.frozen.omittedRefs.length}</strong>
              </summary>
              {audit.frozen.omittedRefs.length ? (
                <ul>
                  {audit.frozen.omittedRefs.map((reference, index) => (
                    <li key={`${reference.referenceId}:${reference.reason}:${index}`}>
                      <code>{reference.referenceId}</code>
                      <small>{reference.reason}</small>
                    </li>
                  ))}
                </ul>
              ) : <small>冻结上下文没有排除记忆引用</small>}
            </details>
          </>
        )}
      </div>
    </section>
  );
}

function CanvasFocusReader({
  node,
  domainNode,
  focusFrameQuery,
  onClose,
  onInspectContext,
  onCreateFocusFrame,
  onTransitionFocusFrame,
  onGenerateFocusPromotionCandidates,
  onReloadFocusFrames,
  onLoadFocusPromotionCandidates,
  onDecideFocusPromotion,
  onLoadFocusPromotionDecisions,
  focusFrameQueryError,
  knowledgeEntitiesById,
  markdownProjectionsByEntityId,
  markdownProjectionErrorsByEntityId,
  markdownProjectionsLoading,
  onImportMarkdownEntityEdit,
  knowledgeRetrieval,
  knowledgeRetrievalLoading,
  knowledgeRetrievalError,
  onRetrieveKnowledge,
}: {
  node: CanvasNodeProjection;
  domainNode: ConversationNode | null;
  focusFrameQuery: CanvasFocusFrameQueryProjection | null | undefined;
  onClose: () => void;
  onInspectContext: () => void;
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
  onReloadFocusFrames?: () => Promise<void>;
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
  focusFrameQueryError?: string | null;
  knowledgeEntitiesById: ReadonlyMap<string, KnowledgeEntity>;
  markdownProjectionsByEntityId: ReadonlyMap<string, readonly MarkdownProjection[]>;
  markdownProjectionErrorsByEntityId: ReadonlyMap<string, string>;
  markdownProjectionsLoading: boolean;
  onImportMarkdownEntityEdit?: (entityId: string) => Promise<MarkdownEditCommandResult>;
  knowledgeRetrieval: CanvasKnowledgeRetrievalProjection | undefined;
  knowledgeRetrievalLoading: boolean;
  knowledgeRetrievalError: string | undefined;
  onRetrieveKnowledge?: (
    node: ConversationNode,
    query: string,
  ) => Promise<CanvasKnowledgeRetrievalProjection>;
}) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const [copiedMode, setCopiedMode] = useState<"readable" | "markdown" | null>(null);
  const readerMarkdownRef = useRef<HTMLDivElement>(null);
  const [objective, setObjective] = useState(node.question);
  const [lifecycleAction, setLifecycleAction] = useState<"create" | "close" | "reopen" | null>(null);
  const [lifecycleError, setLifecycleError] = useState<string | null>(null);
  const [retrievalQuery, setRetrievalQuery] = useState(node.question);
  const [retrievalAction, setRetrievalAction] = useState(false);
  const [promotionReloadToken, setPromotionReloadToken] = useState(0);
  const [selectedPromotionRefs, setSelectedPromotionRefs] = useState<string[]>([]);
  const [promotionGenerationState, setPromotionGenerationState] = useState<FocusPromotionGenerationState>({
    kind: "idle",
  });
  const [promotionState, setPromotionState] = useState<FocusPromotionLoadState>({
    kind: "unavailable",
  });
  const [promotionDecisionState, setPromotionDecisionState] = useState<FocusPromotionDecisionState>({
    kind: "idle",
  });
  const [promotionDecisionHistoryState, setPromotionDecisionHistoryState] = useState<FocusPromotionDecisionHistoryState>({
    kind: "unavailable",
  });
  const branch = BRANCH_META[node.branchType];
  const isImportedSource = node.origin.kind === "importedSource";
  const knowledgeContext = focusFrameQuery?.focusedContext?.knowledgeContext;
  const focusFrameId = focusFrameQuery?.lifecycle.focusFrame.id ?? null;
  const focusMemoryVersion = focusFrameQuery?.lifecycle.focusFrame.memoryVersion ?? null;
  const selectablePromotionEntities = useMemo(
    () => selectableFocusPromotionEntities([...knowledgeEntitiesById.values()], focusFrameQuery),
    [focusFrameQuery, knowledgeEntitiesById],
  );
  const selectablePromotionKey = selectablePromotionEntities.map((entity) => entity.id).join("\u0000");
  const currentPromotionRefs = focusFrameQuery?.lifecycle.focusFrame.memoryScope.promoteRefs ?? [];
  const promotionSelectionChanged = isFocusPromotionSelectionChanged(
    selectedPromotionRefs,
    currentPromotionRefs,
  );

  useEffect(() => {
    dialogRef.current?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  useEffect(() => {
    setObjective(node.question);
    setLifecycleError(null);
    setRetrievalQuery(node.question);
    setPromotionDecisionState({ kind: "idle" });
  }, [node.id, node.question]);

  useEffect(() => {
    const currentRefs = focusFrameQuery?.lifecycle.focusFrame.memoryScope.promoteRefs ?? [];
    setSelectedPromotionRefs(reconcileFocusPromotionSelection(
      currentRefs,
      selectablePromotionEntities.map((entity) => entity.id),
    ));
    setPromotionGenerationState({ kind: "idle" });
  }, [focusFrameId, focusMemoryVersion, selectablePromotionKey]);

  useEffect(() => {
    if (!focusFrameId || focusMemoryVersion === null || !onLoadFocusPromotionCandidates) {
      setPromotionState({ kind: "unavailable" });
      return;
    }

    let active = true;
    setPromotionState({ kind: "loading" });
    void onLoadFocusPromotionCandidates(focusFrameId, focusMemoryVersion).then(
      (candidateSet) => {
        if (!active) return;
        setPromotionState(candidateSet
          ? { kind: "ready", candidateSet }
          : { kind: "empty" });
      },
      (error: unknown) => {
        if (!active) return;
        setPromotionState({
          kind: "error",
          message: error instanceof Error ? error.message : String(error),
        });
      },
    );
    return () => {
      active = false;
    };
  }, [focusFrameId, focusMemoryVersion, onLoadFocusPromotionCandidates, promotionReloadToken]);

  useEffect(() => {
    if (!focusFrameId || !onLoadFocusPromotionDecisions) {
      setPromotionDecisionHistoryState({ kind: "unavailable" });
      return;
    }
    let active = true;
    setPromotionDecisionHistoryState({ kind: "loading" });
    void onLoadFocusPromotionDecisions(focusFrameId).then(
      (decisions) => {
        if (active) setPromotionDecisionHistoryState({ kind: "ready", decisions });
      },
      (error: unknown) => {
        if (!active) return;
        setPromotionDecisionHistoryState({
          kind: "error",
          message: error instanceof Error ? error.message : String(error),
        });
      },
    );
    return () => {
      active = false;
    };
  }, [focusFrameId, onLoadFocusPromotionDecisions, promotionReloadToken]);

  const decidePromotion = async (
    action: FocusPromotionDecisionAction,
    candidateRef: string,
  ) => {
    if (promotionDecisionState.kind === "pending" || promotionState.kind !== "ready" || !focusFrameQuery || !onDecideFocusPromotion) return;
    const entity = knowledgeEntitiesById.get(candidateRef);
    if (!entity) {
      return;
    }
    const confirmation = action === "promote"
      ? "将此候选回流到当前会话知识？系统会保留分支源实体和证据。"
      : action === "reject"
        ? "否决此候选？否决后不会进入检索上下文。"
        : action === "delete"
          ? "删除分支源实体？决定墓碑会保留，重启后不会重新出现。"
          : null;
    if (confirmation && !window.confirm(confirmation)) return;

    const retryInput = promotionDecisionState.kind === "error"
      && promotionDecisionState.input.candidateRef === candidateRef
      && promotionDecisionState.input.action === action
      ? promotionDecisionState.input
      : null;
    const suffix = crypto.randomUUID();
    const input = retryInput ?? buildFocusPromotionDecisionInput(
        action,
        promotionState.candidateSet,
        focusFrameQuery,
        entity,
        {
          decisionId: `focus-promotion-decision-${suffix}`,
          promotedEntityId: `knowledge-promotion-${suffix}`,
          decidedAt: new Date().toISOString(),
        },
      );
    try {
      setPromotionDecisionState({ kind: "pending", input });
      const projection = await onDecideFocusPromotion(input);
      setPromotionDecisionState({ kind: "success", projection });
      setPromotionReloadToken((current) => current + 1);
    } catch (error) {
      setPromotionDecisionState({
        kind: "error",
        input,
        message: error instanceof Error ? error.message : String(error),
      });
    }
  };

  const createFocusFrame = async () => {
    if (!onCreateFocusFrame || !domainNode || lifecycleAction) return;
    setLifecycleAction("create");
    setLifecycleError(null);
    try {
      await onCreateFocusFrame(domainNode, objective);
    } catch (error) {
      setLifecycleError(error instanceof Error ? error.message : String(error));
    } finally {
      setLifecycleAction(null);
    }
  };

  const transitionFocusFrame = async (action: "close" | "reopen") => {
    if (!onTransitionFocusFrame || !focusFrameQuery || lifecycleAction) return;
    setLifecycleAction(action);
    setLifecycleError(null);
    try {
      await onTransitionFocusFrame(action, focusFrameQuery);
    } catch (error) {
      setLifecycleError(error instanceof Error ? error.message : String(error));
    } finally {
      setLifecycleAction(null);
    }
  };

  const generateFocusPromotionCandidates = async (
    retryInput?: FocusPromotionCandidateGenerationCommandInput,
  ) => {
    if (
      !onGenerateFocusPromotionCandidates
      || !focusFrameQuery
      || focusFrameQuery.lifecycle.status !== "active"
      || promotionGenerationState.kind === "pending"
      || selectedPromotionRefs.length === 0
      || !promotionSelectionChanged
    ) return;

    const input = retryInput ?? {
      generationId: `focus-promotion-generation-${crypto.randomUUID()}`,
      focusFrameId: focusFrameQuery.lifecycle.focusFrame.id,
      expectedMemoryVersion: focusFrameQuery.lifecycle.focusFrame.memoryVersion,
      expectedLifecycleRevision: focusFrameQuery.lifecycle.revision,
      candidateRefs: [...selectedPromotionRefs].sort(),
      generatedAt: new Date().toISOString(),
    } satisfies FocusPromotionCandidateGenerationCommandInput;

    setPromotionGenerationState({ kind: "pending", input });
    try {
      const projection = await onGenerateFocusPromotionCandidates(input);
      setPromotionGenerationState({ kind: "success", projection });
    } catch (error) {
      setPromotionGenerationState({
        kind: "error",
        input,
        message: error instanceof Error ? error.message : String(error),
      });
    }
  };

  const reloadFocusFrames = async () => {
    if (!onReloadFocusFrames || lifecycleAction) return;
    setLifecycleAction("create");
    setLifecycleError(null);
    try {
      await onReloadFocusFrames();
    } catch (error) {
      setLifecycleError(error instanceof Error ? error.message : String(error));
    } finally {
      setLifecycleAction(null);
    }
  };

  const markdownSource = isImportedSource ? node.question : node.answer ?? "";

  const markCopied = (mode: "readable" | "markdown") => {
    setCopiedMode(mode);
    window.setTimeout(() => setCopiedMode(null), 1200);
  };

  const copyReadableContent = async () => {
    const readableText = renderedMarkdownText(readerMarkdownRef.current);
    if (!readableText) return;
    await navigator.clipboard.writeText(readableText);
    markCopied("readable");
  };

  const copyMarkdownContent = async () => {
    if (!markdownSource) return;
    await navigator.clipboard.writeText(markdownSource);
    markCopied("markdown");
  };

  const retrieveKnowledge = async () => {
    if (!onRetrieveKnowledge || !domainNode || retrievalAction) return;
    setRetrievalAction(true);
    try {
      await onRetrieveKnowledge(domainNode, retrievalQuery);
    } catch {
      // The parent owns the safe error projection; keep the reader open for retry.
    } finally {
      setRetrievalAction(false);
    }
  };

  return (
    <div
      className="canvas-reader-backdrop"
      onPointerDown={(event) => {
        event.stopPropagation();
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="canvas-reader"
        role="dialog"
        aria-modal="true"
        aria-labelledby="canvas-reader-title"
        tabIndex={-1}
      >
        <header className="canvas-reader-header">
          <div>
            <span className={`canvas-node-relation ${branch.className}`}>{branch.label}</span>
            <span className={`canvas-node-state is-${node.runState}`}>{RUN_LABEL[node.runState]}</span>
          </div>
          <button type="button" onClick={onClose} aria-label="关闭聚焦阅读"><X aria-hidden="true" /></button>
        </header>

        <div className="canvas-reader-scroll">
          {isImportedSource ? (
            <aside className="canvas-reader-origin">
              <FileText aria-hidden="true" />
              <div>
                <strong>导入原文</strong>
                <span>该节点由领域图标记为外部来源，不表示 MindScape 在本地重新生成。</span>
              </div>
            </aside>
          ) : null}
          <section className="canvas-reader-question">
            <span>{isImportedSource ? "原文内容" : "用户问题"}</span>
            <h2 id="canvas-reader-title">{isImportedSource ? "导入原文" : node.question}</h2>
            {isImportedSource ? (
              <SafeMarkdown
                markdown={node.question}
                className="canvas-reader-markdown is-imported-source"
                contentRef={readerMarkdownRef}
              />
            ) : null}
          </section>
          <section className="canvas-reader-answer">
            <div className="canvas-reader-meta">
              <span>
                {isImportedSource ? <FileText aria-hidden="true" /> : <Sparkles aria-hidden="true" />}
                {isImportedSource ? "外部会话原文" : node.modelId ?? "等待模型"}
              </span>
              <time>{formatNodeTime(node.createdAt)}</time>
            </div>
            {/*
              UI-HANDOFF-06
              位置：画布聚焦阅读器的完整正文区域
              用途：消费共享 SafeMarkdown 派生层；画布卡片仍只渲染 answerPreview
              数据：node.answer 保持 raw Markdown；renderer DOM 不写回会话、FocusFrame 或知识对象
              状态：streaming 使用 deferred Markdown；空白、取消和失败继续由既有运行状态表达
              可替换范围：员工06可替换字体、宽度和视觉；不可改变 raw/preview/rendered 三层或安全 URL/HTML 边界
            */}
            {node.answer ? (
              <SafeMarkdown
                markdown={node.answer}
                streaming={node.runState === "streaming"}
                className="canvas-reader-markdown"
                contentRef={readerMarkdownRef}
              />
            ) : (
              <p className="canvas-node-muted">当前节点尚无可阅读的回答内容。</p>
            )}
            {node.runError ? <p className="canvas-node-error">{node.runError.safeMessage}</p> : null}
            {node.runState === "cancelled" ? (
              <p className="canvas-node-muted">
                {node.partialContentRetained ? "生成已停止，以上部分内容已保留。" : "生成已停止。"}
              </p>
            ) : null}
          </section>
          {/*
            UI-HANDOFF-06
            位置：画布聚焦阅读器的 FocusFrame 生命周期控制区
            用途：显式创建当前节点 FocusFrame，并消费内核返回的 active/closed revision
            数据/IPC：list_knowledge_entities 提供可见实体；generate_focus_promotion_candidates、onCreateFocusFrame、onTransitionFocusFrame 由 App 走 typed IPC 并刷新权威投影
            状态：未绑定显示创建表单；active 显示候选空态/多选/提交/成功/失败和关闭；closed 显示重新打开及既有四动作
            交互约束：创建必须由用户确认目标；parentNodeId 只使用领域节点 ID，不从坐标推断；不执行历史内容、不接触 Key
            可替换范围：员工06可替换布局、视觉和动效，但不可改变显式用户选择、generationId 重试、EvidenceRef、memory/lifecycle version 或状态语义
          */}
          <section className="canvas-reader-focus-control" aria-label="FocusFrame 生命周期">
            <div className="canvas-reader-focus-title">
              <Crosshair aria-hidden="true" />
              <div>
                <span>聚焦上下文</span>
                <strong>
                  {focusFrameQuery
                    ? focusFrameQuery.lifecycle.status === "active" ? "运行中" : "已关闭"
                    : "当前节点未绑定"}
                </strong>
              </div>
            </div>
            {focusFrameQueryError ? (
              <>
                <p className="canvas-node-error" role="alert">FocusFrame 状态恢复失败：{focusFrameQueryError}</p>
                <button
                  className="canvas-reader-focus-action"
                  type="button"
                  disabled={!onReloadFocusFrames || Boolean(lifecycleAction)}
                  onClick={() => void reloadFocusFrames()}
                >
                  {lifecycleAction ? <LoaderCircle className="spin" aria-hidden="true" /> : null}
                  {lifecycleAction ? "正在重试…" : "重试读取 FocusFrame"}
                </button>
              </>
            ) : focusFrameQuery ? (
              <>
                <div className="canvas-reader-focus-meta">
                  <span>revision {focusFrameQuery.lifecycle.revision}</span>
                  <span>{focusFrameQuery.lifecycle.status === "active" ? "可继续使用" : "可重新打开"}</span>
                </div>
                {focusFrameQuery.lifecycle.status === "active" ? (
                  <div className="canvas-reader-promotion-selector">
                    <div className="canvas-reader-promotion-selector-title">
                      <strong>正式回流候选</strong>
                      <span>已选 {selectedPromotionRefs.length} / 可选 {selectablePromotionEntities.length}</span>
                    </div>
                    <p>
                      仅展示本 FocusFrame 内带来源的候选或推断知识；内核会在保存时重新校验所属分支、状态、版本与 EvidenceRef。关闭 FocusFrame 后才进入四类动作。
                    </p>
                    {selectablePromotionEntities.length ? (
                      <ul aria-label="选择正式回流候选">
                        {selectablePromotionEntities.map((entity) => (
                          <li key={entity.id}>
                            <label>
                              <input
                                type="checkbox"
                                checked={selectedPromotionRefs.includes(entity.id)}
                                disabled={!onGenerateFocusPromotionCandidates || promotionGenerationState.kind === "pending"}
                                onChange={(event) => {
                                  setPromotionGenerationState({ kind: "idle" });
                                  setSelectedPromotionRefs((current) => event.target.checked
                                    ? [...current, entity.id]
                                    : current.filter((reference) => reference !== entity.id));
                                }}
                              />
                              <span>
                                <strong>{entity.name}</strong>
                                <small>{entity.status} · revision {entity.revision} · {entity.evidence.length} 条来源</small>
                              </span>
                            </label>
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <p className="canvas-node-muted">当前分支还没有可选择的带来源候选知识。</p>
                    )}
                    <button
                      className="canvas-reader-focus-action"
                      type="button"
                      disabled={
                        !onGenerateFocusPromotionCandidates
                        || promotionGenerationState.kind === "pending"
                        || selectedPromotionRefs.length === 0
                        || !promotionSelectionChanged
                      }
                      onClick={() => void generateFocusPromotionCandidates()}
                    >
                      {promotionGenerationState.kind === "pending" ? <LoaderCircle className="spin" aria-hidden="true" /> : <Check aria-hidden="true" />}
                      {promotionGenerationState.kind === "pending" ? "正在保存…" : promotionSelectionChanged ? "保存候选选择" : "候选选择已保存"}
                    </button>
                    {promotionGenerationState.kind === "error" ? (
                      <div className="canvas-reader-promotion-feedback" role="alert">
                        <p className="canvas-node-error">候选选择保存失败：{promotionGenerationState.message}</p>
                        <button
                          className="canvas-reader-promotion-retry"
                          type="button"
                          onClick={() => void generateFocusPromotionCandidates(promotionGenerationState.input)}
                        >
                          使用同一收据重试
                        </button>
                      </div>
                    ) : null}
                    {promotionGenerationState.kind === "success" ? (
                      <p className="canvas-node-muted" role="status">
                        内核已保存 {promotionGenerationState.projection.candidateRefs.length} 条候选 · memory v{promotionGenerationState.projection.memoryVersion} · lifecycle r{promotionGenerationState.projection.lifecycleRevision}
                      </p>
                    ) : null}
                  </div>
                ) : null}
                <button
                  className="canvas-reader-focus-action"
                  type="button"
                  disabled={!onTransitionFocusFrame || Boolean(lifecycleAction)}
                  onClick={() => void transitionFocusFrame(focusFrameQuery.lifecycle.status === "active" ? "close" : "reopen")}
                >
                  {lifecycleAction ? <LoaderCircle className="spin" aria-hidden="true" /> : null}
                  {lifecycleAction === "close" ? "正在关闭…" : lifecycleAction === "reopen" ? "正在重新打开…" : focusFrameQuery.lifecycle.status === "active" ? "关闭 FocusFrame" : "重新打开 FocusFrame"}
                </button>
              </>
            ) : (
              <>
                <label className="canvas-reader-focus-field">
                  <span>本次目标</span>
                  <input value={objective} onChange={(event) => setObjective(event.target.value)} placeholder="例如：围绕当前节点继续拆解" />
                </label>
                <button
                  className="canvas-reader-focus-action"
                  type="button"
                  disabled={!domainNode || !onCreateFocusFrame || !objective.trim() || Boolean(lifecycleAction)}
                  onClick={() => void createFocusFrame()}
                >
                  {lifecycleAction === "create" ? <LoaderCircle className="spin" aria-hidden="true" /> : null}
                  {lifecycleAction === "create" ? "正在创建…" : "为此节点创建 FocusFrame"}
                </button>
                <p className="canvas-node-muted">创建后由本地内核持有生命周期；不会从节点坐标推断 FocusFrame。</p>
              </>
            )}
            {lifecycleError ? <p className="canvas-node-error" role="alert">{lifecycleError}</p> : null}
          </section>
          {/*
            UI-HANDOFF-06
            位置：FocusFrame 生命周期控制与冻结知识上下文之间
            用途：审计内核返回的 branchKind、memoryVersion、inherit/local/exclude/promote 声明及 FocusedContext 实际 selected/omitted
            数据/IPC：get_focus_frame_query 的 typed 只读投影；前端只复制字段，不执行继承、排除或回流规则
            状态：仅在已绑定 FocusFrame 时显示；FocusedContext unavailable 时明确不推断实际选择；空集合保持 0
            回流边界：promoteRefs 只标记为声明；正式候选来自 get_focus_promotion_candidates，四类动作只调用 decide_focus_promotion；pending/error/success 不替代内核终态，陈旧版本通过刷新恢复
            可替换范围：员工06可替换布局、密度与视觉，不可改变声明/冻结结果分层或回流候选状态
          */}
          {focusFrameQuery ? (
            <BranchMemoryAuditPanel
              query={focusFrameQuery}
              promotionState={promotionState}
              knowledgeEntitiesById={knowledgeEntitiesById}
              onRetryPromotion={() => setPromotionReloadToken((current) => current + 1)}
              onRefreshPromotionSource={() => void reloadFocusFrames()}
              decisionState={promotionDecisionState}
              decisionHistoryState={promotionDecisionHistoryState}
              onDecidePromotion={onDecideFocusPromotion ? (action, candidateRef) => void decidePromotion(action, candidateRef) : undefined}
            />
          ) : null}
          {/*
            UI-HANDOFF-06
            位置：画布聚焦阅读器的回答正文下方
            用途：展示本节点冻结知识上下文的 selected / omitted 引用与可审计来源
            数据/IPC：focusFrameQueryByNodeId；由 get_focus_frame_query 提供生命周期与可选 FocusedContextSnapshot，当前 undefined 表示尚未查询
            Markdown 投影：list_markdown_projections 的 typed 结果；仅展示相对路径、修订号与内容哈希，不直接读取或执行 Vault 文件
            显式回流：onImportMarkdownEntityEdit -> import_markdown_entity_edit；仅由用户按钮触发，不接受路径输入、不内建文档编辑器
            状态：undefined 表示等待查询；null 表示当前节点没有 FocusFrame；focusedContextState=unavailable 表示快照尚未编译/暂不可用；availableWithoutKnowledge 表示快照无知识上下文；有知识时显示引用、预算与排除原因
            交互约束：保持只读、键盘可滚动和语义列表；不得执行来源内容或在 UI 重算检索/预算
            可替换范围：员工06可替换分组、密度、图标和视觉强调
            不可改变：undefined/null 的状态区别、selected/omitted 字段语义、EvidenceRef 来源类型和安全边界
          */}
          <section className="canvas-reader-knowledge" aria-label="冻结知识上下文">
            <div className="canvas-reader-knowledge-title">
              <BrainCircuit aria-hidden="true" />
              <div>
                <span>冻结知识上下文</span>
                <strong>
                  {focusFrameQuery === undefined
                    ? "待内核查询接入"
                    : focusFrameQuery === null
                      ? "本节点没有 FocusFrame"
                      : focusFrameQuery.focusedContextState === "unavailable"
                        ? "快照待编译"
                        : focusFrameQuery.focusedContextState === "availableWithoutKnowledge"
                          ? "已查询，无知识引用"
                          : `${knowledgeContext?.selected.length ?? 0} 条已选 · ${knowledgeContext?.omitted.length ?? 0} 条排除`}
                </strong>
              </div>
            </div>
            {focusFrameQuery === undefined ? (
              <p className="canvas-node-muted">
                FocusFrame 查询尚未执行，此处不会伪造知识来源。
              </p>
            ) : focusFrameQuery === null ? (
              <p className="canvas-node-muted">当前节点尚未创建 FocusFrame。</p>
            ) : focusFrameQuery.focusedContextState === "unavailable" ? (
              <p className="canvas-node-muted">
                FocusFrame 生命周期已查询，但 FocusedContextSnapshot 尚未编译或暂不可用。
              </p>
            ) : focusFrameQuery.focusedContextState === "availableWithoutKnowledge" ? (
              <p className="canvas-node-muted">已读取冻结快照，本轮没有注入知识上下文。</p>
            ) : (
              <>
                <div className="canvas-reader-knowledge-meta">
                  <span>FocusFrame {focusFrameQuery.lifecycle.status}</span>
                  <span>revision {focusFrameQuery.lifecycle.revision}</span>
                  <span>{focusFrameQuery.focusedContext?.snapshotId}</span>
                </div>
                <div className="canvas-reader-knowledge-meta">
                  <span>{knowledgeContext?.estimatedTokens ?? 0} tokens</span>
                  <span>检索版本 {knowledgeContext?.retrievalVersion}</span>
                </div>
                {knowledgeContext?.selected.length ? (
                  <ul className="canvas-reader-knowledge-list" aria-label="已选择知识引用">
                    {knowledgeContext.selected.map((reference) => (
                      <li key={`${reference.entityId}:${reference.revision}`}>
                        <strong>{knowledgeEntitiesById.get(reference.entityId)?.name ?? reference.entityId}</strong>
                        <span>
                          {reference.status} · {reference.scopeType} · revision {reference.revision} · {reference.estimatedTokens} tokens
                        </span>
                        <small>
                          {reference.evidence.length
                            ? `${reference.evidence.length} 条证据：${reference.evidence.map((item) => item.targetType).join("、")}`
                            : "无可展示证据"}
                          {knowledgeEntitiesById.has(reference.entityId) ? ` · ${reference.entityId}` : ""}
                        </small>
                        <MarkdownProjectionFact
                          entityId={reference.entityId}
                          projections={markdownProjectionsByEntityId.get(reference.entityId)}
                          loading={markdownProjectionsLoading}
                          error={markdownProjectionErrorsByEntityId.get(reference.entityId)}
                          onImportEdit={onImportMarkdownEntityEdit}
                        />
                      </li>
                    ))}
                  </ul>
                ) : <p className="canvas-node-muted">编译器没有选择可注入的知识引用。</p>}
                {knowledgeContext?.omitted.length ? (
                  <details className="canvas-reader-knowledge-omitted">
                    <summary>查看 {knowledgeContext.omitted.length} 条排除说明</summary>
                    <ul>
                      {knowledgeContext.omitted.map((reference) => (
                        <li key={`${reference.referenceId}:${reference.reason}`}>
                          <strong>{reference.referenceId}</strong>
                          <span>{reference.reason}</span>
                        </li>
                      ))}
                    </ul>
                  </details>
                ) : null}
              </>
            )}
          </section>
          {/*
            UI-HANDOFF-06
            位置：聚焦阅读器冻结知识上下文之后
            用途：触发并展示内核统一检索投影，帮助用户核对候选来源与降级事实
            数据/IPC：onRetrieveKnowledge -> retrieve_knowledge；结果来自 mindscape.knowledge-retrieval.v1
            状态：未查询、加载、成功、空候选、错误均保持区分；候选顺序/分数/来源原样展示
            交互约束：不在前端检索、过滤、重排或确认候选；不显示原文、向量正文、Key 或 reasoning
            可替换范围：员工06可替换布局和视觉，不可改变 candidates/omitted/notice 语义
          */}
          <section className="canvas-reader-knowledge canvas-reader-retrieval" aria-label="统一知识检索">
            <div className="canvas-reader-knowledge-title">
              <BrainCircuit aria-hidden="true" />
              <div>
                <span>统一知识检索</span>
                <strong>{knowledgeRetrieval ? `${knowledgeRetrieval.candidates.length} 条候选` : "尚未查询"}</strong>
              </div>
            </div>
            <form
              className="canvas-reader-retrieval-form"
              onSubmit={(event) => {
                event.preventDefault();
                void retrieveKnowledge();
              }}
            >
              <label className="canvas-reader-focus-field">
                <span>检索问题</span>
                <input
                  value={retrievalQuery}
                  onChange={(event) => setRetrievalQuery(event.target.value)}
                  placeholder="例如：当前会话有哪些已确认决策？"
                />
              </label>
              <button
                className="canvas-reader-focus-action"
                type="submit"
                disabled={!onRetrieveKnowledge || !domainNode || !retrievalQuery.trim() || retrievalAction || knowledgeRetrievalLoading}
              >
                {retrievalAction || knowledgeRetrievalLoading ? <LoaderCircle className="spin" aria-hidden="true" /> : <BrainCircuit aria-hidden="true" />}
                {retrievalAction || knowledgeRetrievalLoading ? "正在检索…" : knowledgeRetrieval ? "重新检索" : "检索知识"}
              </button>
            </form>
            {knowledgeRetrievalError ? <p className="canvas-node-error" role="alert">知识检索失败：{knowledgeRetrievalError}</p> : null}
            {knowledgeRetrieval ? (
              <>
                <div className="canvas-reader-knowledge-meta">
                  <span>检索版本 {knowledgeRetrieval.retrievalVersion}</span>
                  <span>向量 {knowledgeRetrieval.notice.vectorStatus === "available" ? "可用" : "不可用"}</span>
                  {knowledgeRetrieval.notice.usedFallback ? <span>已使用降级召回</span> : null}
                </div>
                {knowledgeRetrieval.notice.safeMessage ? <p className="canvas-node-muted">{knowledgeRetrieval.notice.safeMessage}</p> : null}
                {knowledgeRetrieval.candidates.length ? (
                  <ul className="canvas-reader-knowledge-list" aria-label="检索候选">
                    {knowledgeRetrieval.candidates.map((candidate) => (
                      <li key={`${candidate.entity.id}:${candidate.entity.revision}`}>
                        <strong>{candidate.entity.name}</strong>
                        <span>{candidate.entity.id} · {candidate.entity.status} · {candidate.retrievalScore} 分 · {candidate.estimatedTokens} tokens</span>
                        <small>来源：{candidate.sources.join("、")}{candidate.embedding ? ` · embedding ${candidate.embedding.modelVersion}/${candidate.embedding.dimensions}d` : ""}</small>
                        <small>{candidate.evidence.length ? `${candidate.evidence.length} 条证据：${candidate.evidence.map((item) => item.targetType).join("、")}` : "无可展示证据"}</small>
                        <MarkdownProjectionFact
                          entityId={candidate.entity.id}
                          projections={markdownProjectionsByEntityId.get(candidate.entity.id)}
                          loading={markdownProjectionsLoading}
                          error={markdownProjectionErrorsByEntityId.get(candidate.entity.id)}
                          onImportEdit={onImportMarkdownEntityEdit}
                        />
                      </li>
                    ))}
                  </ul>
                ) : <p className="canvas-node-muted">内核未返回候选知识。</p>}
                {knowledgeRetrieval.omitted.length ? (
                  <details className="canvas-reader-knowledge-omitted">
                    <summary>查看 {knowledgeRetrieval.omitted.length} 条排除说明</summary>
                    <ul>
                      {knowledgeRetrieval.omitted.map((reference) => (
                        <li key={`${reference.referenceId}:${reference.reason}`}>
                          <strong>{reference.referenceId}</strong>
                          <span>{reference.reason}</span>
                        </li>
                      ))}
                    </ul>
                  </details>
                ) : null}
              </>
            ) : (
              <p className="canvas-node-muted">检索结果将由本地内核返回，前端不会猜测候选内容。</p>
            )}
          </section>
        </div>

        <footer className="canvas-reader-footer">
          <span>关闭后将返回原画布视口</span>
          <div>
            <button type="button" disabled={!markdownSource} onClick={() => void copyReadableContent()}>
              {copiedMode === "readable" ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
              {copiedMode === "readable" ? "已复制正文" : "复制正文"}
            </button>
            <button type="button" disabled={!markdownSource} onClick={() => void copyMarkdownContent()}>
              {copiedMode === "markdown" ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
              {copiedMode === "markdown" ? "已复制 Markdown" : "复制 Markdown"}
            </button>
            <button type="button" disabled={!domainNode} onClick={onInspectContext}>
              <Eye aria-hidden="true" />查看上下文
            </button>
          </div>
        </footer>
      </div>
    </div>
  );
}

export function ConversationCanvas({
  graph,
  modelRuns,
  selectedNodeId,
  run,
  onSelectNode,
  onSelectBranch,
  onInspectContext,
  onMoveNode,
  initialViewport,
  onViewportChange,
  focusFrameQueryByNodeId,
  focusFrameQueryError,
  onCreateFocusFrame,
  onTransitionFocusFrame,
  onGenerateFocusPromotionCandidates,
  onReloadFocusFrames,
  onLoadFocusPromotionCandidates,
  onDecideFocusPromotion,
  onLoadFocusPromotionDecisions,
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
}: ConversationCanvasProps) {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const readerTriggerRef = useRef<HTMLButtonElement | null>(null);
  const localPositionsRef = useRef(new Map<string, CanvasPoint>());
  const [localPositions, setLocalPositions] = useState(new Map<string, CanvasPoint>());
  const viewportRef = useRef<CanvasViewport>(initialViewport ?? { ...DEFAULT_CANVAS_VIEWPORT });
  const [viewport, setViewport] = useState<CanvasViewport>(viewportRef.current);
  const [interaction, setInteraction] = useState<PointerInteraction | null>(null);
  const [knowledgeRetrying, setKnowledgeRetrying] = useState(false);
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null);
  const revealedRunIdRef = useRef<string | null>(null);
  useEffect(() => {
    localPositionsRef.current = new Map();
    setLocalPositions(new Map());
    const restoredViewport = initialViewport ?? { ...DEFAULT_CANVAS_VIEWPORT };
    viewportRef.current = restoredViewport;
    setViewport(restoredViewport);
    setInteraction(null);
    setFocusedNodeId(null);
  }, [graph.conversation.id]);

  const updateViewport = (
    updater: CanvasViewport | ((current: CanvasViewport) => CanvasViewport),
  ) => {
    const next = typeof updater === "function" ? updater(viewportRef.current) : updater;
    viewportRef.current = next;
    setViewport(next);
    onViewportChange(next);
  };

  const projection = useMemo(
    () => projectConversationGraph(graph, localPositions, modelRuns),
    [graph, localPositions, modelRuns],
  );

  const domainNodes = useMemo(
    () => new Map(graph.nodes.map((node) => [node.id, node])),
    [graph.nodes],
  );

  const visibleNodes = useMemo(() => {
    if (!run) return projection.nodes;
    const existingNode = projection.nodes.find((node) => node.id === run.nodeId);
    if (existingNode) {
      return projection.nodes.map((node) => node.id === run.nodeId
        ? {
            ...node,
              answer: run.content || node.answer,
              answerPreview: run.content ? markdownToCanvasPreview(run.content) : node.answerPreview,
             runState: nodeRunState(run),
             providerId: run.providerId,
             modelId: run.modelId,
             runError: run.error ?? null,
             partialContentRetained: run.partialContentRetained ?? false,
          }
        : node);
    }
    const parent = projection.nodes.find((node) => node.id === run.parentNodeId);
    const siblings = projection.nodes.filter((node) => node.parentNodeId === run.parentNodeId).length;
    const transientNode: CanvasNodeProjection = {
      id: run.nodeId,
      title: run.prompt.length > 34 ? `${run.prompt.slice(0, 34)}…` : run.prompt,
      question: run.prompt,
      questionPreview: markdownToCanvasPreview(run.prompt),
      answer: run.content || null,
      answerPreview: run.content ? markdownToCanvasPreview(run.content) : null,
      providerId: run.providerId,
      modelId: run.modelId,
      runState: nodeRunState(run),
      runError: run.error ?? null,
      partialContentRetained: run.partialContentRetained ?? false,
      branchType: run.branchType ?? "continues",
      origin: { kind: "localRun" },
      parentNodeId: run.parentNodeId,
      createdAt: new Date().toISOString(),
      position: nextChildPosition(parent, siblings),
    };
    return [...projection.nodes, transientNode];
  }, [projection.nodes, run]);

  const visibleEdges = useMemo(() => {
    if (!run?.parentNodeId) return projection.edges;
    if (projection.edges.some((edge) => edge.targetNodeId === run.nodeId)) return projection.edges;
    return [
      ...projection.edges,
      {
        id: `edge-${run.id}`,
        sourceNodeId: run.parentNodeId,
        targetNodeId: run.nodeId,
        relation: run.branchType ?? "continues" as BranchType,
      },
    ];
  }, [projection.edges, run]);

  const nodesById = useMemo(
    () => new Map(visibleNodes.map((node) => [node.id, node])),
    [visibleNodes],
  );

  const importedNodeCount = useMemo(
    () => visibleNodes.filter((node) => node.origin.kind === "importedSource").length,
    [visibleNodes],
  );

  const knowledgeInventory = useMemo(
    () => summarizeKnowledgeInventory(knowledgeEntities, knowledgeRelations),
    [knowledgeEntities, knowledgeRelations],
  );

  const knowledgeEntitiesById = useMemo(
    () => new Map(knowledgeEntities.map((entity) => [entity.id, entity] as const)),
    [knowledgeEntities],
  );

  const markdownProjectedEntityCount = useMemo(
    () => [...markdownProjectionsByEntityId.values()].filter((history) => history.length > 0).length,
    [markdownProjectionsByEntityId],
  );

  const selectedFocusFrame = selectedNodeId && focusFrameQueryByNodeId
    ? focusFrameQueryByNodeId.get(selectedNodeId)
    : undefined;

  const branchTrail = useMemo(
    () => projectCanvasBranchTrail(visibleNodes, selectedNodeId),
    [selectedNodeId, visibleNodes],
  );

  const focusedNode = focusedNodeId ? nodesById.get(focusedNodeId) ?? null : null;

  const retryKnowledge = async () => {
    if (!onReloadKnowledge || knowledgeRetrying) return;
    setKnowledgeRetrying(true);
    try {
      await onReloadKnowledge();
    } finally {
      setKnowledgeRetrying(false);
    }
  };

  useEffect(() => {
    if (!run || revealedRunIdRef.current === run.id) return;
    const surface = surfaceRef.current;
    const runningNode = nodesById.get(run.nodeId);
    if (!surface || !runningNode) return;
    revealedRunIdRef.current = run.id;
    updateViewport((current) => centerCanvasViewportOnPoint(
      current,
      { width: surface.clientWidth, height: surface.clientHeight },
      {
        x: runningNode.position.x + CANVAS_NODE_WIDTH / 2,
        y: runningNode.position.y + CANVAS_NODE_HEIGHT / 2,
      },
    ));
  }, [run?.id, run?.nodeId, nodesById]);

  const updateLocalPosition = (nodeId: string, position: CanvasPoint) => {
    const next = new Map(localPositionsRef.current);
    next.set(nodeId, position);
    localPositionsRef.current = next;
    setLocalPositions(next);
  };

  const fitView = () => {
    const surface = surfaceRef.current;
    if (!surface || visibleNodes.length === 0) return;
    const bounds = visibleNodes.reduce(
      (current, node) => ({
        minX: Math.min(current.minX, node.position.x),
        minY: Math.min(current.minY, node.position.y),
        maxX: Math.max(current.maxX, node.position.x + CANVAS_NODE_WIDTH),
        maxY: Math.max(current.maxY, node.position.y + CANVAS_NODE_HEIGHT),
      }),
      { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity },
    );
    const padding = 104;
    const width = bounds.maxX - bounds.minX;
    const height = bounds.maxY - bounds.minY;
    const zoom = clampCanvasZoom(Math.min(
      (surface.clientWidth - padding * 2) / Math.max(width, 1),
      (surface.clientHeight - padding * 2) / Math.max(height, 1),
      1,
    ));
    updateViewport({
      zoom,
      x: (surface.clientWidth - width * zoom) / 2 - bounds.minX * zoom,
      y: (surface.clientHeight - height * zoom) / 2 - bounds.minY * zoom,
    });
  };

  const centerNode = (node: CanvasNodeProjection) => {
    const surface = surfaceRef.current;
    if (!surface) return;
    updateViewport((current) => centerCanvasViewportOnPoint(
      current,
      { width: surface.clientWidth, height: surface.clientHeight },
      {
        x: node.position.x + CANVAS_NODE_WIDTH / 2,
        y: node.position.y + CANVAS_NODE_HEIGHT / 2,
      },
    ));
  };

  const locateSelected = () => {
    const selected = visibleNodes.find((node) => node.id === selectedNodeId) ?? visibleNodes.at(-1);
    if (selected) centerNode(selected);
  };

  const selectBranchTrailNode = (nodeId: string) => {
    const node = nodesById.get(nodeId);
    if (!node) return;
    onSelectNode(nodeId);
    centerNode(node);
  };

  const zoomAtCenter = (delta: number) => {
    const surface = surfaceRef.current;
    if (!surface) return;
    updateViewport((current) => zoomCanvasViewportAtPoint(
      current,
      current.zoom + delta,
      { x: surface.clientWidth / 2, y: surface.clientHeight / 2 },
    ));
  };

  const handleWheel = (event: WheelEvent<HTMLDivElement>) => {
    event.preventDefault();
    const surface = surfaceRef.current;
    if (!surface) return;
    const rect = surface.getBoundingClientRect();
    const pointerX = event.clientX - rect.left;
    const pointerY = event.clientY - rect.top;
    updateViewport((current) => zoomCanvasViewportAtPoint(
      current,
      current.zoom * (event.deltaY > 0 ? 0.92 : 1.08),
      { x: pointerX, y: pointerY },
    ));
  };

  const startPan = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || (event.target as HTMLElement).closest(".canvas-toolbar, .canvas-branch-trail")) return;
    if ((event.target as HTMLElement).closest(".conversation-canvas-node")) return;
    surfaceRef.current?.setPointerCapture(event.pointerId);
    setInteraction({
      type: "pan",
      pointerId: event.pointerId,
      startClient: { x: event.clientX, y: event.clientY },
      startViewport: { x: viewport.x, y: viewport.y },
    });
  };

  const startNodeDrag = (
    event: ReactPointerEvent<HTMLDivElement>,
    node: CanvasNodeProjection,
  ) => {
    if (event.button !== 0 || node.id === run?.nodeId) return;
    event.preventDefault();
    event.stopPropagation();
    surfaceRef.current?.setPointerCapture(event.pointerId);
    setInteraction({
      type: "node",
      pointerId: event.pointerId,
      nodeId: node.id,
      startClient: { x: event.clientX, y: event.clientY },
      startPosition: node.position,
    });
  };

  const handlePointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!interaction || interaction.pointerId !== event.pointerId) return;
    const deltaX = event.clientX - interaction.startClient.x;
    const deltaY = event.clientY - interaction.startClient.y;
    if (interaction.type === "pan") {
      updateViewport((current) => panCanvasViewport(
        { ...current, ...interaction.startViewport },
        { x: deltaX, y: deltaY },
      ));
      return;
    }
    updateLocalPosition(interaction.nodeId, {
      x: Math.round(interaction.startPosition.x + deltaX / viewport.zoom),
      y: Math.round(interaction.startPosition.y + deltaY / viewport.zoom),
    });
  };

  const finishInteraction = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!interaction || interaction.pointerId !== event.pointerId) return;
    if (interaction.type === "node") {
      const position = localPositionsRef.current.get(interaction.nodeId) ?? interaction.startPosition;
      onMoveNode(interaction.nodeId, position);
    }
    if (surfaceRef.current?.hasPointerCapture(event.pointerId)) {
      surfaceRef.current.releasePointerCapture(event.pointerId);
    }
    setInteraction(null);
  };

  const closeReader = () => {
    setFocusedNodeId(null);
    window.requestAnimationFrame(() => readerTriggerRef.current?.focus());
  };

  return (
    <div
      ref={surfaceRef}
      className={`conversation-canvas${interaction?.type === "pan" ? " is-panning" : ""}`}
      onWheel={handleWheel}
      onPointerDown={startPan}
      onPointerMove={handlePointerMove}
      onPointerUp={finishInteraction}
      onPointerCancel={finishInteraction}
    >
      <div
        className="canvas-grid"
        style={{
          backgroundSize: `${28 * viewport.zoom}px ${28 * viewport.zoom}px`,
          backgroundPosition: `${viewport.x}px ${viewport.y}px`,
        }}
      />

      {/*
        UI-HANDOFF-06
        位置：画布左上角当前路径导航
        用途：按会话图显式 parentNodeId 显示从入口到当前节点的路径，并允许返回任一祖先节点
        数据/IPC：projectConversationGraph 的只读节点投影；不读取坐标以外的领域关系，不创建或切换 FocusFrame
        状态：未选节点时隐藏；缺失父节点或异常环时在最后一个权威节点停止；当前节点使用 aria-current
        可替换范围：员工06可替换布局、密度和视觉；不可改变 parentNodeId 路径、导入来源或分支类型语义
      */}
      {branchTrail.length ? (
        <nav className="canvas-branch-trail" aria-label="当前会话节点路径">
          <span className="canvas-branch-trail-label"><GitBranch aria-hidden="true" />当前路径</span>
          <div className="canvas-branch-trail-items">
            {branchTrail.map((item, index) => (
              <div className="canvas-branch-trail-item" key={item.nodeId}>
                {index > 0 ? <span aria-hidden="true">/</span> : null}
                <button
                  type="button"
                  className={item.isCurrent ? "is-current" : ""}
                  aria-current={item.isCurrent ? "location" : undefined}
                  title={`${BRANCH_META[item.branchType].label} · ${item.title}`}
                  onClick={() => selectBranchTrailNode(item.nodeId)}
                >
                  {item.parentNodeId === null ? "会话入口" : BRANCH_META[item.branchType].shortLabel}
                  <small>{item.title}</small>
                </button>
              </div>
            ))}
          </div>
        </nav>
      ) : null}

      <div className="canvas-toolbar" aria-label="画布视口控制">
        <button type="button" onClick={() => zoomAtCenter(-0.1)} aria-label="缩小画布"><Minus /></button>
        <span>{Math.round(viewport.zoom * 100)}%</span>
        <button type="button" onClick={() => zoomAtCenter(0.1)} aria-label="放大画布"><Plus /></button>
        <i />
        <button type="button" onClick={fitView} aria-label="适配全部节点" title="适配全部节点"><Maximize2 /></button>
        <button type="button" onClick={locateSelected} aria-label="定位当前节点" title="定位当前节点"><LocateFixed /></button>
      </div>

      {visibleNodes.length === 0 ? (
        <div className="canvas-empty-state">
          <span><Crosshair aria-hidden="true" /></span>
          <strong>空白会话画布</strong>
          <p>从底部输入第一个问题，回答会成为可继续运行的根节点。</p>
        </div>
      ) : null}

      <div
        className="canvas-world"
        style={{ transform: `translate(${viewport.x}px, ${viewport.y}px) scale(${viewport.zoom})` }}
      >
        <svg className="canvas-edges" viewBox="-2000 -2000 10000 10000" aria-hidden="true">
          {visibleEdges.map((edge) => {
            const source = nodesById.get(edge.sourceNodeId);
            const target = nodesById.get(edge.targetNodeId);
            if (!source || !target) return null;
            const branch = BRANCH_META[edge.relation];
            const focused = source.id === selectedNodeId || target.id === selectedNodeId;
            return (
              <g key={edge.id} className={`canvas-edge ${branch.className}${focused ? " is-focused" : ""}`}>
                <path d={edgePath(source, target)} />
                <text
                  x={(source.position.x + CANVAS_NODE_WIDTH + target.position.x) / 2}
                  y={(source.position.y + target.position.y) / 2 + 58}
                >
                  {branch.label}
                </text>
              </g>
            );
          })}
        </svg>

        <div className="canvas-nodes">
          {visibleNodes.map((node) => {
            const domainNode = domainNodes.get(node.id) ?? null;
            return (
              <CanvasNodeCard
                key={node.id}
                node={node}
                domainNode={domainNode}
                selected={node.id === selectedNodeId}
                onStartDrag={(event) => startNodeDrag(event, node)}
                onSelect={() => onSelectNode(node.id)}
                onSelectBranch={(branchType) => onSelectBranch(node.id, branchType)}
                onOpenReader={(trigger) => {
                  readerTriggerRef.current = trigger;
                  setFocusedNodeId(node.id);
                }}
                onInspectContext={() => { if (domainNode) onInspectContext(domainNode); }}
              />
            );
          })}
        </div>
      </div>

      {focusedNode ? (
        <CanvasFocusReader
          node={focusedNode}
          domainNode={domainNodes.get(focusedNode.id) ?? null}
          focusFrameQuery={focusFrameQueryByNodeId?.get(focusedNode.id)}
          focusFrameQueryError={focusFrameQueryError}
          onClose={closeReader}
          onInspectContext={() => {
            const domainNode = domainNodes.get(focusedNode.id);
            if (domainNode) onInspectContext(domainNode);
          }}
          onCreateFocusFrame={onCreateFocusFrame}
          onTransitionFocusFrame={onTransitionFocusFrame}
          onGenerateFocusPromotionCandidates={onGenerateFocusPromotionCandidates}
          onReloadFocusFrames={onReloadFocusFrames}
          onLoadFocusPromotionCandidates={onLoadFocusPromotionCandidates}
          onDecideFocusPromotion={onDecideFocusPromotion}
          onLoadFocusPromotionDecisions={onLoadFocusPromotionDecisions}
          knowledgeEntitiesById={knowledgeEntitiesById}
          markdownProjectionsByEntityId={markdownProjectionsByEntityId}
          markdownProjectionErrorsByEntityId={markdownProjectionErrorsByEntityId}
          markdownProjectionsLoading={markdownProjectionsLoading}
          onImportMarkdownEntityEdit={onImportMarkdownEntityEdit}
          knowledgeRetrieval={knowledgeRetrievalByNodeId.get(focusedNode.id)}
          knowledgeRetrievalLoading={knowledgeRetrievalLoadingNodeId === focusedNode.id}
          knowledgeRetrievalError={knowledgeRetrievalErrorByNodeId.get(focusedNode.id)}
          onRetrieveKnowledge={onRetrieveKnowledge}
        />
      ) : null}

      {/*
        UI-HANDOFF-06
        位置：画布底部会话状态条
        用途：显示内核已查询的知识对象数量、候选状态和当前节点 FocusFrame，不承载检索排序或确认操作
        数据/IPC：App 通过 list_knowledge_entities/list_knowledge_relations、list_markdown_projections 与 list_focus_frames 提供结构化结果
        状态：loading 显示读取中；知识或 Markdown 投影 error 显示安全错误并提供重试；空结果显示 0；FocusFrame 未查询/未绑定/已关闭保持区分
        可替换范围：员工06可替换布局和视觉，但不可改变对象数量、FocusFrame 状态或来源
      */}
      <div className="canvas-status-strip">
        <span><RefreshCcw aria-hidden="true" />{graph.nodes.length} 个正式节点</span>
        <span>{graph.edges.length} 条语义关系</span>
        {importedNodeCount > 0 ? <span><FileText aria-hidden="true" />{importedNodeCount} 个导入原文节点</span> : null}
        <span title={knowledgeError ?? undefined}>
          <BrainCircuit aria-hidden="true" />
          {knowledgeLoading
            ? "正在读取知识对象…"
            : knowledgeError
              ? "知识对象读取失败"
              : `知识 ${knowledgeInventory.entityCount} 实体 · ${knowledgeInventory.relationCount} 关系${knowledgeInventory.candidateCount ? ` · ${knowledgeInventory.candidateCount} 条候选` : ""}`}
        </span>
        <span title={markdownProjectionErrorsByEntityId.size ? `${markdownProjectionErrorsByEntityId.size} 个实体读取失败` : undefined}>
          <FileText aria-hidden="true" />
          {markdownProjectionsLoading
            ? "正在读取 Vault 投影…"
            : markdownProjectionErrorsByEntityId.size
              ? `Vault 投影 ${markdownProjectionErrorsByEntityId.size} 条失败`
              : `Vault 投影 ${markdownProjectedEntityCount} 个实体`}
        </span>
        {knowledgeError || markdownProjectionErrorsByEntityId.size ? (
          <button
            className="canvas-status-retry"
            type="button"
            disabled={!onReloadKnowledge || knowledgeRetrying}
            onClick={() => void retryKnowledge()}
          >
            {knowledgeRetrying
              ? "重试中…"
              : knowledgeError
                ? "重试知识查询"
                : "重试 Vault 投影"}
          </button>
        ) : null}
        <span title={selectedFocusFrame?.lifecycle.focusFrame.objective ?? undefined}>
          <Crosshair aria-hidden="true" />
          {focusFrameQueryError
            ? "FocusFrame 读取失败"
            : selectedFocusFrame
              ? `当前 FocusFrame · ${selectedFocusFrame.lifecycle.status === "active" ? "活动" : "已关闭"}`
              : "当前节点未绑定 FocusFrame"}
        </span>
        <span>拖拽空白处平移 · 滚轮缩放</span>
      </div>
    </div>
  );
}

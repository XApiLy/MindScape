import {
  ArrowDown,
  ArrowUpRight,
  BookOpen,
  Check,
  Copy,
  Crosshair,
  Eye,
  GitBranch,
  LocateFixed,
  Maximize2,
  Minus,
  Move,
  Plus,
  RefreshCcw,
  Sparkles,
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
  ModelRunProjection,
  ProviderError,
  RunState,
} from "../domain";
import {
  CANVAS_NODE_HEIGHT,
  CANVAS_NODE_WIDTH,
  DEFAULT_CANVAS_VIEWPORT,
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

  const copyAnswer = async () => {
    const content = node.answer ?? node.question;
    await navigator.clipboard.writeText(content);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  return (
    <article
      className={`conversation-canvas-node${selected ? " is-selected" : ""}${node.runState === "streaming" ? " is-streaming" : ""}`}
      style={{
        width: CANVAS_NODE_WIDTH,
        minHeight: CANVAS_NODE_HEIGHT,
        transform: `translate(${node.position.x}px, ${node.position.y}px)`,
      }}
      data-node-id={node.id}
      aria-label={`${node.title}，${RUN_LABEL[node.runState]}`}
      onClick={onSelect}
    >
      <div className="canvas-node-header" onPointerDown={onStartDrag}>
        <span className="canvas-node-grip" aria-hidden="true"><Move /></span>
        <span className={`canvas-node-relation ${branch.className}`}>{branch.shortLabel}</span>
        <span className="canvas-node-id">{node.id.slice(-8)}</span>
        <span className={`canvas-node-state is-${node.runState}`}>{RUN_LABEL[node.runState]}</span>
      </div>

      <div className="canvas-node-question">
        <span>用户问题</span>
        <strong>{node.question}</strong>
      </div>

      <div className="canvas-node-answer">
        <div className="canvas-node-answer-meta">
          <span><Sparkles aria-hidden="true" />{node.modelId ?? "等待模型"}</span>
          <time>{formatNodeTime(node.createdAt)}</time>
        </div>
        {node.answer ? <p>{node.answer}</p> : node.runState === "failed" ? (
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

function CanvasFocusReader({
  node,
  domainNode,
  onClose,
  onInspectContext,
}: {
  node: CanvasNodeProjection;
  domainNode: ConversationNode | null;
  onClose: () => void;
  onInspectContext: () => void;
}) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const [copied, setCopied] = useState(false);
  const branch = BRANCH_META[node.branchType];

  useEffect(() => {
    dialogRef.current?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const copyContent = async () => {
    await navigator.clipboard.writeText(`${node.question}\n\n${node.answer ?? ""}`.trim());
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
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
          <section className="canvas-reader-question">
            <span>用户问题</span>
            <h2 id="canvas-reader-title">{node.question}</h2>
          </section>
          <section className="canvas-reader-answer">
            <div className="canvas-reader-meta">
              <span><Sparkles aria-hidden="true" />{node.modelId ?? "等待模型"}</span>
              <time>{formatNodeTime(node.createdAt)}</time>
            </div>
            {node.answer ? (
              <p>{node.answer}</p>
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
        </div>

        <footer className="canvas-reader-footer">
          <span>关闭后将返回原画布视口</span>
          <div>
            <button type="button" onClick={() => void copyContent()}>
              {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
              {copied ? "已复制" : "复制全文"}
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
}: ConversationCanvasProps) {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const readerTriggerRef = useRef<HTMLButtonElement | null>(null);
  const localPositionsRef = useRef(new Map<string, CanvasPoint>());
  const [localPositions, setLocalPositions] = useState(new Map<string, CanvasPoint>());
  const viewportRef = useRef<CanvasViewport>(initialViewport ?? { ...DEFAULT_CANVAS_VIEWPORT });
  const [viewport, setViewport] = useState<CanvasViewport>(viewportRef.current);
  const [interaction, setInteraction] = useState<PointerInteraction | null>(null);
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
      answer: run.content || null,
      providerId: run.providerId,
      modelId: run.modelId,
      runState: nodeRunState(run),
      runError: run.error ?? null,
      partialContentRetained: run.partialContentRetained ?? false,
      branchType: run.branchType ?? "continues",
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

  const focusedNode = focusedNodeId ? nodesById.get(focusedNodeId) ?? null : null;

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

  const locateSelected = () => {
    const surface = surfaceRef.current;
    const selected = visibleNodes.find((node) => node.id === selectedNodeId) ?? visibleNodes.at(-1);
    if (!surface || !selected) return;
    updateViewport((current) => centerCanvasViewportOnPoint(
      current,
      { width: surface.clientWidth, height: surface.clientHeight },
      {
        x: selected.position.x + CANVAS_NODE_WIDTH / 2,
        y: selected.position.y + CANVAS_NODE_HEIGHT / 2,
      },
    ));
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
    if (event.button !== 0 || (event.target as HTMLElement).closest(".canvas-toolbar")) return;
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
          onClose={closeReader}
          onInspectContext={() => {
            const domainNode = domainNodes.get(focusedNode.id);
            if (domainNode) onInspectContext(domainNode);
          }}
        />
      ) : null}

      <div className="canvas-status-strip">
        <span><RefreshCcw aria-hidden="true" />{graph.nodes.length} 个正式节点</span>
        <span>{graph.edges.length} 条语义关系</span>
        <span>拖拽空白处平移 · 滚轮缩放</span>
      </div>
    </div>
  );
}

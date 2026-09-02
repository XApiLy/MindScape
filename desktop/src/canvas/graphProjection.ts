import type {
  BranchType,
  ContentBlock,
  ConversationGraph,
  ConversationNode,
  ModelRunProjection,
  ProviderError,
  RunState,
} from "../domain";

export const CANVAS_NODE_WIDTH = 404;
export const CANVAS_NODE_HEIGHT = 286;

export type CanvasPoint = {
  x: number;
  y: number;
};

export type CanvasViewport = CanvasPoint & {
  zoom: number;
};

export type CanvasNodeOriginProjection =
  | { kind: "localRun" }
  | { kind: "importedSource" };

export const DEFAULT_CANVAS_VIEWPORT: CanvasViewport = {
  x: 72,
  y: 72,
  zoom: 0.86,
};

export type CanvasNodeProjection = {
  id: string;
  title: string;
  question: string;
  questionPreview: string;
  answer: string | null;
  answerPreview: string | null;
  providerId: string | null;
  modelId: string | null;
  runState: RunState;
  runError: ProviderError | null;
  partialContentRetained: boolean;
  branchType: BranchType;
  origin: CanvasNodeOriginProjection;
  parentNodeId: string | null;
  createdAt: string;
  position: CanvasPoint;
};

export type CanvasEdgeProjection = {
  id: string;
  sourceNodeId: string;
  targetNodeId: string;
  relation: BranchType;
};

export type CanvasGraphProjection = {
  nodes: CanvasNodeProjection[];
  edges: CanvasEdgeProjection[];
};

export function blocksToPlainText(blocks: ContentBlock[]) {
  return blocks
    .map((block) => {
      if (block.type === "text") return block.text;
      if (block.type === "code") {
        const language = block.language ? `${block.language}\n` : "";
        return `\`\`\`${language}${block.code}\n\`\`\``;
      }
      if (block.type === "link") return block.label ? `${block.label} (${block.url})` : block.url;
      if (block.type === "attachmentRef") return `[附件：${block.displayName}]`;
      if (block.type === "toolCallRef") return `[工具调用：${block.toolRunId}]`;
      if (block.type === "toolResultRef") return `[工具结果：${block.toolRunId}]`;
      return `[暂不支持的内容：${block.originalType}]`;
    })
    .join("\n");
}

const CANVAS_PREVIEW_SOURCE_LIMIT = 8_192;
export const CANVAS_PREVIEW_LENGTH = 280;
const MARKDOWN_PREVIEW_SIGNAL = /(?:^|\n)\s{0,3}(?:#{1,6}\s|>\s?|[-+*]\s|\d+[.)]\s|`{3}|~{3})|!\[|\[[^\]]+\]\(|<\/?[A-Za-z][^>]*>|[*_~`\\]/;

/**
 * Produces a bounded, non-HTML canvas preview without parsing a full Markdown AST.
 * Raw Markdown remains available on CanvasNodeProjection for the focused reader,
 * copying, export, and the shared safe renderer owned by the workspace layer.
 */
export function markdownToCanvasPreview(
  markdown: string,
  maxLength = CANVAS_PREVIEW_LENGTH,
) {
  if (maxLength <= 0) return "";

  const boundedSource = markdown.slice(0, CANVAS_PREVIEW_SOURCE_LIMIT);
  const plainText = (MARKDOWN_PREVIEW_SIGNAL.test(boundedSource) ? boundedSource
    .replace(/!\[([^\]]*)\]\([^\s)]+(?:\s+"[^"]*")?\)/g, "$1")
    .replace(/\[([^\]]+)\]\([^\s)]+(?:\s+"[^"]*")?\)/g, "$1")
    .replace(/<((?:https?:\/\/|mailto:)[^>]+)>/gi, "$1")
    .replace(/<[^>]*>/g, " ")
    .split(/\r?\n/)
    .map((line) => line
      .replace(/^\s{0,3}(?:#{1,6}\s+|>\s?|[-+*]\s+(?:\[[ xX]\]\s+)?|\d+[.)]\s+)/, "")
      .replace(/^\s{0,3}(?:`{3,}|~{3,})(?:\w+)?\s*$/, "")
      .trim())
    .filter(Boolean)
    .join(" ")
    .replace(/(`+)(.*?)\1/g, "$2")
    .replace(/(\*\*|__)(.*?)\1/g, "$2")
    .replace(/(\*|_)(.*?)\1/g, "$2")
    .replace(/~~(.*?)~~/g, "$1")
    .replace(/\\([\\`*{}[\]()#+\-.!_>])/g, "$1") : boundedSource)
    .replace(/\s+/g, " ")
    .trim();

  const characters = Array.from(plainText);
  if (characters.length <= maxLength) return plainText;
  return `${characters.slice(0, Math.max(0, maxLength - 1)).join("").trimEnd()}…`;
}

function calculateDepth(node: ConversationNode, nodesById: Map<string, ConversationNode>) {
  let depth = 0;
  let current = node;
  const visited = new Set<string>([node.id]);

  while (current.parentNodeId) {
    const parent = nodesById.get(current.parentNodeId);
    if (!parent || visited.has(parent.id)) break;
    visited.add(parent.id);
    depth += 1;
    current = parent;
  }

  return depth;
}

function calculateAutomaticPositions(graph: ConversationGraph) {
  const nodesById = new Map(graph.nodes.map((node) => [node.id, node]));
  const layers = new Map<number, ConversationNode[]>();

  for (const node of graph.nodes) {
    const depth = calculateDepth(node, nodesById);
    const layer = layers.get(depth) ?? [];
    layer.push(node);
    layers.set(depth, layer);
  }

  const positions = new Map<string, CanvasPoint>();
  for (const [depth, layer] of layers) {
    layer
      .sort((left, right) => left.createdAt.localeCompare(right.createdAt))
      .forEach((node, index) => {
        positions.set(node.id, {
          x: 112 + depth * 500,
          y: 116 + index * 330,
        });
      });
  }
  return positions;
}

export function projectConversationGraph(
  graph: ConversationGraph,
  localPositions: ReadonlyMap<string, CanvasPoint> = new Map(),
  modelRuns: readonly ModelRunProjection[] = [],
): CanvasGraphProjection {
  const persistedPositions = new Map(
    graph.positions.map((position) => [position.nodeId, { x: position.x, y: position.y }]),
  );
  const runsByNodeId = new Map<string, ModelRunProjection>();
  for (const run of modelRuns) {
    const current = runsByNodeId.get(run.nodeId);
    if (!current || current.updatedAt.localeCompare(run.updatedAt) <= 0) {
      runsByNodeId.set(run.nodeId, run);
    }
  }
  const automaticPositions = calculateAutomaticPositions(graph);

  return {
    nodes: graph.nodes.map((node) => {
      const run = runsByNodeId.get(node.id);
      const persistedAnswer = node.assistantMessage
        ? blocksToPlainText(node.assistantMessage.contentBlocks)
        : null;
      const question = blocksToPlainText(node.userMessage.contentBlocks);
      const answer = persistedAnswer ?? (run?.partialContent || null);
      const failedEvent = run?.terminalEvent?.type === "failed" ? run.terminalEvent : null;
      const cancelledEvent = run?.terminalEvent?.type === "cancelled" ? run.terminalEvent : null;
      const isImportedSource =
        node.branchType === "importedFrom" || node.userMessage.role === "imported";
      return {
        id: node.id,
        title: node.title,
        question,
        questionPreview: markdownToCanvasPreview(question),
        answer,
        answerPreview: answer ? markdownToCanvasPreview(answer) : null,
        providerId: run?.providerId ?? node.providerId,
        modelId: run?.modelId ?? node.modelId,
        runState: run?.state ?? node.runState,
        runError: failedEvent?.error ?? null,
        partialContentRetained:
          failedEvent?.partialContentRetained ?? cancelledEvent?.partialContentRetained ?? false,
        branchType: node.branchType,
        origin: isImportedSource ? { kind: "importedSource" } : { kind: "localRun" },
        parentNodeId: node.parentNodeId,
        createdAt: node.createdAt,
        position:
          localPositions.get(node.id) ??
          persistedPositions.get(node.id) ??
          automaticPositions.get(node.id) ??
          { x: 112, y: 116 },
      };
    }),
    edges: graph.edges.map((edge) => ({
      id: edge.id,
      sourceNodeId: edge.sourceNodeId,
      targetNodeId: edge.targetNodeId,
      relation: edge.relation,
    })),
  };
}

export function nextChildPosition(
  parent: CanvasNodeProjection | undefined,
  siblingCount: number,
): CanvasPoint {
  if (!parent) {
    return { x: 112, y: 116 + siblingCount * 330 };
  }

  return {
    x: parent.position.x + 500,
    y: parent.position.y + siblingCount * 176,
  };
}

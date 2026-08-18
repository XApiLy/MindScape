import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";
import test from "node:test";
import type {
  ConversationGraph,
  ConversationNode,
  ModelRunProjection,
} from "../domain/index.ts";
import {
  panCanvasViewport,
  zoomCanvasViewportAtPoint,
} from "./canvasViewport.ts";
import {
  DEFAULT_CANVAS_VIEWPORT,
  projectConversationGraph,
} from "./graphProjection.ts";

const timestamp = "2026-08-18T08:00:00.000Z";

function createNode(index: number): ConversationNode {
  const id = `node-${index.toString().padStart(3, "0")}`;
  const parentNodeId = index === 0 ? null : `node-${(index - 1).toString().padStart(3, "0")}`;
  return {
    id,
    conversationId: "conversation-benchmark",
    parentNodeId,
    branchType: index === 0 ? "continues" : index % 3 === 0 ? "diverges" : "continues",
    title: `200节点固定样本 ${index}`,
    userMessage: {
      id: `user-${id}`,
      conversationId: "conversation-benchmark",
      nodeId: id,
      role: "user",
      contentBlocks: [{ type: "text", text: `问题 ${index}：验证固定长度文本下的画布投影。` }],
      createdAt: timestamp,
    },
    assistantMessage: {
      id: `assistant-${id}`,
      conversationId: "conversation-benchmark",
      nodeId: id,
      role: "assistant",
      contentBlocks: [{
        type: "text",
        text: `回答 ${index}：这是用于200节点基准的稳定内容，不包含随机数据。`.repeat(4),
      }],
      createdAt: timestamp,
    },
    providerId: "mock",
    modelId: "mock-stream-v1",
    contextSnapshotId: `context-${id}`,
    runState: "completed",
    createdAt: new Date(Date.parse(timestamp) + index * 1000).toISOString(),
    updatedAt: timestamp,
    revision: 1,
  };
}

function createGraph(): ConversationGraph {
  const nodes = Array.from({ length: 200 }, (_, index) => createNode(index));
  return {
    conversation: {
      id: "conversation-benchmark",
      workspaceId: "workspace-benchmark",
      title: "200节点固定样本",
      createdAt: timestamp,
      updatedAt: timestamp,
      revision: 1,
    },
    nodes,
    edges: nodes.slice(1).map((node, index) => ({
      id: `edge-${index + 1}`,
      conversationId: "conversation-benchmark",
      sourceNodeId: nodes[index]!.id,
      targetNodeId: node.id,
      relation: node.branchType,
      createdAt: node.createdAt,
    })),
    positions: [],
  };
}

function elapsed(operation: () => void) {
  const startedAt = performance.now();
  operation();
  return performance.now() - startedAt;
}

test("records a repeatable 200-node canvas computation baseline", () => {
  const graph = createGraph();
  let projection = projectConversationGraph(graph);
  const initialProjectionMs = elapsed(() => {
    projection = projectConversationGraph(graph);
  });

  let viewport = { ...DEFAULT_CANVAS_VIEWPORT };
  const panMs = elapsed(() => {
    for (let index = 0; index < 5_000; index += 1) {
      viewport = panCanvasViewport(viewport, { x: 1, y: -1 });
    }
  });
  const zoomMs = elapsed(() => {
    for (let index = 0; index < 5_000; index += 1) {
      const requestedZoom = index % 2 === 0 ? viewport.zoom * 1.01 : viewport.zoom * 0.99;
      viewport = zoomCanvasViewportAtPoint(viewport, requestedZoom, { x: 640, y: 360 });
    }
  });

  let focusedEdgeCount = 0;
  const selectionMs = elapsed(() => {
    for (let index = 0; index < 1_000; index += 1) {
      const selectedNodeId = `node-${(index % 200).toString().padStart(3, "0")}`;
      focusedEdgeCount += projection.edges.filter(
        (edge) => edge.sourceNodeId === selectedNodeId || edge.targetNodeId === selectedNodeId,
      ).length;
    }
  });

  const streamingMs = elapsed(() => {
    for (let index = 0; index < 50; index += 1) {
      const run: ModelRunProjection = {
        runId: "run-benchmark",
        conversationId: graph.conversation.id,
        nodeId: "node-199",
        providerId: "mock",
        modelId: "mock-stream-v1",
        state: "streaming",
        lastSequence: index + 1,
        partialContent: "流式增量".repeat(index + 1),
        terminalEvent: null,
        updatedAt: new Date(Date.parse(timestamp) + index * 1000).toISOString(),
      };
      projection = projectConversationGraph(graph, new Map(), [run]);
    }
  });

  const result = {
    nodes: projection.nodes.length,
    edges: projection.edges.length,
    initialProjectionMs: Number(initialProjectionMs.toFixed(3)),
    pan5000Ms: Number(panMs.toFixed(3)),
    zoom5000Ms: Number(zoomMs.toFixed(3)),
    select1000Ms: Number(selectionMs.toFixed(3)),
    streaming50ProjectionMs: Number(streamingMs.toFixed(3)),
  };
  console.info("CAN-019 baseline", JSON.stringify(result));

  assert.equal(result.nodes, 200);
  assert.equal(result.edges, 199);
  assert.ok(focusedEdgeCount > 0);
  assert.ok(initialProjectionMs < 250, `initial projection took ${initialProjectionMs}ms`);
  assert.ok(panMs < 250, `pan computation took ${panMs}ms`);
  assert.ok(zoomMs < 250, `zoom computation took ${zoomMs}ms`);
  assert.ok(selectionMs < 500, `selection computation took ${selectionMs}ms`);
  assert.ok(streamingMs < 2_000, `streaming projections took ${streamingMs}ms`);
});

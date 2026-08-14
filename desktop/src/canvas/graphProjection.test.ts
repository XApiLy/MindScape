import assert from "node:assert/strict";
import test from "node:test";
import type { ConversationGraph, ConversationNode } from "../domain/index.ts";
import {
  nextChildPosition,
  projectConversationGraph,
  type CanvasNodeProjection,
} from "./graphProjection.ts";

const timestamp = "2026-08-14T10:00:00.000Z";

function node(
  id: string,
  parentNodeId: string | null,
  branchType: ConversationNode["branchType"],
): ConversationNode {
  return {
    id,
    conversationId: "conversation-1",
    parentNodeId,
    branchType,
    title: `节点 ${id}`,
    userMessage: {
      id: `user-${id}`,
      conversationId: "conversation-1",
      nodeId: id,
      role: "user",
      contentBlocks: [{ type: "text", text: `问题 ${id}` }],
      createdAt: timestamp,
    },
    assistantMessage: {
      id: `assistant-${id}`,
      conversationId: "conversation-1",
      nodeId: id,
      role: "assistant",
      contentBlocks: [{ type: "text", text: `回答 ${id}` }],
      createdAt: timestamp,
    },
    providerId: "mock",
    modelId: "mock-stream-v1",
    contextSnapshotId: `context-${id}`,
    runState: "completed",
    createdAt: timestamp,
    updatedAt: timestamp,
    revision: 1,
  };
}

function graph(): ConversationGraph {
  return {
    conversation: {
      id: "conversation-1",
      workspaceId: "workspace-1",
      title: "测试会话",
      createdAt: timestamp,
      updatedAt: timestamp,
      revision: 1,
    },
    nodes: [node("root", null, "continues"), node("child", "root", "deepens")],
    edges: [{
      id: "edge-1",
      conversationId: "conversation-1",
      sourceNodeId: "root",
      targetNodeId: "child",
      relation: "deepens",
      createdAt: timestamp,
    }],
    positions: [{ nodeId: "root", x: -120, y: 88 }],
  };
}

test("projects the domain graph without leaking domain objects", () => {
  const source = graph();
  const projection = projectConversationGraph(source);

  assert.deepEqual(projection.nodes[0]?.position, { x: -120, y: 88 });
  assert.deepEqual(projection.nodes[1]?.position, { x: 612, y: 116 });
  assert.equal(projection.nodes[1]?.question, "问题 child");
  assert.equal(projection.nodes[1]?.answer, "回答 child");
  assert.equal(projection.edges[0]?.relation, "deepens");
  assert.notStrictEqual(projection.nodes[0], source.nodes[0]);
  assert.equal("userMessage" in projection.nodes[0]!, false);
});

test("local drag positions override persisted and automatic positions", () => {
  const localPositions = new Map([["child", { x: 940, y: -40 }]]);
  const projection = projectConversationGraph(graph(), localPositions);

  assert.deepEqual(projection.nodes.find((item) => item.id === "child")?.position, {
    x: 940,
    y: -40,
  });
});

test("places a new child to the right of its parent", () => {
  const parent: CanvasNodeProjection = projectConversationGraph(graph()).nodes[0]!;

  assert.deepEqual(nextChildPosition(parent, 2), {
    x: parent.position.x + 500,
    y: parent.position.y + 352,
  });
  assert.deepEqual(nextChildPosition(undefined, 1), { x: 112, y: 446 });
});

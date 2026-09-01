import assert from "node:assert/strict";
import test from "node:test";
import type {
  ConversationGraph,
  ConversationNode,
  ModelRunProjection,
} from "../domain/index.ts";
import { APPLICATION_INTERRUPTED_PROVIDER_CODE } from "../domain/runtime.ts";
import {
  CANVAS_PREVIEW_LENGTH,
  markdownToCanvasPreview,
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
  assert.equal(projection.nodes[1]?.questionPreview, "问题 child");
  assert.equal(projection.nodes[1]?.answer, "回答 child");
  assert.equal(projection.nodes[1]?.answerPreview, "回答 child");
  assert.equal(projection.edges[0]?.relation, "deepens");
  assert.equal(projection.nodes[0]?.origin.kind, "localRun");
  assert.notStrictEqual(projection.nodes[0], source.nodes[0]);
  assert.equal("userMessage" in projection.nodes[0]!, false);
});

test("keeps raw Markdown for focused reading and projects a bounded plain-text card preview", () => {
  const source = graph();
  const markdown = [
    "# 交付计划",
    "",
    "> 先验证 **事实源**，再进入 [统一验收](https://example.com/release)。",
    "",
    "- [x] 保留 `raw markdown`",
    "- [ ] 不执行 <script>alert('x')</script> 内容",
    "",
    "```ts",
    "const result = verifyRelease();",
    "```",
    "",
    "后续说明".repeat(80),
  ].join("\n");
  source.nodes[1] = {
    ...source.nodes[1]!,
    assistantMessage: {
      ...source.nodes[1]!.assistantMessage!,
      contentBlocks: [{ type: "text", text: markdown }],
    },
  };

  const projected = projectConversationGraph(source).nodes[1]!;

  assert.equal(projected.answer, markdown, "raw Markdown must remain available to the reader");
  assert.match(projected.answerPreview!, /^交付计划 先验证 事实源，再进入 统一验收。/);
  assert.equal(projected.answerPreview!.includes("<script>"), false);
  assert.equal(projected.answerPreview!.includes("https://example.com"), false);
  assert.equal(Array.from(projected.answerPreview!).length, CANVAS_PREVIEW_LENGTH);
  assert.equal(projected.answerPreview!.endsWith("…"), true);
});

test("bounds preview work for long streaming Markdown and keeps Unicode intact", () => {
  const preview = markdownToCanvasPreview(`## 标题\n${"🧠结构化内容 ".repeat(2_000)}`, 42);

  assert.equal(Array.from(preview).length, 42);
  assert.equal(preview.startsWith("标题 🧠结构化内容"), true);
  assert.equal(preview.endsWith("…"), true);
  assert.equal(preview.includes("�"), false);
});

test("marks explicit imported domain nodes without changing graph semantics", () => {
  const relationSource = graph();
  relationSource.nodes[1] = {
    ...relationSource.nodes[1]!,
    branchType: "importedFrom",
  };
  relationSource.edges[0] = {
    ...relationSource.edges[0]!,
    relation: "importedFrom",
  };

  const projection = projectConversationGraph(relationSource);
  const importedNode = projection.nodes.find((item) => item.id === "child");

  assert.equal(importedNode?.origin.kind, "importedSource");
  assert.equal(importedNode?.branchType, "importedFrom");
  assert.deepEqual(importedNode?.position, { x: 612, y: 116 });
  assert.equal(projection.edges[0]?.relation, "importedFrom");
  assert.equal(projection.nodes.length, 2);
  assert.equal(projection.edges.length, 1);

  const messageSource = graph();
  messageSource.nodes[1] = {
    ...messageSource.nodes[1]!,
    userMessage: {
      ...messageSource.nodes[1]!.userMessage,
      role: "imported",
    },
  };
  assert.equal(
    projectConversationGraph(messageSource).nodes.find((item) => item.id === "child")?.origin.kind,
    "importedSource",
  );
});

test("local drag positions override persisted and automatic positions", () => {
  const localPositions = new Map([["child", { x: 940, y: -40 }]]);
  const projection = projectConversationGraph(graph(), localPositions);

  assert.deepEqual(projection.nodes.find((item) => item.id === "child")?.position, {
    x: 940,
    y: -40,
  });
});

test("merges the latest persisted run projection by stable node id", () => {
  const source = graph();
  source.nodes[1] = {
    ...source.nodes[1]!,
    assistantMessage: null,
    runState: "pending",
  };
  const olderRun: ModelRunProjection = {
    runId: "run-old",
    conversationId: "conversation-1",
    nodeId: "child",
    providerId: "mock",
    modelId: "mock-stream-v1",
    state: "streaming",
    lastSequence: 2,
    partialContent: "旧的部分内容",
    terminalEvent: null,
    updatedAt: "2026-08-14T10:00:01.000Z",
  };
  const latestRun: ModelRunProjection = {
    ...olderRun,
    runId: "run-latest",
    state: "failed",
    lastSequence: 3,
    partialContent: "重启后保留的部分内容",
    updatedAt: "2026-08-14T10:00:02.000Z",
  };

  const projection = projectConversationGraph(source, new Map(), [latestRun, olderRun]);
  const child = projection.nodes.find((item) => item.id === "child");

  assert.equal(child?.runState, "failed");
  assert.equal(child?.answer, "重启后保留的部分内容");
  assert.equal(child?.modelId, "mock-stream-v1");
  assert.equal(projection.nodes.length, source.nodes.length, "run merge must not create a duplicate card");
});

test("keeps every node position stable while partial output grows", () => {
  const source = graph();
  source.nodes[1] = {
    ...source.nodes[1]!,
    assistantMessage: null,
    runState: "streaming",
  };
  const run: ModelRunProjection = {
    runId: "run-streaming",
    conversationId: "conversation-1",
    nodeId: "child",
    providerId: "mock",
    modelId: "mock-stream-v1",
    state: "streaming",
    lastSequence: 2,
    partialContent: "短内容",
    terminalEvent: null,
    updatedAt: "2026-08-14T10:00:01.000Z",
  };

  const before = projectConversationGraph(source, new Map(), [run]);
  const after = projectConversationGraph(source, new Map(), [{
    ...run,
    lastSequence: 100,
    partialContent: "持续增长的流式内容".repeat(2_000),
    updatedAt: "2026-08-14T10:00:02.000Z",
  }]);

  assert.deepEqual(
    after.nodes.map((item) => ({ id: item.id, position: item.position })),
    before.nodes.map((item) => ({ id: item.id, position: item.position })),
  );
  assert.deepEqual(after.edges, before.edges);
});

test("projects every frozen terminal state from the shared run snapshot", () => {
  const source = graph();
  source.nodes[1] = {
    ...source.nodes[1]!,
    assistantMessage: null,
    providerId: null,
    modelId: null,
    runState: "pending",
  };
  const baseRun: ModelRunProjection = {
    runId: "run-terminal",
    conversationId: "conversation-1",
    nodeId: "child",
    providerId: "deepseek",
    modelId: "deepseek-v4-flash",
    state: "completed",
    lastSequence: 4,
    partialContent: "终态保留内容",
    terminalEvent: {
      type: "completed",
      finishReason: "stop",
      usage: {
        inputTokens: 10,
        outputTokens: 20,
        cachedInputTokens: null,
        costMicrounits: null,
      },
    },
    updatedAt: "2026-08-14T10:00:04.000Z",
  };
  const cases: Array<{
    name: string;
    run: ModelRunProjection;
    expectedErrorCode: string | null;
    partialContentRetained: boolean;
  }> = [
    {
      name: "completed",
      run: baseRun,
      expectedErrorCode: null,
      partialContentRetained: false,
    },
    {
      name: "cancelled",
      run: {
        ...baseRun,
        state: "cancelled",
        terminalEvent: {
          type: "cancelled",
          reason: "userRequested",
          partialContentRetained: true,
        },
      },
      expectedErrorCode: null,
      partialContentRetained: true,
    },
    {
      name: "provider failed",
      run: {
        ...baseRun,
        state: "failed",
        terminalEvent: {
          type: "failed",
          error: {
            category: "network",
            providerCode: "connection_reset",
            safeMessage: "无法连接模型服务。",
            retryable: true,
            retryAfterMs: null,
            providerStatus: null,
          },
          partialContentRetained: true,
        },
      },
      expectedErrorCode: "connection_reset",
      partialContentRetained: true,
    },
    {
      name: "application interrupted",
      run: {
        ...baseRun,
        state: "failed",
        terminalEvent: {
          type: "failed",
          error: {
            category: "unknown",
            providerCode: APPLICATION_INTERRUPTED_PROVIDER_CODE,
            safeMessage: "应用上次退出时中断了模型运行，已保留收到的内容。",
            retryable: true,
            retryAfterMs: null,
            providerStatus: null,
          },
          partialContentRetained: true,
        },
      },
      expectedErrorCode: APPLICATION_INTERRUPTED_PROVIDER_CODE,
      partialContentRetained: true,
    },
  ];

  for (const terminalCase of cases) {
    const projection = projectConversationGraph(source, new Map(), [terminalCase.run]);
    const child = projection.nodes.find((item) => item.id === "child");

    assert.equal(child?.runState, terminalCase.run.state, terminalCase.name);
    assert.equal(child?.answer, "终态保留内容", terminalCase.name);
    assert.equal(child?.providerId, "deepseek", terminalCase.name);
    assert.equal(child?.modelId, "deepseek-v4-flash", terminalCase.name);
    assert.equal(child?.runError?.providerCode ?? null, terminalCase.expectedErrorCode, terminalCase.name);
    assert.equal(
      child?.partialContentRetained,
      terminalCase.partialContentRetained,
      terminalCase.name,
    );
    assert.equal(projection.nodes.length, source.nodes.length, terminalCase.name);
  }
});

test("places a new child to the right of its parent", () => {
  const parent: CanvasNodeProjection = projectConversationGraph(graph()).nodes[0]!;

  assert.deepEqual(nextChildPosition(parent, 2), {
    x: parent.position.x + 500,
    y: parent.position.y + 352,
  });
  assert.deepEqual(nextChildPosition(undefined, 1), { x: 112, y: 446 });
});

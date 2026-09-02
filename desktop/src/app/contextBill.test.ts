import assert from "node:assert/strict";
import test from "node:test";
import type { ContextSnapshot, EvidenceRef } from "../domain/index.ts";
import { presentEvidenceRef, projectContextBill } from "./contextBill.ts";

const importEvidence: EvidenceRef = {
  id: "evidence-import-1",
  target: {
    type: "importContent",
    importSourceId: "source-1",
    importRevisionId: "revision-1",
    locator: "line:8-12",
  },
  contentHash: "hash-1",
  excerpt: "外部会话中的原始结论",
  createdAt: "2026-08-24T10:00:00.000Z",
};

const snapshot: ContextSnapshot = {
  id: "snapshot-1",
  conversationId: "conversation-1",
  parentNodeId: "node-1",
  branchType: "deepens",
  currentInput: "继续核对这个结论",
  selectedMessages: [
    {
      messageId: "message-1",
      role: "assistant",
      contentBlocks: [{ type: "text", text: "已冻结的历史回答" }],
      sourceNodeId: "node-1",
    },
  ],
  selectedImportRefs: [importEvidence],
  explicitConstraints: [
    { text: "只使用本地证据", evidence: [importEvidence], userConfirmed: true },
  ],
  omittedMessages: [{ messageId: "message-0", reason: "不在当前路径" }],
  systemContractVersion: "mindscape.context.v1",
  estimatedTokens: 384,
  createdAt: "2026-08-24T10:01:00.000Z",
};

test("projects an auditable context bill without inventing run budget values", () => {
  const bill = projectContextBill(snapshot);
  assert.deepEqual(bill.metrics, {
    messages: 1,
    importSources: 1,
    constraints: 1,
    omitted: 1,
  });
  assert.equal(bill.branchLabel, "深入当前节点");
  assert.equal(bill.messages[0].roleLabel, "助手消息");
  assert.equal(bill.importSources[0].targetLabel, "source-1 · line:8-12");
  assert.equal(bill.constraints[0].userConfirmed, true);
  assert.match(bill.budgetNotice, /不包含输出、费用或超时预算/);
});

test("keeps an empty frozen snapshot empty instead of synthesizing sources", () => {
  const bill = projectContextBill({
    ...snapshot,
    selectedMessages: [],
    selectedImportRefs: [],
    explicitConstraints: [],
    omittedMessages: [],
    estimatedTokens: 0,
  });
  assert.deepEqual(bill.metrics, { messages: 0, importSources: 0, constraints: 0, omitted: 0 });
  assert.deepEqual(bill.importSources, []);
  assert.equal(bill.estimatedTokens, 0);
});

test("labels every frozen evidence target explicitly", () => {
  const evidence = [
    importEvidence,
    { ...importEvidence, id: "message", target: { type: "messageBlock" as const, messageId: "m-1", contentBlockIndex: 0 } },
    { ...importEvidence, id: "attachment", target: { type: "attachmentContent" as const, attachmentId: "a-1", locator: "page:2" } },
    { ...importEvidence, id: "tool", target: { type: "toolResultBlock" as const, toolRunId: "tool-1", contentBlockIndex: 1 } },
  ];
  assert.deepEqual(evidence.map(presentEvidenceRef).map(({ sourceKind }) => sourceKind), [
    "导入原文",
    "会话消息",
    "本地附件",
    "工具结果",
  ]);
});

import type {
  BranchType,
  ContentBlock,
  ContextSnapshot,
  EvidenceRef,
  MessageRole,
} from "../domain";

export type ContextBillEvidence = {
  id: string;
  sourceKind: string;
  targetLabel: string;
  excerpt: string | null;
};

export type ContextBillProjection = {
  protocolVersion: string;
  branchLabel: string;
  estimatedTokens: number;
  currentInput: string;
  metrics: {
    messages: number;
    importSources: number;
    constraints: number;
    omitted: number;
  };
  messages: Array<{
    id: string;
    roleLabel: string;
    sourceNodeId: string;
    text: string;
  }>;
  importSources: ContextBillEvidence[];
  constraints: Array<{
    id: string;
    text: string;
    userConfirmed: boolean;
    evidenceCount: number;
  }>;
  omitted: Array<{ messageId: string; reason: string }>;
  budgetNotice: string;
};

const branchLabels: Record<BranchType, string> = {
  continues: "继续当前路径",
  deepens: "深入当前节点",
  diverges: "从节点发散",
  reframes: "换角度重构",
  importedFrom: "导入原文来源",
};

const roleLabels: Record<MessageRole, string> = {
  system: "系统约束",
  user: "用户消息",
  assistant: "助手消息",
  imported: "导入原文",
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

export function presentEvidenceRef(evidence: EvidenceRef): ContextBillEvidence {
  const target = evidence.target;
  if (target.type === "importContent") {
    return {
      id: evidence.id,
      sourceKind: "导入原文",
      targetLabel: `${target.importSourceId} · ${target.locator}`,
      excerpt: evidence.excerpt,
    };
  }
  if (target.type === "messageBlock") {
    return {
      id: evidence.id,
      sourceKind: "会话消息",
      targetLabel: `${target.messageId} · 内容块 ${target.contentBlockIndex + 1}`,
      excerpt: evidence.excerpt,
    };
  }
  if (target.type === "attachmentContent") {
    return {
      id: evidence.id,
      sourceKind: "本地附件",
      targetLabel: target.locator ? `${target.attachmentId} · ${target.locator}` : target.attachmentId,
      excerpt: evidence.excerpt,
    };
  }
  return {
    id: evidence.id,
    sourceKind: "工具结果",
    targetLabel: `${target.toolRunId} · 内容块 ${target.contentBlockIndex + 1}`,
    excerpt: evidence.excerpt,
  };
}

export function projectContextBill(snapshot: ContextSnapshot): ContextBillProjection {
  return {
    protocolVersion: snapshot.systemContractVersion,
    branchLabel: branchLabels[snapshot.branchType],
    estimatedTokens: snapshot.estimatedTokens,
    currentInput: snapshot.currentInput,
    metrics: {
      messages: snapshot.selectedMessages.length,
      importSources: snapshot.selectedImportRefs.length,
      constraints: snapshot.explicitConstraints.length,
      omitted: snapshot.omittedMessages.length,
    },
    messages: snapshot.selectedMessages.map((message) => ({
      id: message.messageId,
      roleLabel: roleLabels[message.role],
      sourceNodeId: message.sourceNodeId,
      text: blocksToPlainText(message.contentBlocks),
    })),
    importSources: snapshot.selectedImportRefs.map(presentEvidenceRef),
    constraints: snapshot.explicitConstraints.map((constraint, index) => ({
      id: `${snapshot.id}-constraint-${index}`,
      text: constraint.text,
      userConfirmed: constraint.userConfirmed,
      evidenceCount: constraint.evidence.length,
    })),
    omitted: snapshot.omittedMessages.map((message) => ({ ...message })),
    budgetNotice: "当前 ContextSnapshot 不包含输出、费用或超时预算；等待 Effective Run Profile 接线后才能展示实际预算。",
  };
}

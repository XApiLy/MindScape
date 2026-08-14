import type { ConversationEdge, ConversationNode, ProjectItem } from "../types/workspace";

export const demoProjects: ProjectItem[] = [
  {
    id: "project-ms",
    title: "MindScape 产品设计",
    count: 3,
    conversations: [
      { id: "conv-core", title: "AI 会话与画布架构", updatedAt: "刚刚" },
      { id: "conv-import", title: "外部会话导入策略", updatedAt: "18 分钟前" },
    ],
  },
  {
    id: "project-learning",
    title: "认知工具与学习探索",
    count: 2,
    conversations: [{ id: "conv-tools", title: "透明推理与工具轨迹", updatedAt: "昨天" }],
  },
];

export const demoNodes: ConversationNode[] = [
  {
    id: "node-root",
    type: "conversation",
    position: { x: 80, y: 190 },
    data: {
      title: "构建 MindScape 第一版",
      prompt: "我们第一版应该先实现哪些核心体验？",
      content:
        "第一版聚焦四个部分：**无限会话画布、外部会话导入、多厂商模型接口、基础 Chat**。\n\n画布负责承载探索过程，导入器负责接续过去的工作，Provider 层保持模型无关。",
      model: "Gemini 3.1 Pro",
      createdAt: "10:42",
      tags: ["MVP", "产品边界"],
      branchKind: "main",
      status: "ready",
      reasoningLabel: "已综合 7 条产品决策",
    },
  },
  {
    id: "node-canvas",
    type: "conversation",
    position: { x: 530, y: 70 },
    data: {
      title: "无限画布：对话成为可操作节点",
      prompt: "画布如何避免沦为普通思维导图？",
      content:
        "每张卡片不是静态笔记，而是一个可继续提问的会话上下文。用户可以从任意节点执行：\n\n- **深入**：沿当前问题继续下钻\n- **发散**：创建同层的新视角\n- **换角度**：保留背景，重写当前问题\n\n所有分支保持来源和上下文关系。",
      model: "Claude Sonnet",
      createdAt: "10:46",
      tags: ["画布", "分支"],
      branchKind: "deep",
      status: "ready",
      reasoningLabel: "使用：会话结构 Skill",
    },
  },
  {
    id: "node-import",
    type: "conversation",
    position: { x: 560, y: 500 },
    data: {
      title: "外部会话：原文轨与接续轨",
      prompt: "导入 Claude 或 Codex 会话后怎样保持连续性？",
      content:
        "采用双轨结构：\n\n1. **原文轨**完整保存消息、附件和工具记录。\n2. **接续轨**提取当前目标、约束、决策与未解决问题。\n\n分析层可删除、可重建，永远不覆盖原始会话。",
      model: "GPT-5.6",
      createdAt: "10:51",
      tags: ["导入器", "上下文"],
      branchKind: "parallel",
      status: "ready",
      reasoningLabel: "引用 4 项导入原则",
    },
  },
  {
    id: "node-provider",
    type: "conversation",
    position: { x: 1030, y: 250 },
    data: {
      title: "模型层：统一 Provider 接口",
      prompt: "如何同时接入不同厂商模型？",
      content:
        "应用只依赖统一的流式事件协议。各厂商适配器负责转换消息、SSE 数据和错误结构。\n\n首批支持 OpenAI-compatible、Anthropic 和 Gemini；DeepSeek、OpenRouter 与自定义服务复用兼容接口。",
      model: "DeepSeek V3",
      createdAt: "10:55",
      tags: ["Provider", "API"],
      branchKind: "deep",
      status: "ready",
      reasoningLabel: "已比较 3 种 API 事件格式",
    },
  },
];

export const demoEdges: ConversationEdge[] = [
  {
    id: "edge-root-canvas",
    source: "node-root",
    target: "node-canvas",
    type: "smoothstep",
    animated: false,
    style: { stroke: "#79b79b", strokeWidth: 1.5 },
  },
  {
    id: "edge-root-import",
    source: "node-root",
    target: "node-import",
    type: "smoothstep",
    style: { stroke: "#b8a57f", strokeWidth: 1.5 },
  },
  {
    id: "edge-canvas-provider",
    source: "node-canvas",
    target: "node-provider",
    type: "smoothstep",
    style: { stroke: "#8f86b3", strokeWidth: 1.5 },
  },
];

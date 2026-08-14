import { CanvasNode, Conversation, Edge, HighlightItem, Project } from './types'

// ─────────────────────────────────────────────────────────────
// 会话一：DeepSeek 与 Claude 架构选型(含主线 + 深挖/发散/换角分支)
// ─────────────────────────────────────────────────────────────
const CONV_1_NODES: CanvasNode[] = [
  {
    id: 'Node-01',
    type: 'question',
    title: '用户提问: DeepSeek-V4 与 Claude-3.5 架构区别与选择',
    content: `我想构建一个企业级 LLM 知识库与复杂任务智能体，如何在 **DeepSeek-V4** 与 **Claude-3.5 Sonnet** 之间做出技术选型？重点关注 MoE 架构、推理解析速度与长文本 RAG 能力。`,
    x: 80,
    y: 180,
    width: 460,
    timestamp: '10:42 AM',
    isMainline: true,
  },
  {
    id: 'Node-02',
    type: 'ai_response',
    title: 'DeepSeek-V4 AI 响应',
    model: 'DeepSeek-V4',
    parentId: 'Node-01',
    isMainline: true,
    content: `针对企业级 AI 智能体与 RAG 系统选型，建议从以下 **三大核心技术维度** 进行权衡评估：

### 1. MoE 混合专家架构与推理成本 (Inference Efficiency)
- **DeepSeek-V4**: 采用 MLA (Multi-head Latent Attention) 极低显存占用机制，搭配 DeepSeekMoE 的细粒度专家激活，单位 Token 成本仅为传统 Dense 模型的 1/6。
- **Claude 3.5 Sonnet**: 在复杂逻辑推演、代码重构及 JSON Structured Output 上具有极高的零样本成功率。

### 2. 知识库 RAG 与长上下文注意力机制
\`\`\`python
# DeepSeek MLA 核心计算示意 (KV Cache 压缩)
def compute_mla_attention(q, compressed_kv, w_uk, w_uv):
    # 压缩 KV 向量映射至高维空间，大幅降低显存开销
    k = torch.matmul(compressed_kv, w_uk)
    v = torch.matmul(compressed_kv, w_uv)
    scores = torch.matmul(q, k.transpose(-1, -2)) / (q.size(-1) ** 0.5)
    return torch.matmul(torch.softmax(scores, dim=-1), v)
\`\`\`

### 3. 推荐最佳组合策略 (Hybrid Architecture)
构建 **Dual-Engine 引擎**: 前置网关使用 DeepSeek 进行意图路由与初筛，核心复杂 Agent 思考推演挂载 Claude-3.5。`,
    x: 600,
    y: 100,
    width: 520,
    timestamp: '10:43 AM',
    tags: ['架构选型', 'MoE', 'RAG'],
  },
  {
    id: 'Node-03',
    type: 'sub_card',
    title: '↗ 深挖子卡片: MLA 显存优化原理',
    model: 'DeepSeek-V4',
    parentId: 'Node-02',
    content: `### 为什么 Multi-head Latent Attention (MLA) 能减少 90% KV Cache？

传统 Multi-head Attention 在 128k 上下文下 KV Cache 显存爆炸。MLA 通过 **低秩向量投影 (Low-rank Joint Compression)**：

1. 将盲目膨胀的 Key/Value 矩阵压缩为一个极小的隐向量 $c_t^{KV}$。
2. 推理阶段只需将 $c_t^{KV}$ 存在 GPU HBM 中，计算时动态解压。
3. **SOP 落地建议**: 在部署 vLLM / SGLang 时开启 \`--enable-mla-kv-cache\` 参数。`,
    x: 1200,
    y: 40,
    width: 480,
    timestamp: '10:45 AM',
    tags: ['深挖子节点', '显存优化'],
  },
  {
    id: 'Node-04',
    type: 'divergent_card',
    title: '→ 发散卡片: 跨模型 Agent 双引擎路由图谱',
    model: 'Claude-3.5-Sonnet',
    parentId: 'Node-02',
    content: `### 双模型协作 Agent 路由逻辑：

1. **Gate Router (Fast Pass)**:
   - 简单查询/检索总结 $\\rightarrow$ **DeepSeek-V4** (高吞吐、低延迟)
2. **Deep Reasoning Pass**:
   - 算法编写/代码审查/多步工具调用 $\\rightarrow$ **Claude-3.5 Sonnet**
3. **Failover 熔断机制**:
   - 若 API 响应时间 $> 1200\\text{ms}$，自动回退并降级执行。`,
    x: 1200,
    y: 420,
    width: 480,
    timestamp: '10:47 AM',
    tags: ['平行发散', 'Agent 路由'],
  },
  {
    id: 'Node-05',
    type: 'branch_card',
    title: '↓ 换角分支: 从隐私合规与私有化部署角度看选型',
    model: 'Qwen-2.5-Max',
    parentId: 'Node-02',
    content: `### 换角提问：如果企业对数据出境与金融级合规有硬性要求？

- **开源私有化**: DeepSeek-V4 / Qwen-2.5 72B 权重开源，可直接部署于本地 vLLM 集群，彻底避免数据泄露风控。
- **专有云 SaaS**: Claude 需通过 AWS Bedrock / GCP Vertex AI 专区部署，满足 HIPAA/SOC2 认证。`,
    x: 600,
    y: 660,
    width: 500,
    timestamp: '10:50 AM',
    tags: ['分支讨论', '私有化部署'],
  },
  {
    id: 'Node-06',
    type: 'ai_response',
    title: '主线推进: 落地 Dual-Engine 网关最小实现',
    model: 'DeepSeek-V4',
    parentId: 'Node-02',
    isMainline: true,
    content: `回到主线，我们把上面的选型结论落地为一个可运行的网关雏形：

### 意图路由 + 熔断的最小骨架
\`\`\`typescript
async function routeQuery(q: Query) {
  const intent = await deepseek.classify(q)
  return intent.complexity > 0.7
    ? claude.reason(q)      // 深度推理走 Claude
    : deepseek.answer(q)    // 高吞吐走 DeepSeek
}
\`\`\`

下一步可继续沿主线推进：接入 RAG 检索层与可观测性埋点。`,
    x: 1100,
    y: 180,
    width: 520,
    timestamp: '10:53 AM',
    tags: ['主线推进', 'Dual-Engine'],
  },
]

const CONV_1_EDGES: Edge[] = [
  { id: 'edge-1-2', source: 'Node-01', target: 'Node-02', type: 'sub' },
  { id: 'edge-2-3', source: 'Node-02', target: 'Node-03', type: 'sub', label: '↗ 深挖子卡片' },
  { id: 'edge-2-4', source: 'Node-02', target: 'Node-04', type: 'divergent', label: '→ 平行发散' },
  { id: 'edge-2-5', source: 'Node-02', target: 'Node-05', type: 'branch', label: '↓ 换角分支' },
  { id: 'edge-2-6', source: 'Node-02', target: 'Node-06', type: 'sub', label: '主线推进' },
]

const CONV_1_HIGHLIGHTS: HighlightItem[] = [
  {
    id: 'hl-1',
    nodeId: 'Node-02',
    type: 'amber',
    text: 'MoE 搭配 MLA (Multi-head Latent Attention) 极低显存占用机制，单位 Token 成本仅为传统 Dense 模型的 1/6。',
    note: '关键成本收益对比',
    timestamp: '10:44 AM',
  },
  {
    id: 'hl-2',
    nodeId: 'Node-03',
    type: 'emerald',
    text: '在部署 vLLM / SGLang 时开启 --enable-mla-kv-cache 参数以开启低秩隐向量压缩。',
    note: 'SOP 部署指令',
    timestamp: '10:46 AM',
  },
  {
    id: 'hl-3',
    nodeId: 'Node-04',
    type: 'coral',
    text: '需要为 Agent 路由接入 API 熔断机制：若响应时间 > 1200ms 则触发降级。',
    note: '架构待办点',
    timestamp: '10:48 AM',
  },
]

// ─────────────────────────────────────────────────────────────
// 会话种子工厂：为新对话/其它示例生成起点卡片
// ─────────────────────────────────────────────────────────────
export function makeSeedConversation(
  id: string,
  name: string,
  updatedAt: string,
  rootTitle: string,
  rootContent: string,
): Conversation {
  const rootId = `${id}-Node-01`
  return {
    id,
    name,
    updatedAt,
    mainlineRootId: rootId,
    currentNodeId: rootId,
    nodes: [
      {
        id: rootId,
        type: 'question',
        title: rootTitle,
        content: rootContent,
        x: 120,
        y: 200,
        width: 480,
        timestamp: '刚刚',
        isMainline: true,
      },
    ],
    edges: [],
    highlights: [],
  }
}

const CONV_1: Conversation = {
  id: 'conv-1',
  name: 'DeepSeek vs Claude 架构选型',
  updatedAt: '10 分钟前',
  nodes: CONV_1_NODES,
  edges: CONV_1_EDGES,
  highlights: CONV_1_HIGHLIGHTS,
  mainlineRootId: 'Node-01',
  currentNodeId: 'Node-02',
}

export const INITIAL_PROJECTS: Project[] = [
  {
    id: 'proj-1',
    name: '大模型架构设计与微调策略',
    description: '混合大模型部署、长文本注意力机制优化与 RAG 工作流',
    updatedAt: '10 分钟前',
    conversations: [
      CONV_1,
      makeSeedConversation(
        'conv-2',
        'RAG 检索召回率调优',
        '1 小时前',
        '会话: 如何提升企业知识库 RAG 的召回率？',
        '当前向量检索在长尾问题上召回不足，我想系统梳理 chunking 策略、rerank 与 hybrid search 的组合方案。',
      ),
    ],
  },
  {
    id: 'proj-2',
    name: 'Next.js 15 & React 19 全栈架构',
    description: 'Server Actions, Selective Hydration 与 Edge Cache 方案',
    updatedAt: '2 小时前',
    conversations: [
      makeSeedConversation(
        'conv-3',
        'Server Actions 数据流设计',
        '2 小时前',
        '会话: React 19 下 Server Actions 的最佳数据流',
        '想搞清楚 Server Actions + useOptimistic + Suspense 的协作模式，以及何时该回退到传统 API Route。',
      ),
    ],
  },
  {
    id: 'proj-3',
    name: 'WebGL & Liquid Glass 渲染引擎',
    description: '自定义 Shader、Glassmorphism & Ray-marching 白皮书',
    updatedAt: '昨天',
    conversations: [
      makeSeedConversation(
        'conv-4',
        '液态玻璃折射 Shader',
        '昨天',
        '会话: 如何在 WebGL 里实现液态玻璃折射？',
        '目标是复刻 Liquid Glass 的实时折射与高光，需要梳理法线扰动、环境采样与模糊叠加的管线。',
      ),
    ],
  },
]

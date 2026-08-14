# MindScape 核心算法与实现路线

> 状态：首版技术方案  
> 最近更新：2026-08-13

## 1. 我们实际要实现什么

核心算法不是一个“自动选择 Skill 的分类器”，而是一个持续运行的认知控制回路：

```text
用户消息 / 外部会话 / 文件 / 工具结果
                    ↓
              统一事件流
                    ↓
              当前任务模型
                    ↓
         问题、缺口与候选行动
                    ↓
       分析预算与能力组合决策
                    ↓
        上下文包 + 主模型 + 工具
                    ↓
          成果验证与状态更新
```

它要稳定回答五个问题：

1. 用户现在真正要推进什么？
2. 当前阻碍是什么，缺少什么？
3. 哪些信息值得进一步分析？
4. 应该挂载哪些能力，最小集合是什么？
5. 本轮结果改变了哪些事实、决策、任务和下一步？

## 2. 不追求虚假的绝对正确

“无懈可击”不能建立在模型永远正确上，而要建立在系统约束上：

- 原文永远保留，派生理解不覆盖原文。
- 用户明确表达的内容与系统推断分开存储。
- 每项推断包含证据、置信度、生成器版本和时间。
- 低置信度判断只能建议，不能静默改变关键行为。
- 权限、费用上限和外部副作用由确定性代码控制。
- 能力变化、上下文选择和模型调用可以完整回放。
- 用户纠正后能够重算状态，而不是把错误继续传递。

可靠性来自可验证与可恢复，不来自假装模型不会错。

## 3. 七个核心模块

### 3.1 事件标准化器 Event Normalizer

将用户消息、导入会话、附件、工具调用、文件修改和用户纠正统一成不可变事件。

```ts
type WorkspaceEvent = {
  id: string
  kind: 'message' | 'attachment' | 'tool_call' | 'tool_result' |
        'file_change' | 'user_correction' | 'capability_change'
  source: { platform: string; conversationId?: string; locator?: string }
  actor: 'user' | 'assistant' | 'tool' | 'system'
  content: ContentBlock[]
  occurredAt: string
  parentIds: string[]
  integrityHash: string
}
```

这一层只做解析和校验，不解释意图。

### 3.2 任务状态归并器 Task State Reducer

根据新事件增量更新任务模型，避免每一轮重新分析全部历史。

```ts
type TaskState = {
  goal: Claim | null
  stage: Claim<'explore' | 'define' | 'compare' | 'plan' |
               'execute' | 'verify' | 'synthesize'>
  deliverables: Claim[]
  constraints: Claim[]
  facts: Claim[]
  hypotheses: Claim[]
  openQuestions: Claim[]
  decisions: Claim[]
  activeWorkItem: Claim | null
  relevantResources: EvidenceRef[]
  activeCapabilities: CapabilityBinding[]
  version: number
}

type Claim = {
  value: unknown
  status: 'explicit' | 'inferred' | 'confirmed' | 'rejected' | 'superseded'
  confidence: number
  evidence: EvidenceRef[]
  producedBy: string
}
```

更新器只输出状态差异：新增、修改、冲突和失效项。规则层随后检查：

- 推断是否存在原文证据。
- 是否与用户明确约束冲突。
- 新目标是替代旧目标，还是形成子任务。
- 置信度是否足以自动应用。

### 3.3 缺口诊断器 Gap Detector

从任务状态中找出真正阻碍推进的问题，输出有限的候选缺口：

```ts
type Gap = {
  kind: 'missing_information' | 'ambiguity' | 'contradiction' |
        'missing_capability' | 'unverified_assumption' | 'blocked_action'
  description: string
  affectedGoal: string
  evidence: EvidenceRef[]
  confidence: number
  proposedResolution: string
}
```

优先级不是“提到了多少次”，而是该缺口是否阻断下一步或可能造成高代价错误。

### 3.4 分析门控器 Analysis Gate

对资料片段或候选问题计算两个核心指标。分数必须由可解释的子信号组成，不能只要求模型输出 `0.87`。

目标相关度 `R` 的首版子信号：

- `goal_match`：与明确目标的语义关系。
- `work_item_match`：与当前子任务的关系。
- `dependency`：是否是推进下一步的前置信息。
- `deliverable_effect`：是否影响预期成果。
- `evidence_directness`：是直接证据还是仅主题相似。

决策价值 `D` 的首版子信号：

- `impact`：若判断变化，对结果影响多大。
- `uncertainty`：当前状态是否确实未知或存在冲突。
- `actionability`：分析后是否能采取不同动作。
- `risk_reduction`：是否能避免显著错误或遗漏。
- `capability_effect`：是否会改变能力或工具选择。

模型负责给出各子信号、理由和证据；程序负责验证字段、校准分数和执行阈值。初始优先级可使用：

```text
analysis_value = R × D
```

预算分配再考虑成本：

```text
priority = analysis_value / normalized_cost
```

成本不能把高风险信息永久压到最低；安全和重大损失类内容设置独立硬门槛。

第一版不应凭感觉固定复杂权重。先采用等权或简单规则产生基线，再依据标注集与用户纠正校准。

### 3.5 能力检索与规划器 Capability Planner

每个 Skill、MCP 和工具使用统一能力清单：

```ts
type CapabilityManifest = {
  id: string
  version: string
  kind: 'cognitive' | 'read' | 'execute'
  description: string
  solves: string[]
  requires: string[]
  produces: string[]
  exclusions: string[]
  conflictsWith: string[]
  tokenCost: number
  latencyClass: 'local' | 'fast' | 'slow'
  permission: 'none' | 'read' | 'write' | 'external_side_effect'
  evalSuite: string[]
}
```

选择分两步：

1. **候选检索**：通过结构标签、全文和向量检索召回少量能力。
2. **组合规划**：根据缺口覆盖、任务阶段、冲突、预算和权限选择最小充分集合。

能力效用可表示为：

```text
utility = gap_coverage + expected_quality_gain
          - context_cost - latency_cost - conflict_penalty - permission_friction
```

编排器不应一次挂载十几个相似 Skill。MVP 设置硬上限，例如每轮最多 3 个认知能力，只有用户固定或系统能证明新增能力覆盖独立缺口时才突破。

为防止能力反复装卸，使用迟滞机制：挂载阈值高于保留阈值，并规定最短生命周期；目标显著变化或用户明确操作除外。

### 3.6 上下文编译器 Context Compiler

主模型不直接读取全部状态。编译器按当前目标生成一个有预算的上下文包：

```ts
type ContextPacket = {
  sessionContract: unknown
  currentGoal: unknown
  activeWorkItem: unknown
  confirmedConstraints: unknown[]
  relevantEvidence: EvidenceRef[]
  selectedCapabilities: string[]
  expectedArtifact: unknown
  omittedScope: { reason: string; refs: EvidenceRef[] }[]
  tokenBudget: number
}
```

上下文按“明确约束 → 当前任务 → 直接证据 → 必需 Skill → 补充历史”排序。摘要必须附原文引用；当摘要置信度不足时优先放入关键原文片段。

### 3.7 成果验证与回写器 Artifact Verifier

主模型自由回答后，系统检查：

- 是否满足用户要求和成果契约。
- 关键结论是否有引用或明确标记为推断。
- 是否遗漏已经确认的约束。
- 代码、结构化数据或文件修改是否通过确定性验证。
- 是否产生新的决策、任务、风险和未解决问题。

验证失败时，只修复具体缺口，不默认重新生成全部回答。用户确认的成果再写入项目状态；未经确认的内容先作为候选成果。

## 4. 单轮实际运行

以“我们接下来讨论 NeoCarry 的开发路线”为例：

1. 消息成为不可变事件。
2. 状态归并器识别主目标为“制定开发路线”，阶段为 `plan`，但保留置信度与原文证据。
3. 缺口诊断器发现缺少当前代码状态、目标版本和限制条件。
4. 分析门控器判断：项目概要与当前目标高度相关；完整提交历史相关但当前决策价值较低。
5. 能力检索器召回项目规划、代码库概览、技术风险评估等候选能力。
6. 规划器只挂载项目规划与代码库概览，风险评估暂时保留为候选。
7. 上下文编译器选择项目概要、最近决策与仓库结构，不发送全部聊天和 Git 历史。
8. 主模型提出路线或精准询问缺失信息。
9. 验证器检查路线是否覆盖目标、约束、里程碑和风险。
10. 用户确认的决定写入状态；能力组合根据下一轮任务调整。

## 5. 漫游分析如何落地

漫游分析不是另一套算法，而是同一管线的不同调度策略：

- 普通分析以当前目标为中心，低相关内容快速退出。
- 漫游分析把配方中的每个观察维度视为临时目标，分批扫描全部范围。
- 每批产生结构化发现与证据，随后进行跨批去重、冲突检测和关系合并。
- 当连续若干批没有新增高价值发现，或预算耗尽时停止。

这样可以复用事件解析、评分、上下文编译、验证和溯源能力，不维护另一套不可控系统。

## 6. 模型调用分层

不应让最贵模型承担所有步骤：

- 本地程序：解析、去重、检索、预算、权限和结构校验。
- 小模型或本地模型：片段分类、候选召回、粗粒度任务状态差异。
- 主模型：复杂意图、关键缺口、能力组合和最终工作。
- 验证模型：只在高价值、难以用规则验证的成果上启用，可与主模型相同但使用独立上下文。

MVP 可以先使用同一个模型的不同结构化调用验证管线，再根据真实成本和错误分布拆分模型，避免过早优化。

## 7. 失败与降级策略

- 无法识别意图：保持普通聊天，只展示一个非阻塞建议。
- 任务状态低置信度：不自动挂载项目级能力，继续收集信息。
- 能力规划失败：退回用户固定能力和基础聊天。
- 超出预算：优先保留明确约束与直接证据，并显示未分析范围。
- Skill 冲突：使用清单规则阻止组合，要求规划器重新选择。
- 工具不可用：主模型收到明确失败状态，不得假装已经使用。
- 模型输出格式错误：只重试结构化步骤，设置次数上限。
- 用户纠正：生成高优先级纠正事件，废止冲突推断并重算相关状态。

## 8. 必须先建的评测系统

没有评测集，算法优化会变成“换提示词后感觉更聪明”。首批建立 100 至 300 个经过人工标注的会话片段，覆盖：

- 目标明确和目标模糊。
- 会话中途改变目标。
- 多个项目或多个子任务混合。
- 应装载、可选和不应装载的能力。
- 高相关低价值与低相关高价值内容。
- 导入会话中的工具调用、错误和恶意指令。
- 用户纠正系统理解。

每个样本标注：

- 当前目标、阶段和子任务。
- 证据片段。
- 关键缺口。
- 应深入与应跳过的内容。
- 推荐能力集合及不可使用能力。
- 预期成果契约。

核心指标：

- 关键目标和约束的遗漏率。
- 能力选择准确率与关键能力召回率。
- 无关 Skill 装载率。
- 深入分析命中率与关键片段漏检率。
- 单轮 Token、延迟和费用。
- 用户纠正次数。
- 引用能否准确回到原文。
- 最终成果首次接受率。

涉及权限和外部副作用的错误采用硬失败指标，不能用平均准确率掩盖。

## 9. 实施路线

### 阶段 A：可回放算法骨架

- 定义 `WorkspaceEvent`、`TaskState`、`Claim`、`EvidenceRef` 和能力清单。
- 实现本地事件存储、状态版本和回放器。
- 建立 30 个种子样本和人工标注工具。
- 用结构化模型调用实现任务状态差异提取。

交付标准：任意样本都能重放，并解释当前状态来自哪些原始事件。

### 阶段 B：精准分析闭环

- 实现缺口诊断器。
- 实现相关度与决策价值子信号。
- 实现分析预算与上下文编译。
- 在 100 个样本上建立首个基线。

交付标准：系统能明确展示深入分析与跳过片段的理由，Token 使用可测量。

### 阶段 C：能力装载闭环

- 先建立 10 至 20 个边界清楚的能力清单。
- 实现候选检索、最小组合、冲突检查和迟滞机制。
- 加入用户固定、移除和替换。
- 对比“无编排、全量装载、智能装载”三组结果。

交付标准：智能装载在任务结果不下降的前提下，显著减少无关能力和上下文成本。

### 阶段 D：会话导入与漫游

- 支持通用 Markdown / JSONL，随后加入 Claude 与 Codex 适配器。
- 复用统一事件流和状态归并器。
- 实现标准漫游配方、分批扫描与停止条件。

交付标准：导入结果可追溯，能继续任务；漫游结果明确覆盖与未覆盖范围。

### 阶段 E：产品体验

- 默认只展示目标、关键问题、建议动作和能力状态。
- 高级面板展示证据、评分子信号、调用记录和预算。
- 收集用户纠正作为评测样本，经确认后进入数据集。

交付标准：简单用户无需配置即可完成任务，高级用户能够审计和控制全过程。

## 10. 第一个可开发的垂直切片

不要先做完整聊天客户端。第一个原型只完成一条链路：

```text
粘贴一段项目讨论
→ 识别目标、阶段、约束与关键缺口
→ 对片段进行相关度 / 决策价值门控
→ 从 10 个认知 Skill 中选择最多 3 个
→ 展示“为什么选择”
→ 生成一份带原文引用的下一步建议
→ 用户纠正并重算
```

这个切片可以同时验证任务状态、精准分析、能力规划、透明解释、引用和反馈闭环。它成功后，再连接真正的多模型聊天、MCP 和复杂 UI。


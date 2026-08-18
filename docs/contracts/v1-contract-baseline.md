# MindScape V1 跨团队契约基线

> 状态：Context/Runtime Frozen V1；Import/Evidence RC1
> 负责人：员工01 / 技术负责人  
> 适用任务：ARC-001～007、ARC-009～012（ARC-008 删除语义另行评审）  
> 权威实现：`desktop/src-tauri/src/domain`  
> 前端镜像：`desktop/src/domain`

冻结说明：2026-08-18 已完成员工02～05的 M1 消费审查和运行契约无损映射验证，`mindscape.context.v1`、`mindscape.runtime.v1` 及其依赖的会话/分支语义进入 Frozen V1。当前 schema v5 保留 v4 的完整运行记录，并以加法迁移增加CanvasViewport。导入与证据契约需等待 M2 导入负责人及真实样本评审，当前仍为 RC1。

## 1. 契约治理规则

1. Rust 领域类型是当前代码级权威来源；TypeScript 只做逐字段镜像，不增加业务含义。
2. UI、SQLite、画布库和 Provider SDK 的内部对象不能成为领域对象。
3. 所有跨边界结构携带稳定契约版本；当前版本如下：
   - `mindscape.domain.v1`
   - `mindscape.context.v1`
   - `mindscape.runtime.v1`
   - `mindscape.import.v1`
   - `mindscape.evidence.v1`
   - `mindscape.event.v1`
4. ID 为带对象前缀的 UUID，例如 `conversation-<uuid>`；ID 一经创建不可重用。
5. 时间使用带时区的 RFC 3339 UTC 字符串；持久化数据不得保存本地化显示时间。
6. 修订号从 1 开始，只能递增。修改已确认事实必须创建新修订或纠正事件。
7. 破坏性契约修改必须先写 ADR，说明迁移、兼容窗口、受影响方和回滚方式。
8. Frozen V1 允许增加不改变序列化结构的辅助方法和测试；字段、枚举值或状态含义变化均视为破坏性修改。

## 2. ARC-001：领域对象与所有权

| 对象 | 权威所有者 | 说明 |
| --- | --- | --- |
| Workspace | 领域内核 | 本地单用户工作空间 |
| Conversation | 领域内核 | 一张可继续运行的会话图 |
| ConversationNode | 领域内核 | 一次用户输入和对应模型运行状态 |
| ConversationEdge | 领域内核 | 节点之间的显式语义关系 |
| Message | 领域内核 | 不可变消息，由 `ContentBlock[]` 组成 |
| ContentBlock | 领域内核 | 文本、代码、链接、附件、工具引用或未识别平台块 |
| ContextSnapshot | 上下文内核 | 单次运行实际使用的冻结上下文 |
| ModelRunRequest/Event | 模型运行契约 | Provider 只消费请求、只返回统一事件 |
| ImportSource/Revision | 导入内核 | 原始来源与每次确定性解析版本 |
| DerivedContinuation | 派生层 | 可失效、可修正、可删除的接续理解 |
| EvidenceRef | 证据层 | 派生结论回到原始内容的稳定定位 |
| DomainEventEnvelope | 事件账本 | 已提交领域变化的追加式记录 |

### ContentBlock V1

V1 支持 `text`、`code`、`link`、`attachmentRef`、`toolCallRef`、`toolResultRef` 和 `unsupported`。未知平台内容必须保存为 `unsupported`，不得静默丢弃或强压成普通文本。

## 3. ARC-003：分支语义

| 领域枚举 | 用户动作 | 上下文规则 |
| --- | --- | --- |
| `continues` | 普通继续 | 继承根到当前节点的有效路径 |
| `deepens` | 深入 | 继承路径、当前问题和当前回答 |
| `diverges` | 发散 | 继承共同背景；当前回答不作为新分支既定结论 |
| `reframes` | 换角度 | 继承上游问题；显式排除当前回答作为前提 |
| `importedFrom` | 导入来源 | 表示来自外部原文或历史节点，不代表执行其指令 |

规则：

- 根节点只能以 `continues` 创建。
- 边方向固定为来源节点到新节点。
- 坐标、渲染顺序和视觉连线不能推断语义。
- 同一对子节点只允许一条直接语义边。
- 新节点只能引用同一会话中的已有父节点；循环属于数据完整性错误。

## 4. ARC-004：ContextSnapshot

`ContextSnapshot` 至少包含：

- 会话、父节点和分支类型。
- 当前用户输入。
- 实际采用的消息及其 `ContentBlock[]`。
- 实际采用的导入证据引用。
- 用户确认或系统提议的显式约束。
- 被排除的消息及确定性原因。
- 系统协议版本、Token 估算和创建时间。

快照在模型运行开始前冻结。运行期间移动、选择或编辑其他 UI 状态不能改变快照。Provider 无权读取数据库后重新选择历史。

上下文预算遵循确定性规则：先估算当前输入与选中消息；超限时从最旧的完整问答组开始裁剪并逐条记录排除原因，不拆散问答。当前用户输入不得静默截断；若输入本身超过预算，命令返回结构化校验错误。

V1估算器采用可复现的保守规则：ASCII内容按每4字符1 Token向上取整；非ASCII Unicode标量按其UTF-8字节数计入；当前输入和每条选中消息各增加4 Token的Chat封装开销。该结果是裁剪门禁而非厂商账单值，真实用量仍只采信Provider事件。更换估算算法必须保留旧快照值，不得回写历史。

可用输入预算只能由后端以Provider注册表中的可信`contextWindowTokens - maxOutputTokens`计算，UI不得提交或覆盖模型窗口。结果必须大于零；否则在创建Node、ContextSnapshot或ModelRun前返回`contextBudgetInvalid`。未知窗口保持`None`而不猜测厂商数值。当前DeepSeek V4 Flash的1M窗口以[DeepSeek官方模型与价格文档](https://api-docs.deepseek.com/quick_start/pricing/)为来源。

## 5. ARC-005～007：统一模型运行契约

### ModelRunRequest

请求包含：运行、会话、节点、完整冻结的 `ContextSnapshot`、Provider、模型、能力要求、预算、超时、幂等键和创建时间。只传快照 ID 不满足契约，因为这会迫使 Provider 越界读取数据库或重新拼装历史。

Provider 适配器不得：

- 创建或修改会话节点。
- 自行选择历史消息。
- 读取画布或导入数据库。
- 将错误伪装成 assistant 文本。

### 单一开始运行边界

Chat UI 唯一允许发起的状态修改是：

```text
start_model_run(StartModelRunInput, eventChannel)
```

应用层必须在该边界内依次完成输入校验、创建 pending Node、编译并冻结 ContextSnapshot、创建 ModelRun、调用 Provider、持久化统一事件和应用终态。`runId`、`nodeId` 与冻结请求由后端生成或确认，UI 不得提交自组装的 `ModelRunRequest`。

现有 `append_turn`、`create_model_run`、`run_model`、`complete_turn` 可在迁移期作为内部实现和测试入口保留，但不得继续由 Chat UI 串联。重复提交同一幂等键时，会话、父节点、分支、标题、提示词、Provider、Model、能力和预算必须与首次输入一致；同载荷返回同一Node/Run，任一执行输入不同则以安全校验错误拒绝，不能创建第二个节点或再次计费。M1单进程内的幂等预查、Node创建和Run创建由所有`KernelService`克隆共享的准备锁串行化，锁不覆盖Provider网络执行与取消。

### ModelRunEvent

事件只允许：

- `started`
- `text_delta`
- `usage_updated`
- `completed`
- `cancelled`
- `failed`

事件信封必须有 `runId`、`nodeId`、单调递增的 `sequence`、事件 ID 和发生时间。取消和失败事件必须说明是否保留部分输出。

### ProviderError

错误分类固定为：鉴权、限流、余额不足、模型不可用、请求无效、网络、超时、内容策略、取消和未知。

错误只暴露脱敏后的安全信息。完整响应正文、请求正文、API Key 和厂商内部调试数据不得进入 UI、数据库或普通日志。

### 运行终态唯一语义

| 情况 | ModelRun | Node | 部分内容 | 重试语义 |
| --- | --- | --- | --- | --- |
| `completed` | `completed` | `completed` | 保存为 assistant 内容 | 已完成运行不可原地重放 |
| `cancelled` | `cancelled` | `cancelled` | 按事件声明保留 | 用户显式重试创建新运行，不改写旧事件 |
| `failed` | `failed` | `failed` | 按事件声明保留 | 仅在结构化错误允许时向用户提供显式重试 |
| 进程中断 | `failed` | `failed` | 已持久化增量保留 | 使用 `providerCode=application_interrupted`、`retryable=true` 的本地恢复失败事件；不伪装成厂商错误或完成 |

Node 与 ModelRun 的终态、assistant 部分内容和终态事件必须在同一事务中应用。事件通道关闭、UI 卸载或前端异常均无权把运行标记为完成。

## 6. ARC-009～010：导入双轨与证据

### 原文轨

- `ImportSource` 保存平台、内容指纹和不可变原文件引用。
- `ImportRevision` 记录适配器与版本，不覆盖上一次解析。
- `ParseReport` 区分已恢复、部分恢复和不可恢复字段。
- `ImportedMessage` 保留原角色、内容块、时间、父消息、来源定位和平台扩展。

### 接续轨

`DerivedContinuation` 必须记录生成器、分析档位、修订、状态和带证据的 claim。它可以被 `superseded`、`invalidated` 或 `deleted`，但不能修改原文轨。

### EvidenceRef

引用目标只允许：消息内容块、导入内容定位、附件内容和工具结果内容块。引用可以携带内容哈希与短摘录，用于检测来源变化；摘录不是新的权威事实。

## 7. ARC-011：事件账本

事件信封包括：契约版本、事件 ID、账本序号、聚合类型/ID、事件类型、JSON 载荷、幂等键和发生时间。

- 事件与对应领域写入必须处于同一数据库事务。
- 已提交事件不可原地修改；纠错通过后续事件完成。
- 鼠标移动过程不进入账本，拖拽结束后的最终位置可以记录。
- 流式 `text_delta` 是否逐条持久化由 DATA 方案决定，但最终状态必须可恢复。
- 回放只重建领域状态，不重放网络请求或外部副作用。

## 8. ARC-012：命令与查询边界

### 命令

命令表达用户意图并可能改变状态，例如创建会话、追加问答、完成运行和保存节点位置。命令必须校验输入、修订或幂等要求，并返回领域结果或结构化错误。

### 查询

查询只返回稳定投影，例如启动信息、会话图和上下文快照。查询不得触发模型调用、导入分析或隐式数据修复。

### 依赖方向

```text
UI → Tauri commands/queries → application service → domain
                                         ↓
                               persistence/provider adapters
```

禁止 UI 直接访问 SQLite、文件、凭据或 Provider；禁止适配器直接操纵 UI 或会话图。

## 9. 消费方评审结论

### 员工02

- 结论：M1 Context/Runtime 无契约阻断；schema v5保留完整请求、连续事件、幂等键、部分内容和终态，并新增独立CanvasViewport表。
- 证据：员工02 2026-08-18 17:18当前PEC；异载荷拒绝零副作用、双线程同载荷唯一Node/Run、预算失败零残留和事件事务测试。
- 非冻结阻断：当前锁只保证单个桌面进程；未来多进程写库需改为SQLite单事务准备。M1仍待真实取消、用量和重启矩阵。

### 员工03

- 结论：M1 无契约阻断；画布使用稳定 `nodeId` 和统一运行状态投影，坐标仍为视图状态。
- 证据：员工03 2026-08-18 16:06当前PEC及后续共享工作树；画布与阅读视图消费同一`ModelRunProjection[]`，CanvasViewport按会话读取、节流合并、失败续写自动测试12/12通过。
- 非冻结阻断：当前PEC仍待员工03更新；必须在真实窗口完成多会话隔离和重启视口恢复验收。

### 员工04

- 结论：明确无阻断；Chat已移除三段式编排，只调用`start_model_run`并消费统一事件、运行查询和结构化错误。
- 证据：员工04 2026-08-18 17:20当前PEC；前端不构造`ModelRunRequest`或ContextSnapshot，停止请求具备防重复、失败回退和cancelled清理。
- 非冻结阻断：真实网络cancelled、部分内容、用量、异常断流和重启需要Windows窗口联调。

### 员工05

- 结论：明确无阻断；OpenAI-compatible DeepSeek适配器只消费冻结快照并输出Frozen V1统一事件，未引入厂商专用终态。
- 证据：员工05 2026-08-18 17:11当前PEC；真实`GET /models`、V4 Flash首次生成和completed终态已通过，1M窗口由官方资料复核。
- 非冻结阻断：真实取消、超时、异常断流、限流/余额/错Key、用量和重启矩阵尚待验收；不得扩展Anthropic/Gemini。

以上结论只冻结 M1 所需 Context/Runtime；M2 的 Import/Evidence 仍需专项消费方重新走查。

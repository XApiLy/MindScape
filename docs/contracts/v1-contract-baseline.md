# MindScape V1 跨团队契约基线 RC1

> 状态：Review Candidate 1，等待员工02～05按消费方职责评审后冻结  
> 负责人：员工01 / 技术负责人  
> 适用任务：ARC-001～007、ARC-009～012（ARC-008 删除语义另行评审）  
> 权威实现：`desktop/src-tauri/src/domain`  
> 前端镜像：`desktop/src/domain`

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

## 5. ARC-005～007：统一模型运行契约

### ModelRunRequest

请求包含：运行、会话、节点、完整冻结的 `ContextSnapshot`、Provider、模型、能力要求、预算、超时、幂等键和创建时间。只传快照 ID 不满足契约，因为这会迫使 Provider 越界读取数据库或重新拼装历史。

Provider 适配器不得：

- 创建或修改会话节点。
- 自行选择历史消息。
- 读取画布或导入数据库。
- 将错误伪装成 assistant 文本。

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

## 9. RC1 评审清单

### 员工02

- 所有对象能否事务化持久化并安全迁移？
- 事件序号、幂等键和恢复状态是否可实现？
- 是否存在需要明文密钥进入普通数据库的字段？

### 员工03

- Node、Edge、BranchType 与坐标是否足以生成画布投影？
- 是否有画布库内部字段泄漏进领域类型？

### 员工04

- Chat 是否只需统一运行状态、事件、错误和上下文查询？
- 是否仍需要知道具体厂商 SSE 或自行拼历史？

### 员工05

- 请求是否足够表达能力、预算、超时和取消？
- 所有正式 Provider 的流式、用量和错误是否能无损映射？

完成上述评审并处理阻断意见后，RC1 才能标记为 Frozen V1。

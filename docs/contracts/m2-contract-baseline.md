# MindScape M2 跨团队契约基线 Draft 1

> Owner：员工01（技术负责人）  
> 日期：2026-08-24  
> 状态：第一切片可评审；SQLite、Provider 映射、UI 与检索接线尚未冻结

## 1. 兼容边界

- `mindscape.context.v1`、`mindscape.runtime.v1` 和 SQLite schema v5 保持不变。
- M2 使用新契约 `mindscape.effective-run-profile.v1`、`mindscape.focus.v1`、`mindscape.focused-context.v1`、`mindscape.knowledge.v1`、`mindscape.markdown-projection.v1` 与 `mindscape.generic-import.v1`。
- 新能力先以独立类型和包装快照接入；在员工02完成持久化评审前，不把 M2 字段写入旧 ContextSnapshot 表。
- 原始导入内容和 V1 EvidenceRef 仍是来源事实；实体、Markdown 和索引均不得覆盖原文。

## 2. ARC-M2-001：Effective Run Profile

`EffectiveRunProfile` 冻结本轮实际值，而非只保存用户设置：

- Provider、Model、reasoning 模式和预算。
- temperature、top_p、输出上限、响应格式、seed 及厂商扩展参数。
- 上下文策略、允许能力、工具权限与预算封套。
- 每个关键值的来源和运行时能力快照。

首切片已冻结类型；员工05现已将其作为可选加法字段接入 `ModelRunRequest`、应用幂等比较和 OpenAI-compatible 发送前校验，未提供时保持 M1 行为。员工04已补齐功能占位 UI，可从能力目录构造档案并随 IPC 发送；员工02仍负责后续不可变持久化，真实 Key/重启投影验收尚未完成。所有接线必须继续避免 UI、Provider 和 SQLite 各自定义运行档案。

## 3. ARC-M2-002 / CORE-M2-001～002：FocusFrame

`FocusFrame` 包含稳定 ID、会话、父节点、目标、上下文策略、记忆版本和四个互斥集合：

- `inheritRefs`：从主线或项目明确继承。
- `localRefs`：只在当前分支有效。
- `excludeRefs`：禁止进入当前上下文。
- `promoteRefs`：分支结束后可提交确认的回流候选。

上下文策略固定为：继续当前问题、聚焦新问题、从节点分支、原样续接。`compile_focused_context` 的第一版规则为：

1. FocusFrame 与 ContextCompileInput 必须属于同一会话和同一父节点。
2. 继续当前问题默认保留路径；其余策略只选择显式包含或继承的引用。
3. 排除优先于包含。若节点、用户消息或助手消息任一引用被排除，整轮消息都不进入快照，防止半轮内容泄漏。
4. 同一引用不能同时出现在两个记忆集合；目标不能为空，记忆版本必须大于 0。
5. 输出包装 `FocusedContextSnapshot`，保留选中引用、排除引用、原因和原 V1 ContextSnapshot；不修改旧快照语义。

`FocusFrameLifecycleSnapshot` 以独立加法契约记录 `active/closed`、revision、更新时间和关闭时间。纯领域入口 `close_focus_frame` / `reopen_focus_frame` 只允许 Active→Closed、Closed→Active，保持 FocusFrame 稳定 ID；持久化、切换查询和启动恢复由数据/命令层接入。

## 4. ARC-M2-003～004：知识与 Markdown 投影

- 实体类型固定为 Goal、Decision、Constraint、Question、Source、Project、Topic。
- 状态固定为 candidate、inferred、confirmed、rejected、superseded、stale。
- `KnowledgeScope` 明确 workspace、project、conversation 或 FocusFrame；禁止用画布位置推断作用域。
- 实体、关系和作用域证据均包含稳定 ID、修订、状态、生成器和 EvidenceRef。
- `MarkdownProjection` 只记录实体 ID、相对路径、实体/投影修订和内容哈希；改名不能改变实体身份，Markdown 不能成为事件账本。

## 5. ARC-M2-005 / IMP-M2-001～003：通用导入语义

- 首批格式固定为 Markdown、JSONL、TXT 和通用 JSON；入口固定为文件选择、拖放或粘贴。
- `GenericImportDescriptor` 记录编码、字节长度、内容哈希和不可变存储引用，不定义 ChatGPT/Claude/Codex 专用字段。
- `RawTrackEntry` 以顺序、来源定位和内容哈希连接 ImportSource、ImportRevision 与 ImportedMessage。
- `ImportGraphProjection.analysisPolicy` 目前只有 `disabled`，确保“原样继续”不会触发生成式分析或费用。
- 未知内容继续使用 V1 `Unsupported` ContentBlock 和原始 JSON 保存；ParseReport 必须报告恢复程度和警告。
- `validate_import_bundle` 是命令/存储层共用的纯领域预检：校验 source/revision/report 引用、消息 revision、消息 ID、source locator、父消息图和 ParseReport message count；失败不得提交半个 bundle。

## 6. 跨团队消费规则

- 员工02：消费本契约设计 schema/事务；不得反向以表结构改变领域状态或原文事实。
- 员工03：只投影 FocusFrame、分支和来源；不得在前端复制继承/排除算法。
- 员工04：只通过命令/查询消费导入预览、运行档案和上下文账单；不得直接解析 SQLite/Vault。
- 员工05：将能力目录映射为 EffectiveRunProfile；不支持的参数必须在创建 Node/Run 前失败，不得静默删除。

## 7. CORE-M2-003：知识状态与分支检索规则

- `candidate`/`inferred` 只能确认或否决；`confirmed` 只能被取代或标记过期，不能直接否决后覆盖旧事实。
- `rejected`、`superseded`、`stale` 是检索排除态；重新提出相反事实必须使用新的稳定实体 ID。
- 每次状态变更递增 revision、更新时间并记录新的 generator；旧版本仍由持久化层保留。
- 主线可按匹配作用域读取已确认事实；任务/探索/复盘分支必须显式 `inherit`/`include` 才能读取项目事实。
- 候选或推断即使被显式选入，也只能以 `CandidateOnly` 返回，不能作为已确认事实注入 Context Compiler。
- `exclude` 优先于 `include`/`inherit`；拒绝、取代、过期状态优先于任何 FocusFrame 选择。

纯领域入口为 `transition_entity` 与 `retrieval_decision`，不读写 SQLite，不改变画布或 Markdown 文件。

## 8. CORE-M2-004：知识上下文编译边界

- `KnowledgeRetrievalCandidate` 只代表索引层候选，不代表可直接注入模型的事实；候选必须带实体快照、EvidenceRef、检索分数和正数 Token 估算。
- `compile_knowledge_context` 先按确认状态、作用域和 FocusFrame 规则门控，再按检索分数降序、实体 ID 升序稳定排序，最后执行知识 Token 预算。
- `confirmed` 候选才可进入 `KnowledgeContextSelection.selected`；`candidate/inferred`、拒绝/取代/过期、作用域不匹配、排除项和预算外候选进入 `omitted`，并保留可解释原因。
- 选择结果记录 `retrievalVersion`、实体 revision、scope、EvidenceRef、检索分数和估算 Token；`FocusedContextSnapshot.knowledgeContext` 以可选加法字段承载它，不改变旧 `ContextSnapshot`。
- 重复候选、空检索版本、非法预算、空实体 ID、非正 Token 估算和 Token 溢出均显式失败；不静默截断或重排为未确认事实。

纯领域入口为 `compile_knowledge_context`，不读写 SQLite、全文/向量索引或 Provider；索引适配、关系扩展和持久化由员工02/05后续实现。

## 9. 本切片验证与未完成项

- Rust 契约序列化覆盖运行档案、知识作用域/状态/证据和禁生成式原样导入。
- FocusFrame 覆盖同父节点不同焦点、排除单条消息时整轮隔离、集合冲突拒绝。
- 知识状态机覆盖确认、否决、取代、过期、revision 递增和禁止复活；分支规则覆盖任务分支污染阻断、显式继承和候选不冒充事实。
- 知识上下文编译覆盖确认候选排序、预算排除、候选不注入、重复拒绝和 FocusedContextSnapshot 加法接线。
- 尚未完成：运行档案不可变查询/重启投影与真实 Key 验收、FocusFrame/知识生命周期命令接线、SQLite FocusFrame/知识持久化、Markdown 修订、实际导入解析、全文/向量索引适配、关系扩展、删除失效和 M2 纵向验收。这些不得从本 Draft 推断为已交付。

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

`FocusFrame::promotion_candidates()` 只为任务、探索和复盘分支构造不可变候选集合，冻结来源 FocusFrame、会话、分支类型和 memory version；`FocusFrameLifecycleSnapshot::promotion_candidates()` 是正式查询门禁，仅 Closed 分支可返回集合，Active、主线或空声明统一返回 `null`。重新打开分支后正式候选再次隐藏。该集合不是确认命令，也不会直接修改主线或项目知识；员工03/04只能展示候选，用户确认后才允许由知识状态机创建新版本实体和关系。

### 3.1 2026-09-01 冻结：正式候选生成契约

`mindscape.focus-promotion-generation.v1` 冻结首个正式 `promoteRefs` 入口，命令名预留为 `generate_focus_promotion_candidates`。该入口不是在创建 FocusFrame 时由前端填写裸 ID，也不是测试夹具或 SQLite 手工写入：

1. 仅 `Active` 的 Task / Exploration / Retrospective FocusFrame 可执行；Mainline 和 Closed 均拒绝。
2. 用户显式提交 `generationId`、`focusFrameId`、`expectedMemoryVersion`、`expectedLifecycleRevision`、非空 `candidateRefs` 和 `generatedAt`。引用必须去空白且唯一。
3. Kernel 必须按 ID 从权威存储加载完整 `KnowledgeEntity`；传入不存在、未选择或重复实体均失败，前端不能凭字符串制造候选。
4. 可选实体必须是同一 conversation、同一 FocusFrame scope 的 `candidate` 或 `inferred`，并至少带一条合法 EvidenceRef。这里“由内核确认”指实体存在性和 revision 已核验，不代表允许 `status=confirmed`；confirmed/rejected/superseded/stale 均不能再次进入候选生成。
5. 选择必须由有效 `GeneratorKind::User` 发起。纯领域计划对 candidateRefs 按稳定 ID 排序，冻结每个实体 revision，同时将 FocusFrame `memoryVersion` 和 lifecycle `revision` 各递增一次；旧版本、无变化选择和版本溢出显式失败。
6. 生成后 FocusFrame 仍保持 Active，候选不可用于四动作。用户显式 Close 后，既有 `FocusFrameLifecycleSnapshot::promotion_candidates()` 才把相同 `promoteRefs` 暴露为只读候选集合；Confirm / Promote / Reject / Delete 继续复用现有决策契约。

员工02负责把生成计划、FocusFrame JSON、memory version、lifecycle revision 和 generation receipt 放入同一 SQLite 乐观事务；员工03/04只消费 Kernel 返回的可选实体并发送 typed command，不在 React 侧过滤作用域/状态或自增版本；员工05在新 clean Release 中验证“生成时 Active 隐藏 → Close 后出现 → 四动作 → Reopen 隐藏 → 再 Close 保持已处理过滤”。

上下文策略固定为：继续当前问题、聚焦新问题、从节点分支、原样续接。`compile_focused_context` 的第一版规则为：

1. FocusFrame 与 ContextCompileInput 必须属于同一会话和同一父节点。
2. 继续当前问题默认保留路径；其余策略只选择显式包含或继承的引用。
3. 排除优先于包含。若节点、用户消息或助手消息任一引用被排除，整轮消息都不进入快照，防止半轮内容泄漏。
4. 同一引用不能同时出现在两个记忆集合；目标不能为空，记忆版本必须大于 0。
5. 输出包装 `FocusedContextSnapshot`，保留选中引用、排除引用、原因和原 V1 ContextSnapshot；不修改旧快照语义。

`FocusFrameLifecycleSnapshot` 以独立加法契约记录 `active/closed`、revision、更新时间和关闭时间。纯领域入口 `close_focus_frame` / `reopen_focus_frame` 只允许 Active→Closed、Closed→Active，保持 FocusFrame 稳定 ID；持久化、切换查询和启动恢复由数据/命令层接入。

`FocusFrameQueryProjection`（`mindscape.focus-query.v1`）是 UI/查询适配器的只读边界：`lifecycle` 始终是状态权威，`focusedContext` 可以为空（尚未编译或历史快照暂不可用）。查询投影会校验生命周期契约、FocusFrame、revision、更新时间和 active/closed 元数据，并复用 FocusedContextSnapshot 的持久化校验：检查快照/上下文契约版本、稳定 ID、会话与父节点一致性、记忆引用唯一性，以及知识选择的检索版本、引用唯一性和 Token 汇总；拒绝跨 FocusFrame 或跨会话拼接。前端不得自行推断 Active/Closed、重算知识选择或从空值伪造“无知识”。

当前 Tauri 命令名固定为：`create_focus_frame`（创建 Active/revision=1）、`get_focus_frame_query`（按稳定 ID返回只读投影）、`list_focus_frames`（按 conversationId 返回按更新时间排序的生命周期投影集合）、`close_focus_frame` 与 `reopen_focus_frame`（均接收 `focusFrameId`、`expectedRevision`、`updatedAt`，并返回更新后的查询投影）。关闭/恢复命令必须经过 KernelService 的领域状态机和 SQLite 乐观 revision 校验；FocusedContext 查询持久化尚未接入时只能返回 `null`，不得伪造已编译快照。前端必须使用列表查询发现当前会话的 FocusFrame，不得从节点坐标或 React 状态猜测 ID。

## 4. ARC-M2-003～004：知识与 Markdown 投影

- 实体类型固定为 Goal、Decision、Constraint、Question、Source、Project、Topic。
- 状态固定为 candidate、inferred、confirmed、rejected、superseded、stale。
- `KnowledgeScope` 明确 workspace、project、conversation 或 FocusFrame；禁止用画布位置推断作用域。
- 实体、关系和作用域证据均包含稳定 ID、修订、状态、生成器和 EvidenceRef。
- `MarkdownProjection` 只记录实体 ID、相对路径、实体/投影修订和内容哈希；改名不能改变实体身份，Markdown 不能成为事件账本。
- `MarkdownProjection::validate()` 在进入 SQLite 或文件适配器前拒绝未知契约、空身份、零修订和绝对/父级逃逸/非 Markdown 路径；`next_revision()` 保持投影与实体身份、只递增投影修订，并禁止实体修订倒退。用户改名或编辑必须形成新投影修订，不能覆盖旧版本。

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

KnowledgeEntity、KnowledgeRelation、ScopedEvidenceRef 和 EvidenceRef 在进入 SQLite/索引前必须通过领域校验：契约版本、稳定 ID、作用域、generator、revision、时间戳和 EvidenceTarget 必须完整；证据引用 ID 不得重复，关系两端不得为空或相同。校验失败不得写入事实源，也不得交给 FTS/VectorIndex 派生索引。

纯领域入口为 `transition_entity` 与 `retrieval_decision`，不读写 SQLite，不改变画布或 Markdown 文件。

## 8. CORE-M2-004：知识上下文编译边界

- `KnowledgeRetrievalCandidate` 只代表索引层候选，不代表可直接注入模型的事实；候选必须带实体快照、EvidenceRef、检索分数和正数 Token 估算。
- `compile_knowledge_context` 先按确认状态、作用域和 FocusFrame 规则门控，再按检索分数降序、实体 ID 升序稳定排序，最后执行知识 Token 预算。
- `confirmed` 候选才可进入 `KnowledgeContextSelection.selected`；`candidate/inferred`、拒绝/取代/过期、作用域不匹配、排除项和预算外候选进入 `omitted`，并保留可解释原因。
- 选择结果记录 `retrievalVersion`、实体 revision、scope、EvidenceRef、检索分数和估算 Token；`FocusedContextSnapshot.knowledgeContext` 以可选加法字段承载它，不改变旧 `ContextSnapshot`。
- 重复候选、空检索版本、非法预算、空实体 ID、非正 Token 估算和 Token 溢出均显式失败；不静默截断或重排为未确认事实。

纯领域入口为 `compile_knowledge_context`，不读写 SQLite、全文/向量索引或 Provider；索引适配、关系扩展和持久化由员工02/05后续实现。

`KnowledgeRetrievalProjection`（`mindscape.knowledge-retrieval.v1`）是 FTS、VectorIndex 与 Relation 适配器汇合后的只读候选边界：每个候选必须携带完整 `KnowledgeEntity`、EvidenceRef、估算 Token、整数检索分数和一个或多个来源（Vector / FullText / Relation）；投影同时携带 `retrievalVersion`、selected/omitted 引用和 `RetrievalNotice`。该投影仍不是事实确认结果，必须再经 FocusFrame 作用域、状态和预算编译；向量不可用时只能通过 notice 表达降级，不得伪装为语义检索完成。

## 9. 本切片验证与未完成项

- Rust 契约序列化覆盖运行档案、知识作用域/状态/证据和禁生成式原样导入。
- FocusFrame 覆盖同父节点不同焦点、排除单条消息时整轮隔离、集合冲突拒绝。
- 知识状态机覆盖确认、否决、取代、过期、revision 递增和禁止复活；分支规则覆盖任务分支污染阻断、显式继承和候选不冒充事实。
- 知识上下文编译覆盖确认候选排序、预算排除、候选不注入、重复拒绝和 FocusedContextSnapshot 加法接线。
- Markdown 投影覆盖安全相对路径、稳定身份、投影 revision 递增、实体 revision 防倒退和溢出拒绝；SQLite 修订历史与实际 Vault 文件写入仍由数据层接入。
- 当前共享工作树已出现 FocusFrame/知识生命周期、SQLite 持久化、通用导入解析、全文/向量/关系检索和删除失效实现，但仍需在代码评审、统一提交和同一 Tauri Release 中验收；Markdown/Vault 修订历史、运行档案重启投影、真实跨模块样本和 M2 纵向验收仍未关闭。不得从局部类型、单元测试或未提交工作树推断为已发布。

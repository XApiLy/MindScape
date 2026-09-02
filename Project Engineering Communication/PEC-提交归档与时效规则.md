# PEC 提交、归档与时效规则

> 状态：立即生效
>
> 权威制度：[PEC 当前窗口与归档制度](../docs/engineering/pec-retention-policy.md)

## 当前工程派单：M2 本地知识与聚焦连续性 Alpha

M1 已于 2026-08-20 Go。全员先阅读 [V1 总开发路线与责任归属](../docs/engineering/v1-master-roadmap-and-ownership.md)，再阅读当前 [M2 本地知识与聚焦连续性 Alpha 派单](../docs/engineering/m2-local-knowledge-and-focused-continuity-20260821.md)。员工01～05当前只执行 M2 中自己唯一的主交付和最多一个评审 / 联调 WIP；员工06是创始人直管的视觉协作席位，不进入统一工程派单，由创始人直接告诉其当前需要讨论、设计或验收的内容。

M2 的纵向闭环是：通用 Markdown / JSONL / TXT 会话导入 → 原文和来源保真 → 模型运行档案与思考控制 → Markdown 知识层与讨论日志 → 主线 / 任务分支记忆隔离 → 有限实体确认 → 全文 + 向量 + 关系检索 → FocusFrame 上下文编译 → 带来源回答 → 分支成果候选回流。平台专用导入、快速/详细分析、漫游分析、Obsidian 双向同步、复杂知识图谱、Harness、AI IDE、终端和新的视觉生产重构不得占用 M2 WIP。

M2 同时纳入“Markdown 原生阅读与字体偏好”交付：AI 正文和导入 Markdown 必须在安全 renderer 中呈现为标题、列表、引用、代码块、表格等结构；阅读区支持字体、字号、行高、宽度和受控本地字体偏好；原始 Markdown、渲染文本和运行事实严格分层。该交付不得扩张为文档编辑器、任意 HTML/插件执行器或新的 IDE 能力。

### 2026-08-29：首批字体资源已就位与接入分工

创始人确认的 Caveat、Nunito、Lato、Cormorant、寒蝉半圆体、小赖字体和得意黑已从官方固定版本下载至 `desktop/src/assets/fonts/`，对应 OFL、来源、版本、内部 family name、字节数和 SHA-256 见该目录 `README.md`。任何员工不得重复从第三方字体站下载同名资源，也不得把“资源已下载”写成“字体功能已完成”。

- 员工04把内置七款预设接入自己已有的“阅读偏好收口”WIP：`@font-face`、稳定 preset ID、字体预览/选择器、中文回退、重启偏好和失败回退。该项属于现有 WIP，不新增第三项并行工作。
- 员工02只负责用户导入字体的受控目录、格式/大小/元数据校验、复制/删除/恢复和 typed IPC；内置字体不绕行用户导入 IPC。该项在当前 typed IPC/统一样本 WIP完成或腾出槽位后进入，不挤占现有两个 WIP。
- 员工01只评审内置 preset ID、用户字体 ID 和领域隔离；员工03只检查聚焦阅读/画布重排性能；员工05在代码冻结后统一完成断网、缺字、加载失败、长文、DPI、重启、包体积和 M1 回归。
- 员工06仍由创始人直接安排最终字体卡片、配对与视觉验收，本控制文件不向员工06派发任务。

### 2026-08-30：字体现场预验收通过；本地呈现智能只冻结产品边界

创始人已确认员工04修正后的字体切换、真实长文和画布问答效果满意。该反馈关闭字体视觉预验收，不代表 M2 clean Release 门禁已经通过；员工03/04/05仍按原计划完成断网、DPI、重启、缺字、长会话、画布性能和包体回归，不再等待创始人重复确认同一视觉问题。

创始人同时确认新的产品方向：用户明确同意后，可以配置独立本地呈现模型，为已完成正文识别专业术语、实体和少量金句，并以可撤销标注层展示。详细边界见 `docs/design/visual-and-interaction/07-local-presentation-intelligence-layer.md`。该方向当前只做产品/契约冻结，不进入 M2 实现 WIP，不阻塞 M2 Go，也不允许任何员工自行下载/捆绑模型、扫描本机端口或把标注写回原文/RAG。里程碑获创始人批准后再正式拆分；员工06仍只接受创始人直接安排。

### 2026-08-30：M2 Go 阻断收敛

当前局部代码门禁和字体视觉已经通过，但 M2 仍只允许写 No-Go。最新字体 Build 来自 dirty 工作树，不能替代最终统一候选。下一步停止扩展新功能，按 [M2 Go 阻断审计](../docs/engineering/m2-local-knowledge-and-focused-continuity-20260821.md#10-2026-08-30-go-阻断审计) 收口：clean 集成提交与唯一 Release、候选确认/否决/回流的真实用户闭环、包含 EvidenceRef/Wikilink/讨论日志的 Markdown Vault、真实语义 Embedding 或明确范围裁决、同一真实样本的故障/重启纵向证据，以及仍在 M2 P1 中的受控用户字体导入。

员工01～05下一份 PEC 不得继续重复报告已通过的单元切片，必须明确关闭上述哪一项阻断并给出跨模块证据。员工06不进入本轮派单。创始人当前不需要继续提供 Key 或重复验收字体；只有真实语义 Embedding 与用户字体导入的里程碑范围发生冲突时才请求创始人裁决。

#### 员工直接执行表（阅读本 PEC 后立即按此收口）

| 阻断 | 主责与协作 | 必须交付的完成证据 |
| --- | --- | --- |
| B1 clean 集成基线与唯一 Release | 员工01负责集成边界、冲突与最终代码冻结；员工05负责统一回归和 Release 证据 | clean commit、唯一 Build ID、程序路径、SHA-256、无开发服务器启动、全量 Rust/Chat/Canvas/性能/Clippy/fmt/Vite 结果；dirty Build 不计 |
| B2 候选确认/否决/回流真实用户闭环 | 员工01收口领域契约；员工03负责正式 UI 消费与状态投影；员工04仅在其现有 Chat/阅读入口需要动作接线时协作 | 用户能在正式产品内确认、否决、回流、删除并在重启后看到一致状态；只读候选列表不计 |
| B3 可读 Markdown Vault | 员工02主责存储、修订、EvidenceRef、Wikilink 与讨论日志；员工01评审事务和领域边界 | 实体、会话/项目讨论日志和来源引用实际写入可读 `.md`；外部编辑、冲突、删除/否决和索引失效可复验；只有 frontmatter/H1 空壳不计 |
| B4 真实语义检索 | 员工05主责 Embedding 适配、质量样本与回退；员工02协作持久化、重建和失效；员工01评审接口 | 同义改写/非字面问题能召回正确 EvidenceRef，并提供模型标识、维度、耗时、离线/失败回退和重启重建证据；32 维 hash embedding 只能作为工程占位，不能作为生产质量通过 |
| B5 同一真实样本纵向验收 | 员工05维护验收矩阵并统一执行；员工01～04分别补齐自己链路的可复验证据 | 在同一 clean Release 内完成“导入 → 图谱/FocusFrame → 任务分支 → 确认实体/EvidenceRef → Markdown 外部编辑 → 全文/向量/关系检索 → 带来源回答 → Closed 候选 → 确认/否决/删除 → Reopen/重启”，并覆盖损坏、重复、超大、处理中断、索引失败回退和主线记忆不污染 |
| B6 受控用户字体导入（次级范围门禁） | 员工02只在 B3 核心存储 WIP 腾出后实现；员工04消费稳定字体 ID；员工05做安全和回归验收 | 受控目录、格式/大小/元数据校验、复制/删除/恢复、重启保持、缺字/损坏回退；若会拖延 B1～B5，立即停止并请求创始人决定移至 M3，不得静默拖慢 M2 |

执行顺序固定为：先冻结 B1 的集成入口，同时并行关闭 B2～B4；随后由 B5 使用同一 clean 候选串起全部证据；B6 不得抢占 B1～B5。任何人发现职责冲突，先在自己的最新 PEC 写明接口、阻塞对象和所需决策，不得另开未登记 WIP。员工06不在本表内，由创始人直接安排。

#### 2026-08-31 15:37 现场状态复审

- **B1 红色 / 未关闭**：分支相对远端 ahead 1，但共享树仍约 229 项 dirty/untracked；最新 Acceptance Build 仍是 `20260830-172245-m2-reading-rhythm-fix-f5307ad71b7f-dirty`，早于 B2/B4 最新实现，不能作为冻结候选。
- **B2 黄色 / 工程链已齐，待最终评审与真机证明**：员工01已提交原子决策领域契约 `1eaf191b`；员工02已完成 SQLite v16、Vault journal、索引失效与重启持久化；员工03已完成四动作正式 UI，并发布 Review Lab 员工主线 `20260831T025812Z-b2-回流决策与重启历史正式-ui-00a1`。员工01当前 PEC 早于员工02/03最终报告，其中“仍缺持久化/UI”的描述已过时；员工01下一步只做最终事务/接口评审并纳入 B1，不得重新实现。
- **B3 黄色 / 实现证据存在，待员工01评审并进入 B5**：可读 Vault、EvidenceRef 页面、Wikilink、讨论日志、外部编辑/冲突/否决/重启测试已经存在；尚未取得员工01最终事务边界签字和同一 Release 现场证据。
- **B4 绿色 / 工程阻断关闭**：员工05真实 384 维语义模型、生产禁用 hash 降级、失败回退和员工02持久化/重建已由员工01评审通过；仍需作为 B5 同一 Release 的一段复验，不再另开功能 WIP。
- **B5 黄色 / 审计工具门禁已绿，真实样本未执行**：前端现场复验 Chat 60/60、Canvas 34/34、Canvas 性能 2/2、Vite 2097 modules 均通过；Rust library 197 passed / 0 failed / 1 ignored，新增 `examples/b5_focus_promotion_audit.rs` 2/2，通过全量 `cargo test --all-targets --all-features`、Clippy 和 fmt。该审计文件在现场复审期间曾短暂出现编译/断言失败，员工05已即时修复；现在剩余的是用真实数据库和唯一 clean Tauri Release 执行纵向样本，不得把工具单测通过写成 B5 已关闭。
- **B6 灰色 / 未开始**：代码扫描只有七款内置字体，没有受控用户字体导入 IPC。继续不得抢占 B1～B5；如影响 M2 收口，提交创始人裁决移至 M3。
- **Review Lab 规则已生效**：2026-08-31 员工03已新增两次员工主线 preview，结束了 8 月 18 日后无员工版本的状态。preview 仍不替代 Tauri Release。

当前唯一正确推进顺序：员工01完成 B2/B3最终评审并冻结可集成范围；员工05保持现有全绿门禁、准备脱敏审计方式；随后清理共享树形成 clean commit，由员工05发布唯一 Acceptance Release 并执行 B5。员工04当前 PEC 仍写“等待创始人复验”，但创始人已确认字体视觉满意；该等待项已经关闭，员工04下一份 PEC 必须删除旧依赖。员工06不进入本轮派单。

## M2 当前状态复审（2026-08-25）

当前 PEC 窗口的切片推进正常，但 M2 仍处于 **No-Go / 纵向闭环未完成**，不得因为单元测试或局部 IPC 已通过而宣布 M2 Go。共享工作树本轮统一复验为：Rust 108/108、Chat 38/38、Canvas 20/20、Canvas 性能 1/1、Clippy、fmt、TypeScript/Vite 构建和 PEC retention 均通过；这些是切片回归证据，不是统一 Tauri Release 的 M2 发布证据。

当前已完成或基本完成的切片：员工01完成 FocusedContextSnapshot 领域校验门禁；员工02完成通用 Markdown/TXT/JSONL 原文保存、ParseReport、事务提交、重启查询，以及受控内容寻址孤儿文件的启动扫描回收；员工03完成 FocusFrame 会话列表和画布查询投影；员工04完成 FocusFrame 创建/关闭/重开及导入客户端接线；员工05完成本地 VectorIndex 快照/恢复契约；员工06的 VIS-008 独立视觉原型性能修正已完成，生产 `desktop/` 未修改。

下一道共同门禁必须先解决并用真实样本证明：

1. FocusedContextSnapshot 的 SQLite 持久化、查询和重启恢复。
2. 实体、关系、EvidenceRef、Markdown Vault、讨论日志修订和删除/否决后的索引失效闭环。
3. VectorIndex 与 SQLite/FTS/Relation 的字段映射、事务和可重建查询；离线/hash embedding 不得被描述为生产检索质量。
4. 导入 → FocusFrame → 实体/索引 → 聚焦上下文 → 带来源回答 → 分支隔离的同一无开发服务器 Tauri Release 验收；员工02的孤儿扫描回收必须作为该纵向样本的恢复回归，而不再作为未完成的独立阻塞项。

下一份 PEC 必须明确自己处于上述哪一道门禁，提供真实代码/测试/查询/恢复证据，并写明仍未接入的部分。没有统一 Build ID、程序路径、SHA-256 和真实跨模块证据，不得写“已验收通过”。

当前已建立 [视觉与交互设计中心](../docs/design/visual-and-interaction/README.md)，它是 UI、视觉、动效、模型控制呈现、导入体验、流式阅读和画布定位的统一摘要入口。该目录不改变领域契约，也不授权视觉生产重构；任何新 UI 仍先按占位 UI 规则承载，最终视觉由创始人直管的员工06确认。

本次新增 M2 设计入口：[M2 Markdown 原生阅读与字体偏好规格](../docs/design/visual-and-interaction/05-markdown-reading-and-typography-m2.md)。员工03/04/05涉及 Markdown 呈现、长文阅读或流式结构时，下一份 PEC 必须说明已阅读该规格、落实了哪条边界；员工06继续只接受创始人直接安排，不由本控制文件派发具体视觉任务。

关键时间：2026-08-25 前完成 M2 阅读回执；M2 首周确认真实通用导入样本、向量索引技术路径和统一 QA / 发布窗口；2026-08-25～09-05 完成契约、数据、导入、模型控制和分支闭环；2026-09-06～09-10 完成真实 Markdown Vault、分支污染、删除/否决、检索和 M1 回归验收；2026-09-11～09-14 使用统一 Tauri Release 进行 M2 Go / No-Go。具体责任、依赖和 No-Go 条件以 M2 派单正文为准。

M1 的真实 Chat 派单与验收证据继续作为历史基线，不再作为当前开工入口。任何 M2 任务如果可能破坏 M1 终态、取消、恢复、凭据或脱敏契约，必须先停工并由员工01与受影响负责人评审。

员工06固定承担 [视觉与交互设计工程师](../docs/design/visual-interaction-design-engineer.md) 席位，由创始人直接沟通和安排，不需要产品负责人或工程派单文件再次分配具体任务。员工06在收到创始人指令后与员工03、04共同落地；员工03、04继续负责各自工程正确性与性能，不自行建立另一套设计语言。员工06仍维护唯一最新PEC，用于记录已经收到的创始人指令、确认结果、工程交接和风险，因此七文件窗口不变。

## UI 工程先行占位规则（立即生效）

员工02、03、04在实现数据、画布、Chat、导入、运行控制等功能时，只要功能需要界面入口、状态展示或交互承载，可以先自行完成“功能占位 UI”，不必等待员工06。该 UI 的目标是验证功能、状态、IPC、键盘和可访问性，不是最终视觉交付。

占位 UI 必须遵守：

1. 只使用现有组件、现有色彩和最少必要样式；不得借机建立新设计语言、整体改版、加入特效、Shader、Pixi、复杂动效或大范围 CSS 重构。
2. 保留正确的 DOM 语义、键盘路径、焦点、测试选择器和状态结构，员工06后续应能替换表现层而不改变领域契约、IPC 和业务状态。
3. 在 TSX/HTML/CSS 入口处添加 `UI-HANDOFF-06` 注释，写清楚位置、用途、数据/IPC 来源、正常/加载/空白/错误/停止/恢复状态，以及哪些区域允许员工06替换。
4. 占位 UI 的文字可以是功能性文案，但不得伪造已完成能力；未接入的命令必须明确显示“待内核接入”或保持禁用。
5. 员工02、03、04负责占位 UI 的功能正确性、性能和自动化测试；员工06负责后续视觉、交互、动效和真实应用验收，不因此承担他们的业务实现。
6. 员工06仍由创始人直接安排，不得通过本规则向员工06派发具体任务、WIP 或截止时间。

统一注释模板见 [UI 工程先行占位与员工06交接规范](../docs/design/ui-placeholder-and-employee06-handoff.md)。

## Review Lab 可视进度发布规则（2026-08-31立即生效）

Review Lab 的“员工确认主线”用于让创始人及时看到可操作的 UI、交互和状态变化；`artifacts/acceptance/` 的唯一 Tauri Release 用于原生能力和里程碑验收。两者用途不同、必须同时保留，任何人不得再用“当前工作树是 dirty”或“尚未到最终 clean Release”作为不提交 Review Lab preview 的理由，也不得把 Review Lab preview 写成正式 Release 或 M2 Go 证据。

### 谁必须发布 Review Lab preview

- **员工03、员工04**：每完成一个可独立观察的 UI、交互、画布、Chat、阅读、导入、设置或状态呈现切片，在归档旧 PEC、提交新 PEC 之前，必须把当前可操作界面发布为一次“员工确认主线” `preview`。
- **员工01、员工02、员工05**：纯领域、数据库、Provider、索引、测试或性能内部改动不要求为了打卡生成无意义 preview；一旦改变用户可见状态、错误、恢复、来源、候选、检索结果或设置行为，同样必须发布。
- 后端切片没有任何可见变化时，最新 PEC 必须明确写“本轮无可见 UI 变化，因此不发布 Review Lab preview”，不能保持沉默。
- 员工06继续由创始人直管，负责创始人交给他的视觉评审与引用工作；本规则不要求员工06替员工01～05补发版本，也不改变其 WIP 和汇报关系。

### preview 的有效证据

使用 Review Lab GUI、CLI 或标准 API 进入“员工确认主线”，至少填写任务/B 编号、提交员工、实际变化、请创始人观察的位置和状态。版本必须记录源分支、commit、dirty 状态、dirty 文件数与源码指纹；共享 dirty 工作树生成的 preview 只是“当时完整界面快照”，不证明所有变化都由署名员工独立完成。构建失败必须保留失败记录并修复后新建版本，不得覆盖旧版本。

下一份涉及可见变化的 PEC 必须引用 Review Lab 版本 ID、状态和重点观察项。只写“构建通过”、只附截图、只提供 `localhost` 地址、只发布负责人观察分支，均不算员工主线交付。负责人点击“随时编译当前界面”生成的观察分支只供临时查看，不能代替员工确认主线。

### 与正式 Acceptance Release 的衔接

正式 Windows 验收程序仍执行下方唯一交付规则：员工01冻结 clean 集成源，员工05统一发布一次 Acceptance Build，其他员工不得各自生成等价正式版本。Review Lab `preview` 不要求 clean，不提供真实 Provider、SQLite、凭据或恢复证明。

每个正式 Acceptance Build 发布后必须在 Review Lab 留下同一 Build ID、来源 commit、SHA-256、验收范围和程序路径对应的 `full` 里程碑引用，且不得为登记而重新编译第二份“等价 EXE”。当前 Review Lab 与 `scripts/publish-acceptance.ps1` 尚无自动登记接口；在该接口落地前，员工05必须在 Release PEC 明确写出“Review Lab full 登记待接线”，并通知创始人，不得让正式构建静默消失。自动登记实现不得修改 Acceptance Build 的内容、哈希或唯一性；涉及 Review Lab 本体的修改仍由创始人决定是否交给员工06。

## 验收程序统一交付规则

所有人员立即执行 [验收程序构建与交付规则](../docs/engineering/acceptance-build-policy.md)。交给创始人的Windows程序只能由`scripts/publish-acceptance.ps1`发布到`artifacts/acceptance/versions/<build-id>/`；不得从`target/`直接交付，不得在`desktop/`根目录散放EXE，不得再创建`target-fixed`、`target-new`或同类替代编译目录。

验收脚本必须执行Tauri CLI release构建并内嵌前端。`cargo build`、`cargo test`或其他Rust命令产出的调试EXE即使能打开窗口，也不是有效验收程序；打开后访问`localhost`或要求另开前端服务的版本直接判定无效。

2026-08-19纠正：员工04当前PEC引用的SHA-256 `1ACC36F5CDC09C37AE9CECBEBE96B7FC02555DAD348677382F6B15D3B5EBCAFB`实测打开后访问`localhost:1420`，该EXE及其派生Build ID全部作废，不得用于CHAT-012/PROV-013复验。替代版本必须由Tauri release流程重新发布，并在无开发服务器时实际显示MindScape工作区；员工04下一份PEC必须明确纠正旧结论。

每次通知必须写明Build ID、完整程序路径、SHA-256、来源提交、本次验收范围和已知未通过项。只写“新验收程序：mindscape-desktop.exe”视为无效交付，创始人无需验收。同一轮跨员工验收必须使用同一个Build ID。

## 全员通知：下一次推进前必须完成阅读

员工01～06在继续修改方案、开始下一项任务或提交下一份 PEC 前，必须完成以下阅读。阅读的目的不是打勾，而是确认自己的实现不会违反产品边界、内核契约和协作规则。

### 全员必读

1. [第一版产品规划与范围边界](../docs/v1-product-scope.md)：确认 V1 做什么、不做什么和发布门槛。
2. [当前产品共识](../docs/product-foundation.md)：重点阅读 2026-08-17 确认的 Chat AI 长期边界。
3. [V1 总开发路线与责任归属](../docs/engineering/v1-master-roadmap-and-ownership.md)：理解 M0～M5 顺序、阶段出口和贯穿责任。
4. [M2 本地知识与聚焦连续性 Alpha 派单](../docs/engineering/m2-local-knowledge-and-focused-continuity-20260821.md)：确认当前唯一主交付、时间、依赖、通用导入范围与验收。
5. [产品内核与工程总览](../docs/core-kernel-overview.md)：理解会话图、冻结上下文、模型运行、导入和证据之间的关系。
6. [V1 前五名员工任务分配](../docs/v1-first-five-assignments.md)：重点阅读自己的长期责任、WIP 上限、依赖和禁止事项。
7. [V1 跨团队契约基线 RC1](../docs/contracts/v1-contract-baseline.md)：确认领域对象、事件、错误、导入和证据的统一含义。
8. [V1 安全威胁模型](../docs/architecture/v1-threat-model.md)：确认密钥、导入内容、日志、文件和用户数据边界。
9. [Git 提交、评审与合并规范](../docs/engineering/git-workflow.md)：确认分支、提交、PR、评审、合并和回滚方式。
10. [PEC 当前窗口与归档制度](../docs/engineering/pec-retention-policy.md)：确认新 PEC 与旧 PEC 的原子替换流程。
11. 当前 PEC 目录内其他五名员工的最新报告：只读当前报告，理解接口变化、阻塞和对自己的协作请求；历史细节再按需进入归档目录追溯。
12. [本地实体、会话知识库与混合 RAG 方案](../docs/architecture/local-entities-conversation-knowledge-and-rag-20260821.md)：理解实体、关系、原文、SQLite、Markdown、向量索引与 Context Compiler 的边界。
13. [Markdown 知识库、Obsidian 互操作与分支记忆隔离](../docs/architecture/markdown-vault-obsidian-branches-and-logs-20260821.md)：理解日志、Vault、FocusFrame 和主线 / 任务分支的记忆作用域。
14. [视觉与交互设计中心](../docs/design/visual-and-interaction/README.md)：理解内容优先、状态诚实、简单默认/复杂展开、UI占位交接、流式阅读锁定、画布定位和视觉工程边界；需要修改 UI 的人员必须继续阅读其对应主题文件。
15. [M2 Markdown 原生阅读与字体偏好规格](../docs/design/visual-and-interaction/05-markdown-reading-and-typography-m2.md)：涉及 Chat 正文、聚焦阅读、导入预览、字体设置或流式 Markdown 的人员必须精读。

### 按职责重点阅读

- **员工01｜领域架构与上下文**：重点复核 [架构走查清单](../docs/architecture/v1-architecture-walkthrough.md)、M2 契约和导入语义，并逐份读取员工02～05当前 PEC，处理实体、FocusFrame、分支作用域和冻结条件。
- **员工02｜桌面数据与恢复**：重点阅读 [本地数据布局](../docs/desktop-local-data-layout.md)、[安全凭据边界](../docs/desktop-credential-boundary.md)、M2 的 Markdown / 导入存储任务，以及员工01、员工04当前 PEC。
- **员工03｜无限画布**：重点阅读内核总览中的会话图、语义边、来源投影和 UI 非真相原则，以及员工01、员工02、员工04当前 PEC；在此基础上精读视觉中心的 `01`、`02`、`03` 和 `05`，画布节点保持轻量摘要，聚焦阅读器消费统一 Markdown 投影，不因全文重排破坏画布性能。
- **员工04｜Chat 与工作区**：重点阅读 M2 的运行档案、导入预览、FocusFrame、日志和错误契约，以及员工01、员工03、员工05当前 PEC；同时精读视觉中心的 `01`、`02`、`03`、`05`，落实统一 Markdown renderer、字体偏好占位、复制/导出分层和流式阅读锁定的 UI-HANDOFF-06 边界。
- **员工05｜Provider 运行时**：重点阅读 M2 的 reasoning 能力、Effective Run Profile、Embedding / VectorIndex 边界，以及员工01、员工02、员工04当前 PEC；涉及正文/思考/用量呈现或流式结构时再精读视觉中心 `02`、`05`，补充未闭合 Markdown、终态 flush 和空正文回归，不把展示动画或 HTML 当作运行事实。
- **员工06｜视觉与交互设计**：重点阅读整个[视觉与交互设计中心](../docs/design/visual-and-interaction/README.md)、[视觉与交互设计工程师职责](../docs/design/visual-interaction-design-engineer.md)、产品范围，以及员工03、员工04当前PEC；具体工作只以创始人直接指令为准，不从统一派单自行领取任务，并记录确认、否决、待验证内容和工程交接。

### 下一份 PEC 的阅读回执

每名员工下一份 PEC 必须新增 `## 已阅读与落实`，至少写明：

- 已阅读的文档及版本或当前状态。
- 对本人当前实现产生的具体影响，至少一项。
- 发现的冲突、疑问或需要谁确认的事项；没有问题时说明为什么没有影响。

只写“已阅读”“收到”或复制文档摘要不算完成。未完成阅读与落实说明的下一份 PEC 不进入 Ready for Review。

## 当前目录硬性约束

`Project Engineering Communication/` 必须始终且只能包含 7 个 Markdown 文件：

1. 员工01最新 PEC 一份。
2. 员工02最新 PEC 一份。
3. 员工03最新 PEC 一份。
4. 员工04最新 PEC 一份。
5. 员工05最新 PEC 一份。
6. 员工06最新 PEC 一份。
7. 当前规则文件或当前任务派发文件一份。

此目录禁止建立子目录，禁止同时保留同一员工的两份报告，禁止把旧报告直接删除。

## 每次推进后的操作顺序

1. 完成一个可说明的推进、修改、决策、阻塞变化或交接后，编写新的 PEC 当前快照。
2. 先用 `git mv` 将自己的旧 PEC 移至 `Project Engineering Communication Archive/员工NN/`。
3. 再把新 PEC 放入当前目录，文件名使用 `员工NN-主题-YYYYMMDD-HHmm.md`，时间采用 UTC+8。
4. 归档旧文件和新增当前文件必须位于同一个分支、同一个 PR，原则上位于同一个提交。
5. 从仓库根目录运行 `node scripts/check-pec-retention.mjs`，通过后才能提交。

## 新 PEC 必须回答

- 已阅读哪些必读材料，具体落实到哪里。
- 本次推进了哪些任务，任务编号是什么。
- 相比上一份 PEC 发生了什么变化。
- 交付物和可核验的代码、文档、提交或测试证据在哪里。
- 契约、数据、安全、恢复和跨团队接口是否变化。
- 验证结果、已知风险与阻塞是什么。
- 下一步最多两个 WIP 项是什么，需要谁配合。

旧 PEC 归档后视为历史记录，不再修改。需要纠正历史时，提交新的当前 PEC 说明纠正内容和原因。

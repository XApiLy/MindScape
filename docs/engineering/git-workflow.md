# MindScape Git 提交、评审与合并规范

> 状态：V1 生效
>
> 适用范围：`D:\Project\MindScape` 根仓库及全部子目录
>
> 远程仓库：<https://github.com/XApiLy/MindScape>
>
> 对应任务：REL-002

## 1. 目标

这套规则服务于三个目标：主分支随时可构建、领域与数据变化可追溯、多人并行时不让局部实现破坏 MindScape 内核。Git 历史是工程审计记录，不是个人工作日志。

## 2. 仓库边界与权威来源

| 路径 | 地位 | 合并要求 |
|---|---|---|
| `desktop/` | 唯一正式桌面工程 | 全部工程检查 |
| `desktop/src-tauri/src/domain/` | Rust 领域契约权威来源 | 高风险评审 |
| `desktop/src/domain/` | 前端消费镜像 | 必须与 Rust 契约同 PR 同步 |
| `docs/contracts/`、`docs/architecture/` | 跨团队契约与架构记录 | 契约变更需 ADR |
| `docs/` | 产品、范围和工程记录 | 不得与正式契约相互矛盾 |
| `Project Engineering Communication/` | 员工交付和协作记录 | 不能代替代码、测试或正式决策 |
| `apps/desktop/` | 旧原型 | 默认只读，不得成为正式依赖 |
| `UI Design/` | UI 参考 | 默认只读，不得反向定义领域状态 |

禁止提交依赖目录、构建结果、本地数据库、运行日志、API Key、真实私人会话和未脱敏导入样本。需要保留的大型二进制资产必须先评审是否使用 Git LFS；单个超过 10 MB 的新文件不得直接合并，除非 PR 明确说明必要性和替代方案。

## 3. 分支模型

采用主干开发，长期分支只有受保护的 `main`。

### 3.1 分支命名

格式：`<type>/<TASK-ID>-<short-description>`。

允许的类型：

- `feat`：新增用户或工程能力。
- `fix`：修复缺陷。
- `refactor`：不改变外部行为的结构调整。
- `perf`：性能改进。
- `test`：测试与测试基础设施。
- `docs`：仅文档。
- `build`：构建与依赖。
- `ci`：持续集成。
- `chore`：仓库维护。
- `hotfix`：生产级紧急修复。
- `release`：版本稳定与发布准备。

示例：

```text
feat/CORE-014-run-coordinator
fix/DATA-015-interrupted-run-recovery
docs/ARC-014-freeze-runtime-contract
```

### 3.2 分支纪律

- 所有分支从最新 `main` 创建，一条分支只处理一个可独立验收目标。
- 分支应短生命周期，原则上不超过三个工作日；超过时拆分可合并切片或在 PEC 中说明原因。
- 个人 WIP 上限为两条，不以提前开分支代表开始任务。
- 禁止团队共享功能分支；分支负责人对冲突、测试和最终清理负责。
- 更新主干优先使用 `git rebase origin/main`，不把 `main` 的合并提交带入功能分支。
- 个人分支需要改写历史时只允许 `git push --force-with-lease`，禁止 `--force`。
- PR 已批准后发生实质性重写，旧批准失效，必须重新评审。

## 4. 提交规范

提交格式采用 Conventional Commits：

```text
<type>(<scope>): <imperative summary>

Why this change is needed and any non-obvious constraints.

Refs: CORE-014
```

推荐 scope：`kernel`、`data`、`canvas`、`chat`、`provider`、`import`、`analysis`、`desktop`、`docs`、`release`。

示例：

```text
feat(kernel): finalize nodes from terminal run events
fix(provider): abort network stream after persistence failure
test(data): cover interrupted schema migration recovery
docs(repo): define protected-main merge policy
```

要求：

- 一个提交只表达一个原因明确、可单独审查的变化。
- 摘要使用祈使语气，描述结果，禁止 `update`、`changes`、`fix bug`、`WIP` 等无信息文本。
- 代码、测试和必要文档应在同一逻辑提交或同一 PR 内到齐。
- 纯格式化不得与语义修改混在同一提交。
- 破坏性契约变更使用 `type(scope)!:`，并在尾注写 `BREAKING CHANGE:`、迁移窗口和回滚方案。
- 每个提交或 PR 必须关联稳定任务编号；临时发现的问题先登记任务再扩大范围。
- 禁止提交被注释掉的替代实现、调试密钥、个人配置和无关文件。

PR 建议控制在 400 行以内；超过 800 行非生成代码时，作者必须说明为什么无法拆分。迁移、机械重命名或生成锁文件不计入该判断，但必须与语义变化分开方便评审。

## 5. 变更风险等级

### 标准变更

普通 UI、局部实现、测试、无语义文档调整。要求至少一名非作者评审者批准并通过全部必需检查。

### 高风险变更

以下内容默认高风险：

- Rust 领域契约、事件信封、ContextSnapshot 和状态机。
- SQLite schema、migration、数据删除和恢复。
- 凭据、日志、文件系统、导入不可信数据。
- Provider 网络请求、取消、超时、重试、计费和错误映射。
- 发布、签名、更新和遥测策略。
- 跨模块公共接口和任何 `BREAKING CHANGE`。

高风险变更要求：两名非作者批准，其中至少一名为对应领域负责人或技术负责人；必须提供失败模式、数据影响、测试证据和可执行回滚方案。破坏性契约变更还必须先有已接受 ADR。

### 紧急修复

仅用于生产数据损坏、安全事故或核心功能完全不可用。仍必须通过 PR，不允许直接推送 `main`。可以缩短评审等待，但至少需要一名技术负责人批准和针对性测试；合并后一个工作日内补齐完整回归、事故记录和长期修复任务。

## 6. Pull Request 生命周期

1. **开工**：任务编号、负责人、范围、依赖和验收标准明确。
2. **草稿 PR**：尽早创建 Draft PR，用于暴露接口和风险，不用于提前索取批准。
3. **作者自检**：完成代码自审，删除调试内容，填写 PR 模板并运行适用检查。
4. **契约检查**：涉及领域、数据、Provider 或导入时，确认权威契约、TS 镜像、迁移和错误语义同步。
5. **评审**：评审者检查正确性、边界、失败路径、可恢复性、测试充分性和范围漂移。
6. **更新主干**：合并前 rebase 最新 `main`；冲突由作者本地解决并重新运行检查。
7. **必需检查**：所有自动化检查成功，无未解决对话，无新的高风险变更未重新批准。
8. **合并**：默认使用 Squash Merge，Squash 标题必须符合提交规范并包含任务编号。
9. **清理**：删除远程和本地功能分支，更新 PEC、任务状态和必要的决策记录。

禁止以“后面再补测试”“只是临时”“先合进去联调”为理由绕过门禁。需要联调时使用 Draft PR、测试替身或临时集成环境，不污染 `main`。

## 7. 合并策略

- `main` 禁止直接 push、禁止强推、禁止删除。
- 默认只允许 Squash Merge，保持每个 PR 在主干上对应一个可回滚原子变更。
- 禁用普通 Merge Commit；Rebase Merge 仅在发布负责人为保留经过设计的多提交迁移序列时特批。
- 合并按钮只能在必需检查通过、审批数量满足、讨论解决、分支为最新状态时启用。
- 多个高风险 PR 修改同一领域时串行合并；后一个 PR 必须基于前一个合并后的 `main` 重新验证。
- 合并后的问题使用新的修复 PR 或 `git revert <squash-commit>`；禁止重写公共历史或 reset 主干。

远程仓库建立后，`main` 至少配置以下保护：

- Require a pull request before merging。
- Require approvals：标准变更 1，高风险通过 CODEOWNERS/流程保证 2。
- Dismiss stale approvals when new commits are pushed。
- Require review from Code Owners。
- Require conversation resolution。
- Require status checks and branch up to date。
- Require linear history。
- Block force pushes and deletions。
- Include administrators / do not allow bypass，紧急流程除外且必须审计。

## 8. 必需检查与验证层级

正式工程最低必需检查：

```text
frontend / canvas tests
frontend / chat tests
frontend / TypeScript production build
rust / fmt
rust / clippy -D warnings
rust / tests
security / secret and forbidden-file scan
governance / PEC six-file current-window check
```

GitHub 远程建立后，将以下 CI job 名称配置为 `main` 的 required status checks：

```text
repository-hygiene
frontend
rust
```

按变化追加：

- 画布：交互回归、200 节点性能、最低窗口尺寸。
- Provider：契约测试、错误矩阵、真正网络取消、超时和用量测试。
- 数据：上一发布版本升级、失败备份、强制退出和重启恢复。
- 导入：真实脱敏样本、恶意输入、超大文件、原文哈希与中断恢复。
- UI：截图或录屏、键盘焦点、空态、长文本和错误态。
- 发布：干净环境安装、升级、卸载、重装和签名验证。

测试失败只能通过修复代码、测试或经评审修正错误要求解决，禁止删除有效测试或降低门槛来获得绿色状态。

## 9. 领域所有权与强制评审

当前临时责任映射：

| 区域 | 主责任 | 强制协作 |
|---|---|---|
| 领域契约、上下文、ADR | 员工01 | 受影响模块负责人 |
| SQLite、迁移、恢复、凭据 | 员工02 | 员工01；安全变更加员工05 |
| 无限画布、图投影、视口 | 员工03 | 员工01、员工04 |
| Chat、工作区、运行状态呈现 | 员工04 | 员工01、员工05 |
| Provider、流式、取消、用量 | 员工05 | 员工01、员工02、员工04 |

真实账号确定后，用同样的映射建立 `.github/CODEOWNERS`，不得把临时“员工01”文字伪装成平台账号。跨区域 PR 由影响最大的领域负责人担任主评审者。

评审批准表示评审者确认变化可以进入主干，不只是“看过代码”。作者不得批准自己的 PR；管理者不以职位代替领域评审。

## 10. 数据库、契约与安全特别规则

- 已进入发布版本的 migration 只能追加，禁止原地修改。
- schema PR 必须包含从上一发布版本升级的测试样本、失败恢复和备份说明。
- 领域契约先改 Rust 权威类型，再同步 TS 镜像和消费者；不得由 UI、SQLite 或 Provider SDK 反向定义语义。
- 每个产生实质进展或状态变化的工程 PR 必须同时更新负责人 PEC；旧 PEC 进入独立归档，当前目录始终满足六文件不变量。
- 导入原文、事件账本和已冻结 ContextSnapshot 不得被派生结果覆盖。
- API Key 只进入操作系统安全凭据存储，不进入前端状态持久化、数据库、日志、测试快照或错误信息。
- 一旦密钥进入提交，无论是否尚未推送，都按泄漏处理：立即撤销密钥，清理历史，记录事故；仅删除当前文件不算解决。
- 安全历史重写由技术负责人主持并通知所有协作者重新同步，个人不得自行改写公共历史。

## 11. 发布、版本与热修复

- 采用语义化版本；V1 稳定前使用 `v0.x.y-alpha.n` 或 `v0.x.y-beta.n`。
- 日常开发始终进入 `main`。只有进入版本冻结后才创建短期 `release/vX.Y.Z`，其中只允许发布阻断修复、版本元数据和说明。
- 发布提交通过签字后创建不可移动的带注释标签；标签必须指向通过完整发布矩阵的主干提交。
- 热修复从当前生产标签或 `main` 创建 `hotfix/<TASK-ID>-...`，修复必须回到 `main`，不得维护永久分叉。
- 回滚优先 `git revert`。涉及不可逆数据迁移时，不把“代码回滚”等同于“数据回滚”，必须执行 PR 中已验证的恢复方案。

## 12. 管理指标

每周检查：

- PR 从 Ready 到首次评审的时间。
- PR 从创建到合并的周期和超龄分支数量。
- 合并后七日内的 revert / hotfix 比例。
- 主干红灯持续时间和绕过门禁次数。
- 高风险 PR 是否具备 ADR、恢复测试和双人批准。
- 单人 WIP 是否超过两条。

这些指标用于发现流程瓶颈，不按提交数量、代码行数或 PR 数量评价个人绩效。

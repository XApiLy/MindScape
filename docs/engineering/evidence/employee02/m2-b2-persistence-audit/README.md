# B5｜B2 原子持久化脱敏审计说明

> 负责人：员工02（桌面端与数据）  
> 适用范围：B5 唯一 clean Tauri Release 的真实导入提案审核，以及 `confirm / promote / reject / delete` 纵向验收  
> 工具性质：只读核对；不会写入 SQLite、Vault 或索引，也不替代员工05的 Release 现场验收

## 现场前置条件

1. 必须使用员工01冻结、员工05发布的唯一 clean Release；不要使用个人开发构建冒充 Release。
2. 从真实 ImportSource/Revision/ImportedMessage 开始，显式请求知识建议；至少 Confirm 一条带真实 ImportContent EvidenceRef 的 FocusFrame proposal，并 Reject 另一条 proposal。不得手工写 SQLite、Vault、EvidenceRef 或候选。
3. 选择已确认生成的 branch-local entity，通过正式 generation receipt 进入 Closed 候选，再用四个不同候选分别完成 `confirm`、`promote`、`reject`、`delete`。
4. 完成动作后执行一次 Reopen → Close，确认已处理候选没有重新出现；随后彻底退出 MindScape，再启动一次并核对提案审核、决定历史与最终 UI。
5. 退出应用后再运行审计工具，避免把正在写入的 WAL/Vault 瞬间状态误判为失败。

## 脱敏只读审计

在 `desktop/src-tauri` 目录执行。路径只通过环境变量传入，不会写入 JSON；输出中的 decision、FocusFrame、candidate 只保留 SHA-256 前 12 位，且不输出名称、正文、EvidenceRef ID、Key、数据库路径或 Vault 路径。

```powershell
$env:MINDSCAPE_B5_DATABASE_PATH = '<本次唯一 Release 的 app-data>\mindscape.sqlite3'
$env:MINDSCAPE_B5_VAULT_ROOT = '<本次唯一 Release 的 app-data>\vault'
cargo run --locked --release --example b5_focus_promotion_audit > '<B5 证据目录>\b2-persistence-sanitized.json'
if ($LASTEXITCODE -ne 0) { throw 'B2 persistence audit failed' }
```

通过标准：

- 进程退出码为 `0`，JSON 的 `violations` 为空。
- `integrityCheck = "ok"`、`foreignKeyViolations = 0`、`schemaVersion >= 18`、`decisionTablePresent = true`、`generationTablePresent = true`、`proposalRequestTablePresent = true`、`proposalReviewTablePresent = true`。
- `proposalRequestCount > 0` 且 `proposalReviewCount > 0`；request receipt 必须先于生成结果存在、保存稳定 `generationRunId`，同 `requestId` 精确重试不得产生第二次运行。Confirm 必须产生实体与 EvidenceRef，Reject 必须只有 review receipt 而没有实体、证据、索引或 Vault 派生物。
- `generationCount > 0` 且每条 generation `valid = true`；receipt 中候选数量与 source revision 数量一致，memory/lifecycle 均只递增一次。现场仍须用同一 `generationId` 模拟“服务端成功但客户端未收到响应”的重试，审计不能代替该动作证据。
- `knowledgeInventory.entityCount > 0`、`embeddedEvidenceCount > 0`、`importEvidenceCount > 0`；`materializedEvidenceCount`、`evidenceVaultFileCount` 均等于 `embeddedEvidenceCount`，`resolvedImportEvidenceCount` 等于 `importEvidenceCount`。这证明实体内 EvidenceRef 已进入独立证据表和 Vault 来源页，且每个 `ImportContent` 都能解析到同一会话的真实 ImportSource、Revision 与 locator。
- `knowledgeInventory.entities` 只输出实体 ID 指纹、状态、作用域和计数；每个保留实体的 `provenanceComplete = true`，不得用手工写 SQLite 或仅有前端对象的候选替代。
- `allFourActionsPresent = true`，且 `actionsPresent` 同时包含四动作。
- 每条 `valid = true`；请求/终态 projection 不可变字段一致，决定 revision 为 `1`。
- Confirm/Promote 的源实体可检索；Promote 的目标实体、Evidence、FTS/Vector 与 Vault 投影完整。
- Reject 不进入 FTS/Vector；Delete 的源实体、关系、FTS/Vector、Vault 实体文件已清理，`deletedCandidateVaultLinks = 0`，但 decision tombstone 仍存在。
- `pendingVaultTransactions = 0`、`pendingEntityDeleteTransactions = 0`、`pendingDiscussionTransactions = 0` 且 `pendingImportKnowledgeReviewTransactions = 0`，证明进程重启后没有遗留未恢复的 Vault journal。

若向量模型版本已变化，`vectorIndexStatus = "stale"` 是允许的显式降级，但 FTS 和 Evidence/Vault 仍必须完整；不能把缺失状态静默当作通过。

## 故障与并发门禁

真实用户数据只做上述只读审计。下列故障注入只允许在临时验收目录或自动化测试夹具执行：

1. **陈旧版本**：读取候选后，通过 Reopen/Close 推进 lifecycle，再提交旧 revision；命令必须失败，且 SQLite decision、实体、FTS/Vector、Vault 均无部分写入。
2. **幂等重试**：同一 `decisionId` 和完整输入重试，必须返回同一 projection，数据库仍只有一条记录。
3. **键冲突**：同一 `decisionId` 换输入，或用不同 ID 决策同一候选，必须失败且不覆盖旧决定。
4. **崩溃窗口**：仅使用自动化夹具模拟 Vault 已写而 SQLite 未提交、以及 SQLite 已提交两种窗口；重启后前者恢复 pre-image，后者保留新投影，两者都清理 journal。
5. **提案请求/审核重放**：相同 `requestId` 或 `decisionId` 与完整 typed input 精确重试必须返回原 receipt；复用 ID 改变 source/hash/scope/proposal/decision 必须冲突。Confirm 的 Vault-first journal 在 SQLite 失败时恢复 pre-image，Reject 不得创建 journal 或知识派生物。

对应确定性回归：

```powershell
cargo test --locked focus_promotion -- --nocapture
cargo test --locked focus_promotion_journal_rolls_back_uncommitted_and_keeps_committed_projection -- --nocapture
cargo test --locked import_knowledge_proposal -- --nocapture
cargo test --locked import_review_journal -- --nocapture
```

自动化回归是故障注入证据，真实 clean Release 的四动作、Reopen、进程重启和脱敏 JSON 仍需员工05在同一轮 B5 记录。

## 证据交接

- 员工05保存命令退出码和 `b2-persistence-sanitized.json`，并与同一 Build ID、EXE SHA-256、真实 UI 截图归档；审计文件只保留计数和短指纹，不记录提案正文、导入正文、绝对路径、完整 ID 或 Key。
- 员工01依据 `violations = []`、唯一 clean Release 身份和跨进程 UI 结果决定是否关闭 B5；员工02不单独发布 Release，也不单独宣称 B5 已关闭。
- 若审计失败，只共享脱敏 JSON 中的指纹和计数；原始数据库、Vault 正文和真实路径不得进入 PEC。

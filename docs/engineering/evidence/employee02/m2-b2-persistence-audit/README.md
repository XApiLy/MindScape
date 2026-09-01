# B5｜B2 原子持久化脱敏审计说明

> 负责人：员工02（桌面端与数据）  
> 适用范围：B5 唯一 clean Tauri Release 的 `confirm / promote / reject / delete` 纵向验收  
> 工具性质：只读核对；不会写入 SQLite、Vault 或索引，也不替代员工05的 Release 现场验收

## 现场前置条件

1. 必须使用员工01冻结、员工05发布的唯一 clean Release；不要使用个人开发构建冒充 Release。
2. 在同一验收数据库中，用四个不同候选分别完成 `confirm`、`promote`、`reject`、`delete`。
3. 完成动作后执行一次 Reopen → Close，确认已处理候选没有重新出现；随后彻底退出 MindScape，再启动一次并核对决定历史与最终 UI。
4. 退出应用后再运行审计工具，避免把正在写入的 WAL/Vault 瞬间状态误判为失败。

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
- `integrityCheck = "ok"`、`foreignKeyViolations = 0`、`schemaVersion >= 16`、`decisionTablePresent = true`。
- `allFourActionsPresent = true`，且 `actionsPresent` 同时包含四动作。
- 每条 `valid = true`；请求/终态 projection 不可变字段一致，决定 revision 为 `1`。
- Confirm/Promote 的源实体可检索；Promote 的目标实体、Evidence、FTS/Vector 与 Vault 投影完整。
- Reject 不进入 FTS/Vector；Delete 的源实体、关系、FTS/Vector、Vault 实体文件已清理，`deletedCandidateVaultLinks = 0`，但 decision tombstone 仍存在。
- `pendingVaultTransactions = 0`、`pendingEntityDeleteTransactions = 0` 且 `pendingDiscussionTransactions = 0`，证明进程重启后没有遗留未恢复的 Vault journal。

若向量模型版本已变化，`vectorIndexStatus = "stale"` 是允许的显式降级，但 FTS 和 Evidence/Vault 仍必须完整；不能把缺失状态静默当作通过。

## 故障与并发门禁

真实用户数据只做上述只读审计。下列故障注入只允许在临时验收目录或自动化测试夹具执行：

1. **陈旧版本**：读取候选后，通过 Reopen/Close 推进 lifecycle，再提交旧 revision；命令必须失败，且 SQLite decision、实体、FTS/Vector、Vault 均无部分写入。
2. **幂等重试**：同一 `decisionId` 和完整输入重试，必须返回同一 projection，数据库仍只有一条记录。
3. **键冲突**：同一 `decisionId` 换输入，或用不同 ID 决策同一候选，必须失败且不覆盖旧决定。
4. **崩溃窗口**：仅使用自动化夹具模拟 Vault 已写而 SQLite 未提交、以及 SQLite 已提交两种窗口；重启后前者恢复 pre-image，后者保留新投影，两者都清理 journal。

对应确定性回归：

```powershell
cargo test --locked focus_promotion -- --nocapture
cargo test --locked focus_promotion_journal_rolls_back_uncommitted_and_keeps_committed_projection -- --nocapture
```

自动化回归是故障注入证据，真实 clean Release 的四动作、Reopen、进程重启和脱敏 JSON 仍需员工05在同一轮 B5 记录。

## 证据交接

- 员工05保存命令退出码和 `b2-persistence-sanitized.json`，并与同一 Build ID、EXE SHA-256、真实 UI 截图归档。
- 员工01依据 `violations = []`、唯一 clean Release 身份和跨进程 UI 结果决定是否关闭 B5；员工02不单独发布 Release，也不单独宣称 B5 已关闭。
- 若审计失败，只共享脱敏 JSON 中的指纹和计数；原始数据库、Vault 正文和真实路径不得进入 PEC。

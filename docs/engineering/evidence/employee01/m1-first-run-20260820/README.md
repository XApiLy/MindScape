# 员工01 M1 首次空状态与真实首答验收

## 验收范围

- Build ID：`20260820-155308-m1-clean-gate-98e12706c2f4`
- 来源提交：`98e12706c2f4`；`sourceTreeDirty=false`
- 程序：`D:\Project\MindScape\artifacts\acceptance\versions\20260820-155308-m1-clean-gate-98e12706c2f4\mindscape-desktop.exe`
- 程序 SHA-256：`6FE02B0BD551288805B8034AE39DB19528815F901F24D1C69471865985ABA59A`
- 实测时间：2026-08-20 16:47—16:57（UTC+8）

## 安全隔离与恢复

- 正常关闭应用后，将原应用数据目录原子重命名为 `com.mindscape.desktop.m1-backup-20260820-164707`；备份时数据库 SHA-256 为 `A242722D07F68CD7AA96A9F2F4CDE180C333BB4C31B6AE380549BFB559E28A03`。
- 验收结束后先关闭测试进程，将隔离测试目录保留为 `com.mindscape.desktop.m1-first-run-20260820-164707`，再把原目录原子恢复；恢复后、重新启动前的数据库 SHA-256 与备份值一致。
- 重新启动后原库只读复核为 14 个会话、27 个节点，`PRAGMA integrity_check=ok`、外键违规 0；应用正在同一 clean Build 上运行。SQLite 在运行中会正常写入 WAL/检查点，因此不把应用启动后的数据库文件哈希当作恒定值。
- DeepSeek 凭据位于操作系统安全凭据库，不属于应用数据目录。隔离与恢复过程中未读取、导出、复制或删除完整凭据。

## 真实操作与结果

1. 在不存在应用数据目录的条件下启动同一 clean Build，界面显示“还没有会话”和首次开始页；新库中会话、节点、运行均为 0。截图：[01-empty-state.png](./01-empty-state.png)。
2. 创建“新会话 1”，输入不敏感的最小诊断问题并发送。Windows 输入法改变了截图中英文诊断文本的空格/大小写，但不影响验收对象，也没有输入任何秘密。
3. 应用沿用系统安全凭据库中已经验证过的 DeepSeek 配置，完成真实 API 请求；节点显示 `deepseek-v4-flash` 和“已完成”，回答正文为 1 个字符。截图：[03-first-answer-completed.png](./03-first-answer-completed.png)。
4. 隔离测试库只读事实：1 个会话、1 个节点、0 条边、1 个 ContextSnapshot、1 个 ModelRun；运行状态 `completed`、`last_sequence=4`、`pending/streaming=0`，provider/model 为 `deepseek/deepseek-v4-flash`，数据库完整性正常。

凭据配置动作此前已有真实 `GET /models`、有效/无效凭据边界及真实 DeepSeek 回答证据；本次在不接触完整密钥的前提下补齐“空应用数据 → 创建首个本地会话 → 首次真实回答”。两组证据共同闭环统一验收脚本 1，并在同一 clean Build 中复演脚本 2 的根节点真实回答。

## 安全扫描

- 对恢复后的 Roaming 数据、隔离测试数据和 Local 日志/Crashpad 执行只读常见秘密模式扫描：341 个文件中 307 个可读，命中文件 0。
- 34 个运行中不可读文件不计作已扫描；两份 SQLite 均已成功读取并完成完整性检查。仓库秘密扫描另由 `scripts/check-secrets.mjs` 在提交前执行。
- 脱敏结构化事实见 [sanitized-first-run-facts.json](./sanitized-first-run-facts.json)，不包含请求正文、响应正文、完整凭据或日志原文。

## 结论

M1 脚本 1：**通过**。原用户数据已恢复并重新打开；隔离测试副本暂时保留以便审计，未删除任何原数据。

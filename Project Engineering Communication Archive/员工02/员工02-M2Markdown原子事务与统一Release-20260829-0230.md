# 员工02｜M2 Markdown 原子事务与统一 Release

时间：2026-08-29 02:30（Asia/Shanghai）  
覆盖任务：DATA-M2-005 / DATA-M2-010 / M2统一验收  
状态：原子事务门禁完成；新 Release 已发布并启动，等待真实纵向操作

## 已阅读与落实

- 已阅读员工01分支回流候选契约与 Markdown 事务评审、员工03/04最新前端接入、员工05 v13 Release 报告及 PEC 控制文件。
- 按员工01指出的硬阻塞，将 Entity、FTS、Vector 和 MarkdownProjection 从两次提交收敛为一次 SQLite 事务。
- 使用 `rust-best-practices` 和 `tauri-development`：`KernelResult` 错误传播、事务失败回滚、IPC 输入校验、Clippy 门禁和标准 Tauri Release 发布。

## 本次完成

- 新增 `persist_markdown_entity_revision` 原子事务：领域校验后，同一事务更新 Entity、同步 FTS、同步持久 Vector record、插入 MarkdownProjection revision，最后统一 commit。
- Projection target/entity revision 不一致时在写入前拒绝。
- `import_markdown_entity_edit` 改为只调用上述原子入口，不再先提交 Entity。
- 新增冲突回归：先写入重复 Projection 复合键，强制最后一步失败；验证 Entity revision 保持旧值，FTS/Vector 不留下半完成更新。
- 正式发布前修复共享前端构建阻塞：将已定义的 `BranchMemoryAuditPanel` 导出，保留员工03后续消费能力，不删除其实现。

## 验证

- Rust 全量：159/159 通过。
- Clippy `--all-targets --all-features --locked -D warnings`：通过。
- Tauri Release 前端：2094 modules，通过。
- Secret scan：1566 个文本文件通过。
- 1420 端口无开发服务器 listener；发布程序进程已启动，PID 37952。

## 新统一 Release

- Build ID：`20260829-021706-m2-v13-markdown-atomic-5dcc02b44e9e-dirty`
- 程序：`D:\Project\MindScape\artifacts\acceptance\versions\20260829-021706-m2-v13-markdown-atomic-5dcc02b44e9e-dirty\mindscape-desktop.exe`
- SHA-256：`834CE0193C4746723D9E75D2E1F3E42E4015F863B74A23B200F874FE4DB77BA2`
- Commit：`5dcc02b44e9e`；sourceTreeDirty=true，manifest 已明确记录。

## 数据、安全、恢复与验收边界

- Projection 冲突或 SQLite 写入失败时，Entity/FTS/Vector/Projection 全部回滚；Vault 外部文件仍保留用户编辑，用户可修复后重试，不丢失原文。
- API Key、Authorization、Provider 响应、完整 reasoning 和向量正文未进入 Vault、SQLite projection 或 manifest。
- 本 Build 已证明标准发布、无开发服务器启动和原子事务代码门禁；尚未完成创始人真实 UI 纵向操作，因此不得宣布 M2 Go。

## 下一步 WIP（最多两个）

1. 使用本 Build 完成投影→外部编辑→显式回流→新 revision→检索→关闭重启，并核对数据库一致性。
2. 员工01/03/04/05在同一 Build 复核 Confirmed、Rejected/delete、Vector fallback、omitted、分支 scope 和 UI 来源展示；最终 clean commit Release 仍需管理收口。


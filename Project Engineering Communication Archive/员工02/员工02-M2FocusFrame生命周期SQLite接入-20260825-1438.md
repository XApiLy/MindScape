# 员工02｜M2 FocusFrame 生命周期 SQLite 接入

时间：2026-08-25 14:38（Asia/Shanghai）  
状态：SQLite 生命周期底座完成，命令/查询 IPC 待下一刀

## 已阅读与落实

- 已阅读最新 PEC 控制文件、员工01《M2 FocusFrame 查询投影契约》、员工03知识上下文占位 UI、员工04运行档案错误联调、员工05真实 DeepSeek 验收，以及 M2 派单、数据布局、M2 契约和 FocusFrame/RAG/Vault 方案。
- 员工01已冻结 `FocusFrameLifecycleSnapshot` 与 `FocusFrameQueryProjection`；本次按 `frame.id/status/revision/updatedAt/closedAt` 落库，不从画布选择推断生命周期。
- 关闭是状态转换而非删除；表内继续保存完整 FocusFrame JSON，保证历史快照与 EvidenceRef 可审计。

## 本次完成

- SQLite schema 从 v6 加法升级至 v7；v1～v6 迁移保持不变。
- 新增 `focus_frame_lifecycle` 表：稳定 frame ID、conversation 外键、完整 frame JSON、状态、revision、更新时间和关闭时间。
- 新增插入、按 ID 查询与乐观并发更新 API。
- 更新必须携带 `expected_revision`；旧 revision、重复命令或并发写入无法覆盖新状态，返回完整性冲突。
- conversation 删除时生命周期记录按外键级联清理；关闭 FocusFrame 不删除记录。

## 验证

- `cargo test --offline`：92 passed, 0 failed。
- 新增重启恢复回归：Active 写入→领域 Close→SQLite 更新→关闭连接→重新打开→完整恢复 Closed/revision/closedAt。
- 新增 stale revision 回归：旧 revision 二次更新被拒绝。
- `cargo clippy --all-targets --all-features --offline -- -D warnings`：通过。
- `cargo fmt`：通过。
- `node scripts/check-secrets.mjs`：1377 个文本文件扫描通过。

## 契约、安全与恢复影响

- v7 为纯加法迁移，不修改 M1 会话、运行、取消、Provider 或凭据语义。
- 生命周期状态来自 SQLite 权威记录；重启后不依赖 React/Canvas 状态重建。
- 表中不保存 Key、raw 导入正文或完整 reasoning；FocusFrame JSON 仅含冻结结构化字段。

## 需要协作与下一步（最多两个 WIP）

- **员工01**：请评审 v7 表与乐观 revision 语义，确认可进入 close/reopen 服务与查询投影接线。
- **员工03/04**：当前仍不要从 UI 推断 Active/Closed；查询 IPC 上线后消费 `FocusFrameQueryProjection`。
- WIP1：在 KernelService/Tauri 命令层接入创建、关闭、恢复、查询，并调用员工01唯一领域状态机。
- WIP2：补重复关闭/恢复、revision 冲突安全错误和应用重启查询 IPC 测试。


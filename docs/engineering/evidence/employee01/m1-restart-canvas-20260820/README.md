# 员工01 M1 重启恢复复核

## 验收范围

- Build ID：`20260820-155308-m1-clean-gate-98e12706c2f4`
- 来源提交：`98e12706c2f4`
- 程序 SHA-256：`6FE02B0BD551288805B8034AE39DB19528815F901F24D1C69471865985ABA59A`
- 复核时间：2026-08-20 16:20（UTC+8）

## 只读事实

- 数据库完整性：`PRAGMA integrity_check=ok`；外键违规 0。
- `pending/streaming=0`；全库 27/27 Node 都有 provider/model。
- 终态 Node：`cancelled=7`、`failed=3`；最新 `application_interrupted` 运行仍保留 `failed`、部分内容长度 1392、viewport 记录。
- 最新两次用户取消均保留唯一 `cancelled` Node/Run、父节点和 Edge；无重复主键或重复终态。

## 位置持久化与重启结果

- 创始人在同一 Build 中将取消节点拖动到新位置，等待保存后通过正常窗口关闭退出。
- 重新打开同一 Build 后，位置记录仍存在：该会话 5 个 Node、3 条 Edge、3 个 `canvas_node_positions`、1 个 viewport；重复 Edge 分组 0。
- 所有该会话 Node 均有 provider/model；`integrity_check=ok`；外键违规 0；`pending/streaming=0`。
- 最新取消节点位置为脱敏数值 `x=14、y=773`，viewport 仍有有效 `x/y/zoom`；未读取正文或凭据。

## 结论

脚本 8：**通过**。运行终态、历史、模型映射、Edge、节点位置和 viewport 在关闭/重新打开后保持；强退会话仍作为单独运行恢复事实保留，不产生永久 pending。

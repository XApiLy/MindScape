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

## 未闭合项

- 取消 Node 的 `canvas_node_positions` 记录为 0。
- `application_interrupted` Node 的 `canvas_node_positions` 记录为 0；其 conversation 有 viewport，但没有该 Node 的持久位置。
- 因此目前只能签署“运行终态、历史、模型映射、Edge 和 viewport 恢复事实”，不能签署“位置持久化与画布联合恢复”完整通过。

## 结论

脚本 8 维持 **部分通过 / No-Go**。需要在同一 Build 中对取消或强退会话至少拖动并保存一个节点位置，关闭程序后重新打开，交叉核对该 Node 的位置、Edge、viewport 与唯一性；本证据不读取正文或凭据。

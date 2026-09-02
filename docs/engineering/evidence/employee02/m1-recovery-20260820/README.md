# M1真实强退恢复与主动停止数据验收

> 执行时间：2026-08-20（UTC+8）  
> 负责人：创始人执行Windows动作；员工02复核SQLite事实  
> 结果：强退恢复通过；主动停止通过

## 窗口证据

- 创始人提交的强退恢复截图显示：已收到的长文内容保留，界面显示“上次生成被应用退出中断”，诊断信息为`MindScape exited before this response finished.`，并提供显式重试。
- 创始人提交的主动停止截图显示：运行状态为“已停止”，零正文时明确提示“本次没有保留部分内容”，并提供显式重试。
- 两种状态没有互相混淆，也没有将应用中断标记为用户取消或普通完成。

## SQLite只读聚合证据

查询使用只读URI与`PRAGMA query_only=ON`；未读取或保存正文、请求JSON、事件正文、内部ID或API Key。

| 用例 | 状态 | 终态 | Provider code | 序列 | 部分内容长度 | retained |
|---|---|---|---|---:|---:|---|
| 流式中强退并重启 | `failed` | `failed` | `application_interrupted` | 775 | 1392 | true |
| 用户立即停止（零正文） | `cancelled` | `cancelled` | 无 | 2 | 0 | false |
| 用户停止（已有正文） | `cancelled` | `cancelled` | 无 | 21 | 34 | true |

全库复核：`integrity_check=ok`、外键违规0、`pending/streaming=0`。

## 验收结论

- 流式中强制退出后，启动扫描正确收口为Frozen V1的`failed + application_interrupted`，保留部分内容且没有永久pending：通过。
- 用户主动停止正确收口为唯一`cancelled`；零正文和有正文的`partialContentRetained`均与实际长度一致：通过。
- 窗口与SQLite对两类终态的判断一致：通过。

## 边界

- 本证据关闭员工02的“流式中强退、启动扫描、部分内容、无永久pending”数据门禁。
- DeepSeek空正文问题由员工05另行修复；应使用Build ID `20260820-142838-prov-empty-response-fix-0cb1c0053b5a-dirty`复验，不以本次较早窗口替代该修复验收。
- M1整体仍受无效Key、超时、异常断流、脱敏矩阵和clean release阻塞。

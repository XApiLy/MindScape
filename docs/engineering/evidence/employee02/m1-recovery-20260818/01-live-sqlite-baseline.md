# M1真实SQLite脱敏基线

> 采集时间：2026-08-18 18:38（UTC+8）  
> 负责人：员工02  
> 用例：M1恢复与数据一致性验收前置基线  
> 结果：基线通过；取消、强退、重启和真实视口仍待人工执行

## 证据边界

- 数据源：`<Tauri app_data_dir>/mindscape.sqlite3`，使用SQLite只读URI打开。
- 未读取或记录API Key、消息正文、提示词、`request_json`、部分回答正文、事件原始JSON或本机用户名路径。
- 数据库文件未复制进仓库；以下仅保存聚合计数和完整性结果。
- 采集时数据库大小：163840 bytes；SHA-256：`6E2A2C37095F12B7C50F940B5CBBBAAF81BA0E0179A9F5DF6DF81CDA5BD354B9`。

## 聚合结果

| 检查项 | 结果 |
|---|---:|
| Schema版本 | 5 |
| `PRAGMA integrity_check` | `ok` |
| 外键违规 | 0 |
| 会话 | 1 |
| 节点 / ContextSnapshot | 2 / 2 |
| ModelRun / ModelRunEvent | 2 / 24 |
| 运行状态 | `completed`: 2 |
| 节点状态 | `completed`: 2 |
| 永久`pending`或`streaming`候选 | 0 |
| 重复幂等键 | 0 |
| 重复运行序列 | 0 |
| 孤儿ModelRun | 0 |
| CanvasViewport | 0 |

事件类型聚合：

| 事件类型 | 数量 |
|---|---:|
| `started` | 2 |
| `text_delta` | 18 |
| `usage_updated` | 2 |
| `completed` | 2 |

两个`completed`终态均包含`inputTokens`、`outputTokens`、`cachedInputTokens`、`costMicrounits`字段，且每个终态至少有一个非空用量值。本证据只确认用量已进入事件与终态JSON；重启后UI投影一致性仍需窗口验收。

## 当前判断

- 首次真实DeepSeek运行的数据库事实一致：没有永久pending、重复运行、重复序列或孤儿节点。
- 用量落库已经存在真实证据，不再列为“数据库是否写入未知”；界面显示与重启投影仍是No-Go。
- `canvas_viewports`当前为0，员工03的真实双会话移动、关闭和重启用例尚未产生数据库证据。
- 本基线是后续取消、强退与重启前的对照点；每个用例执行后必须重新采集并比较状态、事件数和终态唯一性。

## 后续用例

1. 真实长回答停止：期望新增一个稳定Node/Run、唯一`cancelled`终态、无终态后delta，部分内容按事件事实保留。
2. 流式中强退：期望重启扫描将残留运行收口为`failed + application_interrupted`，无永久pending且历史completed运行不变。
3. 真实视口：会话A/B分别写入一条最终视口，切换flush与重启读取后数据库仍为每会话唯一记录。

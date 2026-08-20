# PROV-013 真实 DeepSeek 取消证据

> 日期：2026-08-19（UTC+8）  
> 负责人：员工04（界面与交互）、员工05（Provider 与网络事实）  
> 结论：真实取消的数据与网络不变量通过；重复投影已修复，并在统一 Tauri release 验收版本中完成单卡复验。

## 现场操作与界面结果

- Provider / model：`deepseek / deepseek-v4-flash`。
- 在真实流式回答生成期间点击“停止生成”，界面进入 `CANCELLED / 已停止`。
- 已收到的部分回答继续显示，并提供明确的“重试”入口。
- 截图未打开设置页，也未出现 API Key：`01-original-cancelled-reading-view.png`。
- 修复后 release 截图：`03-release-single-card-recheck.png`，SHA-256 `D34BFE4AE7A32B3DC5E48A62FDBF81B910D58AB599ECBBFCBBD3BB7E40F43C93`。

## 只读数据核对

- 运行终态与节点终态均为 `cancelled`，取消原因是 `userRequested`。
- 共 126 个事件，唯一终态事件 1 个；终态后新增 delta 为 0。
- 部分回答长度为 230，助手消息存在，`partialRetained=true`。
- 同一节点只有 1 个运行；事件序列无重复。
- SQLite `integrity_check=ok`，外键违规为 0；停止后等待 10 秒再次读取，结果保持稳定。
- 脱敏运行指纹：`f01bb1192188`（SHA-256 前 12 位）。

只读核对没有读取或保存消息正文、请求 JSON、事件正文、数据库路径、内部 ID 或 API Key。结构化聚合结果见 `02-sanitized-cancel-facts.json`。

## 发现的问题与修复

原截图中，同一已取消运行在阅读视图出现两次：上方是已经落库的 `NodeCard`，下方是 `ActiveRunCard`。数据库只有一个 Node 和一个 Run，因此这是前端时间线投影重复，不是数据重复。

修复引入 `projectChatTimeline`：当当前或恢复运行的 `nodeId` 已存在于会话图时，在原节点位置使用运行卡替换节点卡；只有节点尚未进入图时才追加运行卡。这样保留取消状态、部分文本和重试入口，同时避免重复和位置跳动。

## 验收边界

- 已通过：真实 Provider 取消、唯一终态、无终态后 delta、部分内容保留、数据完整性、重试入口存在。
- 已通过：阅读视图同一节点/运行的重复投影修复；自动测试与真实 release 窗口均确认只显示一张“已停止”卡片。
- 修复后截图中的运行卡仍标注 `deepseek-v4-flash · deepseek`；底部 `mock-stream-v1` 是下一次发送的当前选择，不改变已持久化运行来源。

## 正式复验版本

- Build ID：`20260819-144548-chat-012-single-card-recheck-0cb1c0053b5a-dirty`。
- 程序路径：`artifacts/acceptance/versions/20260819-144548-chat-012-single-card-recheck-0cb1c0053b5a-dirty/mindscape-desktop.exe`。
- SHA-256：`AF0378DAF3D4B680A60272283CCA047698CCC71008A0CF52494FF090185D30D4`。
- 来源提交：`0cb1c0053b5a`，工作树为 dirty，已由 manifest 明示。
- 构建方式：Tauri release no-bundle；已在没有开发服务器时独立显示 MindScape 工作区。
- 进程核对：创始人截图时运行路径与上述不可变发布目录一致。

此前引用的标准 `target/debug` 程序 SHA-256 `1ACC36F5CDC09C37AE9CECBEBE96B7FC02555DAD348677382F6B15D3B5EBCAFB` 会访问 `localhost:1420`，不是有效验收程序，相关交付结论已作废。

统一验收脚本 4 已闭环通过；M1 仍因其他脚本缺口保持 No-Go。

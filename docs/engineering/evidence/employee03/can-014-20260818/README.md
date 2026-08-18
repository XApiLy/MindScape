# CAN-014 真实 Windows 双会话视口恢复证据

> 执行时间：2026-08-18 18:32～18:44（UTC+8）  
> 应用：`desktop/src-tauri/target/debug/mindscape-desktop.exe`  
> 环境：Windows 真实 Tauri 窗口，1936 × 1048 最大化窗口  
> 结果：双会话切换与正常关闭/重启视口恢复通过；取消、失败和强退恢复状态仍待联合验收。

## 用例与结果

| 步骤 | 操作 | 可见结果 | 证据 |
|---|---|---|---|
| 1 | 启动应用并打开会话 A“新会话 1” | 初始 86%，2 个完成节点、1 条边 | `01-app-launched.png` |
| 2 | A 放大两级并向右下平移 | A 为 106%，节点与边保持稳定 | `02-conversation-a-moved.png` |
| 3 | 新建会话 B“新会话 2” | B 初始 86%，与 A 独立 | `03-conversation-b-created.png` |
| 4 | B 完成一次 Mock 回答，缩小两级并向左上平移 | B 为 66%，只有 1 个完成节点 | `04-conversation-b-moved.png` |
| 5 | 切回 A | A 恢复 106%，仍为 2 个节点、1 条边 | `05-switch-back-a.png` |
| 6 | 正常关闭窗口并重新启动 | 默认打开 B，恢复 66% 与同一节点位置 | `06-restart-default-conversation-b.png` |
| 7 | 重启后切回 A | A 恢复 106%，2 个节点、1 条边及位置不变 | `07-restart-switch-conversation-a.png` |

## SQLite 只读核对

数据库：Windows 应用数据目录中的 `mindscape.sqlite3`。查询只读取会话 ID、视口、节点数和边数，未读取正文、凭据或 Provider 请求。

| 会话 | conversation_id | x | y | zoom | 节点 | 边 |
|---|---|---:|---:|---:|---:|---:|
| 新会话 2 | `conversation-70b41e07-ed74-49b5-9a30-5a4b21a52e4f` | 417.76 | 138.56 | 0.66 | 1 | 0 |
| 新会话 1 | `conversation-7aaeaa22-7d09-4327-a184-2ee9bc90a09d` | 109.2093 | 131.8837 | 1.06 | 2 | 1 |

## 边界

- 本次签署 CAN-014 的“真实双会话隔离、切换、正常关闭与重启恢复”。
- 成功状态的稳定节点与正确连边通过；未出现重复节点或位置跳变。
- 尚未取得真实 `cancelled`、`failed` 和强制退出恢复后的对应画布证据，因此 CAN-012 的四状态联合验收仍为 No-Go。
- 验收使用本地 Mock Provider 生成 B 的单个节点，不消耗真实额度，不接触或展示 API Key。

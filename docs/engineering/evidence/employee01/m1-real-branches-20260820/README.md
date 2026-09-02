# 员工01 M1 真实分支与 ContextSnapshot 证据

## 验收范围

- Build ID：`20260820-155308-m1-clean-gate-98e12706c2f4`
- 来源提交：`98e12706c2f4`
- 程序：`D:\Project\MindScape\artifacts\acceptance\versions\20260820-155308-m1-clean-gate-98e12706c2f4\mindscape-desktop.exe`
- 程序 SHA-256：`6FE02B0BD551288805B8034AE39DB19528815F901F24D1C69471865985ABA59A`
- `sourceTreeDirty=false`；真实窗口无开发服务器运行。

## 操作与结果

创始人在上述 Build 的真实窗口中，从已完成节点分别执行“深入”和“发散”，各发送一次真实问题并等待 DeepSeek 完成。操作后只读核对 SQLite；未读取或保存问题正文、回答正文、请求 JSON 或凭据。

- 新增 `deepens`：1 个 Node、1 条 incoming Edge、1 个 ModelRun、1 个 ContextSnapshot，Node/Edge/Snapshot 分支类型一致，终态 `completed`。
- 新增 `diverges`：1 个 Node、1 条 incoming Edge、1 个 ModelRun、1 个 ContextSnapshot，Node/Edge/Snapshot 分支类型一致，终态 `completed`。
- 两条新分支均为 `deepseek / deepseek-v4-flash`，无永久 `pending/streaming`。
- `deepens` Snapshot 选择 2 条消息、无省略；`diverges` Snapshot 选择 3 条消息、记录 1 条省略，符合分支上下文差异。
- 全库分支聚合：`continues=23`、`deepens=1`、`diverges=1`；Snapshot 聚合完全一致。

## 数据完整性与安全

- `PRAGMA integrity_check=ok`；外键违规 0；`pending/streaming=0`。
- 证据只保存脱敏计数、长度、分支类型和 ID 指纹；未保存 API Key 或正文。

## 结论

- M1 验收脚本 3：**通过**。
- 脚本 8 仍待联合窗口证据：现有取消节点位置落库为 0，`application_interrupted` 失败节点只有 viewport，尚未证明取消/失败/强退后的节点、边、位置和 viewport 同时稳定。

结构化脱敏事实见同目录 `sanitized-branch-facts.json`。

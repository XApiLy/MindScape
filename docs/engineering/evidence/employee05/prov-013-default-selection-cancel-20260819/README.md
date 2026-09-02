# PROV-013 真实Provider默认选择与取消复验

> 日期：2026-08-19（UTC+8）  
> 负责人：员工05  
> 结论：修复启动默认选择后，DeepSeek真实流式与取消在同一正式release中通过。

## 验收版本

- Build ID：`20260819-150500-prov-013-mock-default-fix-0cb1c0053b5a-dirty`。
- 程序：`artifacts/acceptance/versions/20260819-150500-prov-013-mock-default-fix-0cb1c0053b5a-dirty/mindscape-desktop.exe`。
- SHA-256：`B0F424B5C55E1FEDC1E49577DA8001751F79A352E629366AC45436997C415606`。
- 来源提交：`0cb1c0053b5a`；`sourceTreeDirty=true`由manifest明确披露。
- 构建方式：Tauri release no-bundle；无`localhost:1420`监听时独立启动。

## 窗口事实

- 创始人在当前任务中提交截图确认：下一次运行模型为`deepseek-v4-flash`，Provider为`deepseek`。
- 流式输出期间执行停止后，单卡显示“已停止”，保留已收到内容并提供“重试”入口。
- 底部下一次模型仍为`deepseek-v4-flash / DeepSeek · 真实 API 可用`，没有静默回落Mock。
- 截图未打开设置页，不包含API Key；本报告不复制临时附件，只记录脱敏验收事实。

## SQLite只读聚合核对

- 数据源：`<Tauri app_data_dir>/mindscape.sqlite3`，以SQLite只读URI打开并启用`query_only`。
- 脱敏运行指纹：`a16ba48ffc4a`（Run ID SHA-256前12位）。
- Provider / model：`deepseek / deepseek-v4-flash`。
- ModelRun状态：`cancelled`；Node状态：`cancelled`。
- 部分内容长度：193字符；未读取或保存正文。
- 运行事件：104个；唯一终态为序号104的`cancelled`。
- 终态后`text_delta`：0。
- `PRAGMA integrity_check=ok`；外键违规0。
- 最后更新时间：`2026-08-19T07:11:15.611252600+00:00`。

## 验收边界

- 已通过：真实Provider自动选择、真实DeepSeek流式、用户取消、唯一终态、终态后无delta、部分内容保留、重试入口、Node/Run一致和SQLite完整性。
- 未覆盖：无效Key、模型不存在、断网、连接/首Token/空闲超时、异常断流、用量重启投影、日志与崩溃信息泄漏扫描。
- 本Build来自dirty工作树，只用于专项验收；M1最终Go仍需clean commit统一release。


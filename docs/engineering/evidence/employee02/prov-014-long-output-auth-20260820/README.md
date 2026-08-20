# PROV-014长文输出与错误Key数据复核

> 执行时间：2026-08-20（UTC+8）  
> 窗口操作：创始人  
> 数据复核：员工02  
> 结果：长文输出通过；错误Key安全识别通过

## 验收版本

- Build ID：`20260820-142838-prov-empty-response-fix-0cb1c0053b5a-dirty`。
- SHA-256：`03B9B611511FEC24035D41F0A4F72F8F12BB3BAAD2377C0F81F7159958CEEC0A`。
- 来源提交：`0cb1c0053b5a`，manifest已披露`sourceTreeDirty=true`。

## 窗口结果

- 创始人确认“不少于3000字”请求可以正常输出完整可见正文，不再出现`completed`但零正文。
- 创始人使用错误Key执行连接测试，设置页显示“Provider拒绝了当前API Key，请在模型设置中替换凭据并重新测试连接”。
- 错误提示未回显Key或Provider原始响应，且没有静默回退Mock。

## SQLite只读复核

查询使用只读URI与`PRAGMA query_only=ON`，未读取正文、请求JSON、事件正文、内部ID或Key。

- 最新真实运行：`completed`，唯一终态序列1648，持久化可见正文长度3084字符。
- `finishReason=stop`；Usage为input 22、output 1649、cached input 0。
- 全库ModelRun 18、Node 18，`pending/streaming=0`。
- `PRAGMA integrity_check=ok`，外键违规0。
- 错误Key连接检查后没有新增authentication failed ModelRun，符合连接测试不创建运行的边界。

## 结论与边界

- DeepSeek直接回答模式与空正文网关修复：真实通过。
- 错误Key分类、安全提示、不回显和零运行副作用：真实通过。
- 本专项Build来自dirty工作树，不替代最终clean release。
- 模型不存在、断网、连接/首Token/空闲超时、异常断流和日志/崩溃脱敏仍待关闭。

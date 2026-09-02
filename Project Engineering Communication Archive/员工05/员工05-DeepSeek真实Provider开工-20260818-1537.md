# 员工05｜DeepSeek真实Provider开工

> 提交时间：2026-08-18 15:37（UTC+8）  
> 覆盖任务：PROV-005～008、PROV-013～017  
> 状态：进行中  
> 当前分支：`docs/REL-002-pec-retention`（受共享工作树与只读Git元数据限制，目标功能分支尚未建立）

## 相比上一份的变化

- 当前唯一WIP切换为M1真实Chat Alpha，停止此前OpenAI优先与多Provider计划。
- 首发实现冻结为OpenAI-compatible协议底座，唯一真实验收端点为DeepSeek。
- 员工02已提供`start_model_run`、统一终态事务和启动恢复；员工05不再扩展前端三段式编排。

## 已阅读与落实

- 已阅读V1范围、2026-08-17产品共识、V1总路线、M1派单、内核总览、前五人责任书、RC1、安全威胁模型、Git与PEC制度、凭据边界及员工01～04最新PEC。
- 具体落实：Provider只消费冻结`ModelRunRequest`并输出统一事件；Key仅由Rust内部`CredentialService::resolve`取得；流式开始后不默认自动重试；未做真实契约测试的兼容端点不标记支持。
- 当前冲突：工作树位于文档治理分支且包含员工01/02未提交变更，`.git`只读，无法按建议建立`feat/PROV-008-openai-compatible-streaming`或Draft PR；实现将严格限制在Provider文件及必要注册边界。

## 本次完成与证据

- 完成强制阅读、M1 No-Go核验、最新统一运行边界和凭据接口审计。
- 旧员工05 PEC已移入`Project Engineering Communication Archive/员工05/`，当前目录恢复六文件窗口。

## 契约、数据和安全影响

- 暂无破坏性契约或schema变化。
- 计划新增DeepSeek Provider注册与Rust内部凭据解析，不向前端、请求契约、数据库或日志暴露秘密。

## 验证结果

- 开工快照完成后先运行PEC保留检查；工程测试以本轮实际执行结果为准。

## 风险与阻塞

- 当前环境网络受限，真实DeepSeek端点联调与新依赖下载可能受阻；先完成可离线验证的协议、SSE、错误和取消测试。
- 真实API Key尚未确认，不使用环境变量或源码密钥替代操作系统凭据。

## 下一步 WIP（最多两项）

1. PROV-005～008：安全请求配置、OpenAI-compatible SSE适配器与DeepSeek注册。
2. PROV-013～017：取消、分层超时、无自动计费重试、用量归一化与脱敏错误测试。

## 需要同事配合

- **员工01**：冻结Provider消费的统一运行契约与终态语义。
- **员工02**：评审CredentialService注入、取消和协调器错误终态映射。
- **员工04**：后续只从Provider注册表读取DeepSeek能力与可用状态，不硬编码厂商协议。

# MindScape 桌面端安全凭据边界

> 责任任务：DATA-012、DATA-013  
> 状态：第一周安全底座

## 存储与访问

- API Key 由操作系统原生安全凭据库保存；Windows 使用 Credential Manager，
  macOS 使用 Keychain，Linux 使用 Secret Service。
- 凭据由 `providerId + accountId` 的非秘密引用定位。
- Provider 只能在 Rust 运行时内部解析秘密，解析结果使用可清零内存包装。
- SQLite、React 状态、localStorage、日志和 Tauri 错误不得保存或返回秘密。

## Tauri 命令

前端只允许调用：

- `set_provider_credential`：新增或替换凭据。
- `has_provider_credential`：查询是否已配置，不返回秘密。
- `delete_provider_credential`：幂等删除凭据。

不存在读取秘密到前端的命令。凭据引用只允许 ASCII 字母、数字、连字符和
下划线，防止路径、服务名和账户名注入。

## 错误边界

所有 Tauri 命令返回稳定的结构化错误：`code`、`safeMessage`、`retryable`。
数据库内部错误、文件路径、系统凭据错误详情和 Provider 原始响应不得跨越
命令边界。

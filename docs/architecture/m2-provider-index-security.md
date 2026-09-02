# M2 Provider 与索引脱敏边界

> 对应任务：`PROV-M2-007`  
> 状态：M2 首版安全边界  
> 责任人：员工05（Provider / Index），员工01/02评审领域与数据边界

## 1. 目标与信任边界

Provider 响应、导入原文、Embedding 输入和向量索引均属于不可信或派生数据。Provider 适配器只能消费后端冻结的 `ModelRunRequest` 与 `ContextSnapshot`，只能返回统一 `ModelRunEvent` / `ProviderError`；索引只能返回候选和 provenance，不能成为事实源。

```text
React / UI 临时状态
        ↓ 结构化 Tauri 输入
应用服务与领域预检
        ↓ 冻结 ContextSnapshot / EffectiveRunProfile
Provider 或本地 Index 适配器
        ↓ 统一事件、候选和安全错误
SQLite / 原文层 / 操作系统凭据
```

## 2. 数据分类与允许流向

| 数据 | 允许保存位置 | 禁止流向 |
| --- | --- | --- |
| API Key | 操作系统凭据库、Rust 清零内存 | React、SQLite、localStorage、日志、PEC、Embedding metadata |
| 用户原文 / 导入原文 | 原文内容寻址存储、会话事件 | Provider 之外的未授权网络、索引 metadata、错误消息 |
| `ContextSnapshot` 选中内容 | 一次 ModelRun 的 Provider 请求 | Provider 自行查询数据库或扩大历史范围 |
| `reasoning_content` | 统一运行事件的受控临时消费 | 可见正文、知识实体、下一轮上下文、普通日志 |
| 向量 | 本地索引或未来受控索引存储 | 事实状态、EvidenceRef、跨作用域自动共享 |
| 向量 metadata | 模型版本、维度、源哈希、分块版本、稳定候选 ID | API Key、原文正文、完整 prompt、完整 Provider 响应 |
| Provider 错误 | `ProviderError` 白名单字段 | 原始响应正文、请求头、URL 中的秘密、Key、堆栈详情 |

## 3. Provider 网络控制

1. API Key 只由 Rust 凭据服务解析；请求结束后立即释放清零包装。
2. 真实 Provider 请求必须来自显式运行档案和冻结快照；不得由 UI 直接拼接历史或由 Provider 读库补上下文。
3. Embedding 默认走 `LocalHashEmbedding` 等本地实现；任何云端 Embedding 必须有独立能力声明、用户可见授权、费用边界和单独适配器，不能隐式切换。
4. 不支持的 reasoning、生成参数、工具或 response format 必须在发送前失败；不得删除参数后继续请求，也不得静默切换模型。
5. 重试只由上层按错误分类和幂等策略决定；流式开始后 Provider 不自动重试，避免重复计费和重复事实。

## 4. Index 控制

1. 索引是派生层；原文和 SQLite 领域记录是重建真相源。
2. 每个向量记录必须带 `modelVersion`、`dimensions`、`sourceHash` 和 `chunkVersion`。模型或分块版本变化时，旧向量不能与新向量静默混用，应重建或明确标记失效。
3. `VectorMatch`、FTS 命中和关系邻居只能形成 `KnowledgeRetrievalCandidate` 输入；确认状态、scope、FocusFrame 排除、证据和预算由领域编译器统一决定。
4. 向量不可用时可以回退 FTS / Relation，但必须返回 `RetrievalNotice`，让上层显示降级状态；不能假装仍然完成了语义检索。
5. 删除、否决、过期或跨作用域排除的实体不能仅依赖索引删除；查询层必须以 SQLite/领域状态再次过滤。
6. 索引重建失败不得覆盖旧原文或领域事实；应保留可恢复状态并报告安全错误。

## 5. 错误与日志脱敏

允许对用户展示的内容仅限：稳定错误码、分类、安全提示、可重试性和必要的重试等待时间。以下内容不得进入日志、事件、PEC 或前端：

- API Key、Authorization header、凭据引用的秘密部分；
- 完整 Provider 响应正文、请求 JSON、原始 SSE 帧和网络堆栈；
- 用户原文、完整 prompt、导入文件内容和未授权路径；
- 完整 reasoning 内容；
- 能反推出原文的向量调试转储。

调试需要时只允许使用脱敏 fixture、稳定 ID、哈希前缀和计数信息，并确保测试夹具不包含真实 Key 或私人原文。

## 6. 发布前检查

- `node scripts/check-secrets.mjs` 通过；
- Provider / Index 单元与集成测试覆盖未授权云端调用、错误脱敏、向量失效和回退提示；
- 真实 Tauri 验收只使用统一发布脚本生成的 Build ID；
- 任何发现 Key、原文或原始 Provider 响应跨越上述边界都属于 No-Go，必须先修复并重新审计。

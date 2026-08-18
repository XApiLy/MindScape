# MindScape V1 架构走查清单

> 状态：M1 Context/Runtime走查完成；M2 Import/Evidence待召开
> 主持：员工01  
> 对应任务：ARC-015

## 走查输入

- [V1 契约基线 RC1](../contracts/v1-contract-baseline.md)
- [V1 安全威胁模型](v1-threat-model.md)
- [产品内核与工程总览](../core-kernel-overview.md)

## 所有人必须解释

1. 原始事实、用户确认状态和系统派生的区别。
2. 为什么 UI 不是数据真相。
3. 为什么每次运行必须冻结 ContextSnapshot。
4. 为什么 Provider 不得选择历史或创建节点。
5. 失败、中断、重试和恢复分别产生什么状态。

## 消费方确认

### 员工02

- [x] 对象、事件和快照可持久化；schema v5保留完整运行请求并以加法迁移增加CanvasViewport。
- [x] 事件顺序和幂等边界明确；异载荷拒绝零副作用，同进程并发准备只产生一个Node/Run。
- [x] Key 不进入普通数据库或ModelRunRequest。

### 员工03

- [x] M1画布只依赖节点和运行查询投影。
- [x] 坐标不参与语义判断。
- [x] 分支枚举独立于渲染器表达；导入边在M2复核。

### 员工04

- [x] Chat 只消费统一运行事件。
- [x] 上下文入口读取真实快照。
- [x] 错误不会变成 assistant 消息。

### 员工05

- [x] Provider 可实现统一请求和事件。
- [x] 取消、超时、用量和错误可以映射。
- [x] 厂商差异由能力声明表达。

## M1走查结论（2026-08-18）

- Frozen：`mindscape.context.v1`、`mindscape.runtime.v1`、分支上下文语义。
- 唯一启动边界：`start_model_run(StartModelRunInput, eventChannel)`；UI三段式编排已移除。
- 可信预算：后端从实际Provider注册表读取模型窗口、减去输出预留，并在冻结快照前完成裁剪；UI不能提交窗口。零可用预算在任何Node/Run写入前拒绝。多语言估算对非ASCII按UTF-8字节保守计入，并包含每条Chat消息的固定封装开销。
- 幂等准备：相同键仅允许完全相同的执行输入；同载荷重放返回稳定Node/Run，异载荷安全拒绝。M1共享锁只串行化单进程准备阶段，不包住网络运行。
- 进程中断以本地 `failed` 事件表达，`providerCode=application_interrupted`，保留已持久化部分内容。
- 已通过真实门禁：DeepSeek连接、V4 Flash首次生成、正式回答和completed终态。
- 已进入共享树但待真实验收：schema v5视口按会话读取、节流保存与失败续写已通过自动测试；员工03需更新PEC并完成真实窗口多会话/重启证据。
- 剩余非契约阻断：员工02～05完成真实停止/cancelled、部分内容、用量、超时、异常断流、应用中断和重启恢复矩阵。
- Import/Evidence 未冻结，M2开始前必须由导入负责人和真实样本重新走查。

## 退出条件

- 所有阻断意见有任务 ID、负责人和期限。
- 没有团队维护平行 Node、Context、ModelRun 或 Error 模型。
- 契约状态从 RC1 更新为 Frozen V1，或明确保留未冻结项及原因。

# 员工05｜B5 建议 Producer 与 Request 链闭环

> 提交时间：2026-09-02 12:06（UTC+8）  
> 关闭项：`B2/B5-PROVIDER` suggestion producer 与 `request_import_knowledge_proposals` 真实命令缺口  
> 状态：**员工05本项已关闭；B5 / M2 仍为 No-Go，等待重启发现入口、员工01评审、clean Release 纵向验收**

## 已阅读与落实

- 已重读控制 PEC 顶部“2026-08-30：M2 Go 阻断收敛”和员工直接执行表，并读取员工01《B2B5实体生产前半段契约冻结》、员工02《B5导入实体提案原子持久化》、员工03《B5导入知识建议审核入口》、员工04《B5正式候选选择入口接入》。
- 落实 `mindscape.import-knowledge-proposal.v1`：导入仍不自动分析；只有用户显式选择 ImportedMessage 后才生成 suggestion。Producer 只能输出 ordinal、建议 kind/name/aliases 与已选择 message ID，不能写 proposal/EvidenceRef/entity/scope/status/revision/time。
- Rust/Tauri 实现遵循 typed `Result`、无生产 `panic`、无 Key/正文日志；本轮选择离线确定性规则，不引入隐藏模型选择、网络请求或计费。

## 本次关闭的 B 编号

### B2/B5-PROVIDER：关闭 suggestion producer 和核心 request 命令缺失

- 新增 `ImportKnowledgeSuggestionProducer` 边界及 `DeterministicImportKnowledgeSuggestionProducer`。规则按已选择原文逐条生成受限 draft，提供稳定 `DeterministicRule` generator，并拒绝空正文；建议名称按 Unicode 字符限长，Evidence 只能引用输入 message ID。
- `KernelService::request_import_knowledge_proposals` 已串起 v18 request receipt、权威 import bundle、受限 producer、Kernel-authored source snapshot/EvidenceRef/proposal 与原子 batch 持久化。
- 注册 typed Tauri 命令 `request_import_knowledge_proposals`，员工03的正式导入审核入口不再因命令缺失而失败。
- 同一 `requestId` 已完成批次时在 producer 前直接返回原 batch。计数回归证明响应丢失后的精确重试只调用 producer 一次，不发生二次生成；离线规则无 Provider 计费或网络取消状态。

## 跨模块证据

1. 正式链路：`ImportIntakeDialog → kernelClient typed invoke → Tauri command → KernelService → SQLite v18 request receipt → deterministic producer → domain planner → ImportContent EvidenceRef/proposal batch → Confirm/Reject UI`。
2. 新增真实 SQLite 服务回归：保存 ImportSource/Revision/ImportedMessage 后显式请求，生成 `Decision` proposal 与 ImportContent EvidenceRef；相同请求精确重放，batch 完全一致且 producer 调用次数为 1。
3. Producer 单测覆盖来源引用白名单、Decision/Constraint 分类和空正文失败；失败不会伪造 batch、实体或证据。
4. Rust：library `243 passed / 0 failed / 1 ignored`，B5 audit `4/4`；Clippy all targets/features `-D warnings`、rustfmt 通过。
5. 前端：Vite `2099 modules`，Chat `66/66`、Canvas `34/34`、Canvas 性能 `2/2`；秘密扫描 `559` 个文本文件、`git diff --check` 通过。

## 契约、安全与恢复影响

- Proposal、EvidenceRef 与 generationRunId 仍由 Kernel/SQLite 权威生成；Producer 不能接触凭据、写数据库或提升 KnowledgeEntity。
- request receipt 在 producer 前持久化；完成批次精确重放。producer 失败保留可审计未完成 receipt，同一精确请求可安全重试，不存在自动降级到 Mock 或隐式联网。
- 本轮激活的是原生 Tauri/SQLite 后端行为，静态 Review Lab preview 无法运行该命令且没有新的静态 UI 形态，因此不发布会继续显示“命令不可用”的误导 preview；不生成 dirty full 或 Acceptance Build。

## 剩余阻断与协作

1. **重启发现入口**：员工01/02/03仍需冻结并接入“按 import source/revision 发现既有 request/batch”的权威查询；当前 `get_import_knowledge_proposal_batch(requestId)` 需要调用方已知 requestId，不能仅靠 React 临时状态证明崩溃恢复。
2. **评审与发布**：请员工01评审 producer 边界、v18 request/batch/review 事务及 Windows path-safe EvidenceRef；随后冻结 clean commit。共享树仍 dirty，本报告不宣称 B1、B5 或 M2 Go。
3. **统一验收**：clean 源冻结后由员工05发布唯一 Acceptance Release，并从全新数据库执行同一真实样本的导入、建议、确认实体、候选选择、Close 四动作、Reopen/重启及故障矩阵。

## 下一步 WIP（最多两个）

1. 配合员工01/02/03关闭既有 request/batch 的重启发现入口并复跑跨模块回归。
2. 等员工01 clean 冻结后执行唯一 Release 与 B5 真实纵向验收；冻结前不发布 dirty Release。

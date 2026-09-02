# 员工05｜M2 本地 Embedding 与 VectorIndex 首切片

> 提交时间：2026-08-25 14:59（UTC+8）  
> 覆盖任务：PROV-M2-004～005 首切片  
> 状态：本地适配器边界完成，等待与 SQLite/检索查询接线

## 已阅读与落实

- 已读取最新 PEC 控制文件及员工01～04、06当前报告。
- 员工01/02当前主线已转向 FocusFrame 查询投影和 SQLite 生命周期；因此本次只交付 Provider 负责的本地 Embedding/VectorIndex 边界，不修改其 schema、命令或查询 DTO。
- 遵守 M2 禁止项：不调用云端 Embedding、不把向量相似度当成事实、不覆盖原文、不让索引反向定义知识状态。

## 本次完成与证据

- 新增 `desktop/src-tauri/src/adapters/provider/embedding.rs`：
  - `EmbeddingAdapter` trait，声明模型版本、维度和文本向量化入口。
  - `LocalHashEmbedding` 确定性、零网络、零 Key 的本地实现。
  - `EmbeddingMetadata` 固定记录 `modelVersion`、`dimensions`、`sourceHash`、`chunkVersion`。
  - `LocalVectorIndex` 支持 upsert、cosine-like dot search、remove、rebuild；upsert 会替换旧 metadata，避免旧模型/旧分块向量混用。
  - `IndexInput` 作为后续 SQLite/FTS 查询层的纯输入边界。
- `desktop/src-tauri/src/adapters/provider/mod.rs` 已导出上述适配器类型，未接通云端或 UI。

## 契约、数据和安全影响

- 该切片只在内存中保存向量，原文仍由调用方管理；不写 API Key、完整 reasoning 或 raw 导入内容。
- 向量索引结果仅提供候选匹配和 provenance，不自动升级 KnowledgeEntity 状态，也不绕过 FocusFrame/确认状态规则。
- 模型版本、维度、源哈希、分块版本随记录保存，为未来重建和失效检测提供依据。

## 验证结果

- `cargo test --manifest-path desktop/src-tauri/Cargo.toml --all-targets --locked`：95/95 通过。
- `cargo clippy --manifest-path desktop/src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings`：通过。
- Embedding 定向测试：3/3 通过。
- `cargo fmt --all`、`git diff --check`：通过。

## 风险与阻塞

- 当前索引是内存适配器，尚未持久化到 SQLite，也未与 FTS/关系召回合并；不能宣称 RAG-M2 或 M2 Go 完成。
- hash embedding 仅用于本地工程验证和离线候选召回，不代表语义模型质量或生产推荐。

## 下一步 WIP（最多两项）

1. 与员工02评审 VectorIndex 持久化字段和重建触发边界，接入其 SQLite 检索/事务层前置校验。
2. 推进 PROV-M2-006：向量不可用时回退全文/关系候选，并保持 provenance 与错误提示可见。

## 需要同事配合

- **员工02**：评审 `EmbeddingMetadata` 到 SQLite/索引表的字段映射；不得让向量索引改变导入事务或知识状态。
- **员工01**：确认 VectorMatch 只能作为 `KnowledgeRetrievalCandidate` 输入，仍需经过 FocusFrame、scope、status 和证据规则。
- **员工03/04**：暂不直接消费内存索引；等待查询 IPC 和结构化检索 DTO 冻结。

# MindScape

MindScape 是一个模型无关的 AI 工作空间。它帮助用户表达意图、组织上下文、按需挂载能力、控制交付形式，并把对话沉淀为可继续推进的成果。

## 正式工程

正式桌面工程位于 [`desktop`](desktop/README.md)，采用 Tauri v2、Rust、React、TypeScript、Vite 与 SQLite。`apps` 下的内容属于旧原型，不作为当前工程实现基线。

## 工程协作

- [参与贡献与本地最低检查](CONTRIBUTING.md)
- [Git 提交、评审与合并规范](docs/engineering/git-workflow.md)
- `main` 是唯一长期主干；功能通过短生命周期分支和 Pull Request 合并。
- `desktop/` 是唯一正式工程，`apps/desktop/` 与 `UI Design/` 只能作为参考，不能成为正式依赖或领域权威来源。

## 产品记录

- [当前产品共识](docs/product-foundation.md)
- [关键决策记录](docs/decisions.md)
- [讨论与更新日志](docs/discussion-log.md)
- [认知能力编排器](docs/cognitive-orchestrator.md)
- [外部 AI 会话导入](docs/conversation-import.md)
- [自适应分析引擎](docs/adaptive-analysis.md)
- [核心算法与实现路线](docs/core-algorithm-roadmap.md)
- [透明推理与工具边界](docs/reasoning-transparency-and-tool-scope.md)
- [MVP 开发基线](docs/mvp-build-baseline.md)
- [第一版产品规划与范围边界](docs/v1-product-scope.md)
- [产品内核与工程总览](docs/core-kernel-overview.md)
- [第一版任务拆分与团队分配](docs/v1-work-breakdown.md)
- [V1 前五名员工任务分配](docs/v1-first-five-assignments.md)
- [V1 跨团队契约基线 RC1](docs/contracts/v1-contract-baseline.md)
- [V1 安全威胁模型](docs/architecture/v1-threat-model.md)
- [V1 架构走查清单](docs/architecture/v1-architecture-walkthrough.md)

## 记录约定

后续每次产品讨论或实现更新，都同步维护以上文档：

1. 新共识写入 `product-foundation.md`，旧表述被替代时保留原因到决策记录。
2. 会影响产品定位、交互、架构或范围的选择写入 `decisions.md`。
3. 每轮讨论的新增观点、分歧和待办写入 `discussion-log.md`。
4. 未确定的内容明确标记为“待决策”，不伪装成已经达成的结论。

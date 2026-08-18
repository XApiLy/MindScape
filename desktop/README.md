# MindScape Desktop

MindScape 的正式桌面工程。技术基线为 Tauri v2、Rust、React、TypeScript、Vite 和 SQLite。

## 目录边界

```text
src/
  app/          Tauri 命令客户端与前端用例入口
  domain/       按 conversation/context/runtime/import 等拆分的 TypeScript 契约镜像
src-tauri/src/
  domain/       会话图、ContentBlock、上下文快照、跨团队契约与领域不变量
  application/  用例编排，不依赖 UI
  adapters/     SQLite 等外部基础设施
  commands.rs   稳定的 Tauri 命令边界
```

依赖方向从外向内：UI 和适配器依赖应用与领域契约，领域层不依赖 React、Tauri 或 SQLite。

## 当前底座

- SQLite 版本化迁移（当前 schema v5）、ModelRun事件与CanvasViewport持久化、本地默认工作区。
- 会话、完整问答节点、显式语义边与画布位置。
- 每轮调用前生成并保存冻结的 `ContextSnapshot`。
- 深入、发散、换角度的确定性上下文继承规则。
- Frozen V1上下文/模型运行契约，以及RC1导入双轨、证据引用和事件信封。
- 追加式领域事件账本。
- 创建会话、加载图、追加/完成问答、读取上下文和保存位置的 Tauri 命令。

跨团队契约说明见 [`docs/contracts/v1-contract-baseline.md`](../docs/contracts/v1-contract-baseline.md)。Context/Runtime已冻结；Import/Evidence等待M2专项消费方评审。

桌面端数据库、原始导入内容与迁移备份的物理边界见
[`docs/desktop-local-data-layout.md`](../docs/desktop-local-data-layout.md)。

## 本地验证

```bash
pnpm install
pnpm build
cd src-tauri
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

运行桌面开发窗口：

```bash
pnpm tauri dev
```

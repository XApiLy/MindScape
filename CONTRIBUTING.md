# Contributing to MindScape

MindScape 使用短生命周期分支、强制 Pull Request、线性主干和默认 Squash Merge。完整规则见 [Git 提交、评审与合并规范](docs/engineering/git-workflow.md)。

## 开始前

1. 确认任务已有稳定编号，例如 `CORE-014`、`CAN-014` 或 `PROV-008`。
2. 从最新 `main` 创建分支：`feat/CORE-014-run-coordinator`。
3. 一条分支只解决一个可独立验收的问题；个人同时进行中的任务不得超过两个。
4. `desktop/` 是唯一正式工程；`apps/desktop/` 是旧原型，`UI Design/` 仅作界面参考。

## 本地最低检查

在 `desktop/` 下执行：

```powershell
pnpm test:canvas
pnpm test:chat
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

只运行受影响检查并不足以合并；Pull Request 合并前必须通过仓库规定的完整必需检查。

## 提交与 Pull Request

- 提交格式：`<type>(<scope>): <summary>`。
- 示例：`feat(provider): stream OpenAI-compatible responses`。
- 提交正文或尾注必须关联任务：`Refs: PROV-008`。
- PR 标题同样使用上述格式，并完整填写风险、测试、数据迁移、回滚和安全说明。
- 禁止直接推送 `main`，禁止把密钥、数据库、本地会话、日志和构建产物加入仓库。

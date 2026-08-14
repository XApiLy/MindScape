## 目标

<!-- 一句话说明用户或工程问题，以及本 PR 的唯一目标。 -->

- Task：<!-- 例如 CORE-014 -->
- 类型：<!-- 标准 / 高风险 / 紧急修复 -->
- 关联 PEC / ADR / Issue：

## 改动范围

<!-- 列出主要改动和明确不在本 PR 内的内容。 -->

## 契约与数据影响

- [ ] 不修改 Rust 权威领域契约
- [ ] 已同步 TypeScript 镜像
- [ ] 不修改 SQLite schema / migration
- [ ] migration 可从上一发布版本升级，并有失败恢复测试
- [ ] 不改变导入原文、事件账本或 ContextSnapshot 的不可变语义

不适用项说明：

## 安全与隐私

- [ ] 不包含 API Key、Token、个人会话、数据库、日志或真实未脱敏样本
- [ ] 新增日志已经脱敏
- [ ] Provider / 导入 / 文件路径输入已按不可信输入处理
- [ ] 凭据仍不能被前端读回

不适用项说明：

## 验证证据

- [ ] `pnpm test:canvas`
- [ ] `pnpm test:chat`
- [ ] `pnpm build`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test`
- [ ] 已完成适用的手工 / E2E / 性能 / 恢复验证

证据、截图或测试输出摘要：

## 风险与回滚

- 失败模式：
- 观测方式：
- 回滚步骤：
- 数据是否可逆：

## 评审关注点

<!-- 主动告诉评审者最需要检查的两到三处内容。 -->

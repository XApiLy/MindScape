# B1 文件归属与 clean 集成审计

> 审计时间：2026-09-01（UTC+8）  
> 审计基线：`f4759c21460cd206e64ee56f8b8ac689216fd1bc`  
> 审计负责人：员工01（技术负责人）  
> 结论：291 个正式变更路径已完成归属确认；加上本审计、员工01当前 PEC 与旧 PEC 归档后，最终 294 个路径组成单一 B1 源码冻结提交。本地生成物、外部参考仓和不可独立构建的旧原型不进入提交。

## 1. 审计范围与处理结果

- 初始共享树有 1716 条 tracked/untracked 状态；逐项按产品源码、测试、正式文档、证据、PEC、工具源码和本地材料分类。
- 最终纳入 291 个工程/资料路径；加上本轮三项审计记录与 PEC 原子替换，提交共 294 个路径。暂存区外无未跟踪或未暂存的非忽略文件。
- `.playwright-cli/`、`output/`、`example/` 为浏览器运行数据、截图/构建输出和外部视觉参考仓，只写入 `.gitignore`，本地文件未删除。
- `tools/visual-prototype/` 直接编译期引用 `example/liquid-glass-main` 与 `example/glace-main`。在外部参考仓被正确隔离后，该工具无法从干净检出独立构建，且历史 PEC 已声明它不再作为材质通过证据，因此同样保留本地但不进入冻结提交。
- 字体二进制、视觉测试媒体与验收截图均小于 100 MiB；最大文件为 `Xiaolai-Regular.ttf`（22,220,806 bytes）。字体来源、OFL、固定版本、字节数和 SHA-256 由 `desktop/src/assets/fonts/README.md` 记录。
- `.gitattributes` 补齐 TTF/OTF 二进制属性，并仅对固定第三方 OFL 与 vendored 源码声明 whitespace 例外；自有代码仍由 `git diff --check` 严格校验。

## 2. 文件归属矩阵

| 路径/模块 | 主责 | 集成决定 | 归属依据 |
| --- | --- | --- | --- |
| `desktop/src-tauri/src/domain/`、`application/` 及共享契约 | 员工01，员工02/05协作 | 纳入 | Focus 决策、导入/讨论/运行契约与 Kernel 服务边界已完成 B2/B3/B4 评审 |
| `desktop/src-tauri/src/adapters/data_paths.rs`、`sqlite.rs`、`generic_import.rs`、`import_storage.rs`、`markdown_vault.rs` | 员工02 | 纳入 | SQLite 迁移、导入事务、Vault journal、恢复与删除闭环 |
| `desktop/src-tauri/src/adapters/provider/`、`semantic_embedding.rs` | 员工05 | 纳入 | Provider 流式终态、Embedding、混合检索、重建/回退与安全边界 |
| `desktop/src-tauri/src/commands.rs`、`lib.rs`、锁文件及 `examples/b5_focus_promotion_audit.rs` | 员工01集成，员工02/05实现 | 纳入 | typed IPC、启动恢复、统一依赖冻结和 B5 审计入口必须同一提交编译 |
| `desktop/src/canvas/`、`ConversationCanvas.tsx` 及画布投影样式 | 员工03 | 纳入 | FocusFrame、分支、知识来源、检索结果和稳定布局正式投影 |
| `desktop/src/app/`、Chat/设置/导入/安全 Markdown 组件、字体和阅读样式 | 员工04，员工05协作运行态 | 纳入 | 正式 UI 消费、错误状态、运行档案、安全渲染、七款内置字体及重启偏好 |
| `desktop/src/App.tsx`、`kernelClient.ts`、领域索引、package/lock | 员工01集成 | 纳入 | 跨 Rust/Tauri/React 的入口接线与可重复依赖快照 |
| `docs/architecture/`、`docs/design/`、`docs/engineering/` 及正式证据 | 对应员工，员工01审计 | 纳入 | M2 架构、视觉边界、发布制度和可追溯验收证据 |
| `tools/selected-visual-baseline/`、`tools/source-reference-lab/` | 员工06/创始人直管，员工01审计 | 纳入 | 独立于生产工程、带第三方声明，均可从当前提交完成 production build |
| `Project Engineering Communication/` 与归档目录 | 员工01～06 | 纳入 | 当前窗口严格保留六份个人最新 PEC 与一份控制文件，历史报告只归档不删除 |
| `.playwright-cli/`、`output/`、`example/`、`tools/visual-prototype/` | 本地运行/外部参考 | 排除 | 非产品源码、体积大、可再生或无法在干净检出独立构建 |

## 3. 跨模块门禁证据

- 前端：Chat `60/60`、Canvas `34/34`、Canvas 性能 `2/2`；`pnpm build` 通过，Vite 转换 2097 modules。
- Rust/Tauri：`cargo test --all-targets --all-features --locked --quiet` 为 library `205 passed / 0 failed / 1 ignored`，B5 audit example `2/2`；ignored 为需要显式真实模型包的固定质量测试，不作为 B1 证据。
- Rust 质量：`cargo clippy --all-targets --all-features --locked -- -D warnings` 与 `cargo fmt --all -- --check` 通过。
- 工具：`tools/selected-visual-baseline` 与 `tools/source-reference-lab` 的 `pnpm build` 均通过；旧 `tools/visual-prototype` 构建失败原因已转化为明确排除规则，而非带入不可复现依赖。
- 仓库：`git diff --cached --check` 通过；`node scripts/check-secrets.mjs` 扫描 539 个文本文件通过；无 staged 文件超过 100 MiB。
- PEC：`node scripts/check-pec-retention.mjs` 必须在员工01报告替换后再次通过。

## 4. 冻结结论与剩余阻断

- 员工01可以从本审计集合交付唯一 clean 源码提交；提交后工作树必须为空，并验证本地分支与远端同步。
- 本文件只关闭员工01负责的 B1 集成边界、文件归属和源码冻结，不等同于 Acceptance Release 或 M2 Go。
- 员工05仍须只从该 clean commit 运行 `scripts/publish-acceptance.ps1`，记录唯一 Build ID、程序路径、SHA-256、无开发服务器启动及 B5 同一真实样本证据。
- B6 受控用户字体导入仍未开始，继续不得抢占 B1/B5；是否移入 M3 由创始人裁决。

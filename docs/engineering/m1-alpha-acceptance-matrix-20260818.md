# MindScape M1 真实 Chat Alpha 统一验收表

- Owner：员工01（技术负责人）
- 初始日期：2026-08-18；最近复核：2026-08-20 16:59（UTC+8）
- 当前结论：**Go**。M1 统一验收脚本 1—10 已闭环；M2 仍须等待正式派单，不自动开工。
- 验收口径：[v1-execution-dispatch-20260817.md](./v1-execution-dispatch-20260817.md#11-m1-真实-chat-alpha-统一验收脚本) 与 [m1-pec-review-and-closeout-20260818.md](./m1-pec-review-and-closeout-20260818.md)

## 当前 clean 验收基线

- Build ID：`20260820-155308-m1-clean-gate-98e12706c2f4`
- 来源提交：`98e12706c2f4`；`sourceTreeDirty=false`
- 程序：`D:\Project\MindScape\artifacts\acceptance\versions\20260820-155308-m1-clean-gate-98e12706c2f4\mindscape-desktop.exe`
- SHA-256：`6FE02B0BD551288805B8034AE39DB19528815F901F24D1C69471865985ABA59A`
- Tauri release 构建与无 `localhost:1420` 独立启动通过，实际显示 MindScape 工作区。

## 脚本 1—10

| # | 真实验收动作 | 当前状态 | 已有证据与缺口 | 放行影响 |
|---|---|---|---|---|
| 1 | 新用户创建本地对话并配置 DeepSeek 凭据 | 通过 | 原应用数据安全隔离后，以空应用数据启动同一 clean Build；系统安全凭据库中的既有 DeepSeek 配置保持可用，创建首个会话并完成首次真实回答，随后原数据完整恢复。证据：`evidence/employee01/m1-first-run-20260820/README.md`。 | 普通 |
| 2 | 根节点真实流式回答，显示真实 provider/model | 通过 | 长文、provider/model 与 Usage 已有真实证据；同一 clean Build 再次创建根节点并完成真实 `deepseek/deepseek-v4-flash` 运行，状态 `completed`、`last_sequence=4`。证据：`evidence/employee02/prov-014-long-output-auth-20260820/README.md`、`evidence/employee01/m1-first-run-20260820/README.md`。 | **失败即 No-Go** |
| 3 | 建立至少两类分支并核对实际 ContextSnapshot | 通过 | 创始人在统一 clean Build 中真实执行“深入”和“发散”；Node、Edge、ContextSnapshot 分支类型分别一致，真实库聚合为 `continues=23`、`deepens=1`、`diverges=1`。证据：`evidence/employee01/m1-real-branches-20260820/README.md`。 | 普通 |
| 4 | 第二次回答中途停止，保留部分文本且不得为 completed | 通过 | 真实 DeepSeek 已进入唯一 `cancelled` 终态，部分文本保留，终态后 delta 为 0，数据完整性通过；重复投影修复已通过自动测试。创始人在统一 Tauri release Build ID `20260819-144548-chat-012-single-card-recheck-0cb1c0053b5a-dirty` 中确认阅读视图仅一张已停止卡片，部分内容与重试入口保留；进程路径与发布目录一致。证据：`evidence/employee04/prov-013-real-cancel-20260819/README.md`。 | **失败即 No-Go** |
| 5 | 流式期间强制关闭，重启后无永久 pending 且历史完整 | 通过（运行与数据） | Windows 结束任务后重启，运行确定性恢复为 `failed + application_interrupted`，序列 775、部分正文 1392 字符保留、无永久 pending、SQLite 完整。证据：`evidence/employee02/m1-recovery-20260820/README.md`。画布联合恢复归入脚本 8。 | **失败即 No-Go** |
| 6 | 使用无效 Key，得到可修复认证错误且不无限重试 | 通过 | 错误 Key 显示安全鉴权提示，不回显凭据、不回退 Mock、不创建 ModelRun。证据：`evidence/employee02/prov-014-long-output-auth-20260820/README.md`。 | **失败即 No-Go** |
| 7 | 断网/超时，进入正确错误态且不生成伪助手消息 | 通过 | 已有正文后持续断网进入 `failed/stream_idle_timeout`，部分正文保留、唯一终态、无永久 pending、无自动计费重试。证据见员工05当前 PEC。 | 普通 |
| 8 | 重启恢复会话、节点、边、模型、位置和 viewport | 通过 | 同一 clean Build 中拖动取消节点后正常关闭并重新打开；只读核对对应会话 5 个 Node、3 条 Edge、3 个位置、1 个 viewport，重复 Edge 0，所有 Node 均有 provider/model，`integrity_check=ok`，pending/streaming=0。证据：`evidence/employee01/m1-restart-canvas-20260820/README.md`。 | **失败即 No-Go** |
| 9 | 数据库、日志、崩溃材料均不含完整 API Key | 通过 | clean Build 验收后复扫恢复数据、隔离数据及 Local 日志/Crashpad：341 个文件中 307 个可读文件未命中常见 API Key、Token 或私钥模式；34 个运行中不可读文件如实排除，两份 SQLite 均完成只读完整性检查；仓库秘密扫描通过。证据：`evidence/employee01/m1-first-run-20260820/README.md`。 | **失败即 No-Go** |
| 10 | 前端、Rust、PEC、仓库治理全部通过 | clean 基线通过 | 提交 `98e1270` 已推送 PR 分支；同一 clean 源码上 Chat 18/18、Canvas 13/13、性能 1/1、Rust 49/49、Review Lab 9/9、前端/Tauri release、Rust fmt/clippy、PEC 与秘密扫描均通过。 | 普通 |

## 员工01底层不变量复核

| 场景 | 必须满足的不变量 | 结论 |
|---|---|---|
| 正常完成 | Node、ModelRun、ContextSnapshot 关联一致；完成态不可被迟到事件覆盖 | 真实长文证据与同一 clean Build 首答 SQLite 复演通过 |
| 用户取消 | 部分文本可保留；run 为 cancelled；节点不得误记 completed | 员工01签署通过 |
| Provider 失败 | 错误可诊断；不得生成伪完成消息；不得无限重试 | 无效 Key、持续断网和空正文失败门禁通过 |
| 应用中断与恢复 | pending run 被确定性恢复；历史、图结构及 viewport 不丢失 | 运行恢复、图结构、位置和 viewport 联合证据通过 |
| 幂等与并发 | 同一幂等键只创建一个有效运行；重复完成/取消无副作用 | 自动化契约门禁与真实取消/恢复的唯一终态证据通过；统一脚本未要求单独手工并发场景 |

## 集成冻结清单

- [x] 创建 `codex/m1-integration-recovery` 集成恢复分支。
- [x] 忽略 `desktop/MindScape-DeepSeek-Fixed.exe` 与 `desktop/src-tauri/target-fixed/`。
- [x] 按合同/上下文、数据/恢复、Provider、Chat、Canvas、Review Lab、文档/PEC 七组完成审阅。
- [x] 对工作树执行密钥扫描和禁止文件检查；暂存区扫描在提交前执行。
- [x] 在同一集成树复跑前端、Rust、Review Lab、Tauri 门禁。
- [x] 形成可回滚、披露员工01—06共同贡献的集成提交并推送恢复分支。
- [x] 已创建 PR #1，最新 M1 修复与证据已推送；统一真实验收闭环后由员工01签署 Go。
- [x] 从 clean commit 发布统一 Tauri release，并核对 manifest、SHA-256 与独立启动。

## 证据规则

验收必须记录操作、期望结果、实际结果、数据库/日志核对及证据位置。截图、文件名、终端输出和命令历史不得包含完整 API Key；失败证据与通过证据同样保留，禁止用自动化测试替代脚本 1—9 的真实运行验收。

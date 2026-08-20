# MindScape M1 真实 Chat Alpha 统一验收表

- Owner：员工01（技术负责人）
- 初始日期：2026-08-18；最近复核：2026-08-20 16:03（UTC+8）
- 当前结论：**No-Go**。在本表的真实运行证据全部闭环前，不进入 M2。
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
| 1 | 新用户创建本地对话并配置 DeepSeek 凭据 | 部分通过 | 已有真实 `GET /models` 成功证据；仍需从空状态完整演示首次配置与创建对话。 | 普通 |
| 2 | 根节点真实流式回答，显示真实 provider/model | 通过（待 clean 基线复演） | 真实 DeepSeek 根节点、长文正文、provider/model 与 Usage 已通过；最终 clean Build 须回放一次。证据：`evidence/employee02/prov-014-long-output-auth-20260820/README.md`。 | **失败即 No-Go** |
| 3 | 建立至少两类分支并核对实际 ContextSnapshot | 未通过 | 2026-08-20 只读核对真实库：21 个 Node 与 21 个 ContextSnapshot 的分支类型均为 `continues`，尚无第二种真实分支类型及冻结上下文证据。 | 普通 |
| 4 | 第二次回答中途停止，保留部分文本且不得为 completed | 通过 | 真实 DeepSeek 已进入唯一 `cancelled` 终态，部分文本保留，终态后 delta 为 0，数据完整性通过；重复投影修复已通过自动测试。创始人在统一 Tauri release Build ID `20260819-144548-chat-012-single-card-recheck-0cb1c0053b5a-dirty` 中确认阅读视图仅一张已停止卡片，部分内容与重试入口保留；进程路径与发布目录一致。证据：`evidence/employee04/prov-013-real-cancel-20260819/README.md`。 | **失败即 No-Go** |
| 5 | 流式期间强制关闭，重启后无永久 pending 且历史完整 | 通过（运行与数据） | Windows 结束任务后重启，运行确定性恢复为 `failed + application_interrupted`，序列 775、部分正文 1392 字符保留、无永久 pending、SQLite 完整。证据：`evidence/employee02/m1-recovery-20260820/README.md`。画布联合恢复归入脚本 8。 | **失败即 No-Go** |
| 6 | 使用无效 Key，得到可修复认证错误且不无限重试 | 通过 | 错误 Key 显示安全鉴权提示，不回显凭据、不回退 Mock、不创建 ModelRun。证据：`evidence/employee02/prov-014-long-output-auth-20260820/README.md`。 | **失败即 No-Go** |
| 7 | 断网/超时，进入正确错误态且不生成伪助手消息 | 通过 | 已有正文后持续断网进入 `failed/stream_idle_timeout`，部分正文保留、唯一终态、无永久 pending、无自动计费重试。证据见员工05当前 PEC。 | 普通 |
| 8 | 重启恢复会话、节点、边、模型、位置和 viewport | 部分通过 | 正常重启已证明 A/B 会话、节点、边、位置、viewport；强退已证明运行终态与部分正文。仍缺取消、失败、强退后的唯一节点、边、位置、viewport 在同一 Build 的联合窗口证据。 | **失败即 No-Go** |
| 9 | 数据库、日志、崩溃材料均不含完整 API Key | 技术通过（待 clean Build 复签） | 仓库秘密扫描通过；Roaming 数据库/备份与 Local 日志/Crashpad 共 59/59 文件可读扫描，未发现常见 API Key、Token 或私钥模式。最终 clean Build 仍须按同一范围复跑并保存脱敏结果。 | **失败即 No-Go** |
| 10 | 前端、Rust、PEC、仓库治理全部通过 | clean 基线通过 | 提交 `98e1270` 已推送 PR 分支；同一 clean 源码上 Chat 18/18、Canvas 13/13、性能 1/1、Rust 49/49、Review Lab 9/9、前端/Tauri release、Rust fmt/clippy、PEC 与秘密扫描均通过。 | 普通 |

## 员工01底层不变量复核

| 场景 | 必须满足的不变量 | 结论 |
|---|---|---|
| 正常完成 | Node、ModelRun、ContextSnapshot 关联一致；完成态不可被迟到事件覆盖 | 真实长文与 SQLite 证据通过，最终 clean Build 待复演 |
| 用户取消 | 部分文本可保留；run 为 cancelled；节点不得误记 completed | 员工01签署通过 |
| Provider 失败 | 错误可诊断；不得生成伪完成消息；不得无限重试 | 无效 Key、持续断网和空正文失败门禁通过 |
| 应用中断与恢复 | pending run 被确定性恢复；历史、图结构及 viewport 不丢失 | 运行恢复通过；图结构与 viewport 联合证据待补 |
| 幂等与并发 | 同一幂等键只创建一个有效运行；重复完成/取消无副作用 | 待真实复核 |

## 集成冻结清单

- [x] 创建 `codex/m1-integration-recovery` 集成恢复分支。
- [x] 忽略 `desktop/MindScape-DeepSeek-Fixed.exe` 与 `desktop/src-tauri/target-fixed/`。
- [x] 按合同/上下文、数据/恢复、Provider、Chat、Canvas、Review Lab、文档/PEC 七组完成审阅。
- [x] 对工作树执行密钥扫描和禁止文件检查；暂存区扫描在提交前执行。
- [x] 在同一集成树复跑前端、Rust、Review Lab、Tauri 门禁。
- [x] 形成可回滚、披露员工01—06共同贡献的集成提交并推送恢复分支。
- [x] 已创建 PR #1，最新 M1 修复与证据已推送；真实验收未闭环时保持 No-Go。
- [x] 从 clean commit 发布统一 Tauri release，并核对 manifest、SHA-256 与独立启动。

## 证据规则

验收必须记录操作、期望结果、实际结果、数据库/日志核对及证据位置。截图、文件名、终端输出和命令历史不得包含完整 API Key；失败证据与通过证据同样保留，禁止用自动化测试替代脚本 1—9 的真实运行验收。

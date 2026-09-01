# 决策台账与原始来源

> 用途：区分已经确认的产品方向、只在原型中成立的结论、明确后置项和仍待确认项。

## 1. 已确认

- MindScape定位为把Chat做到极致的认知工作空间，不进入AI IDE或本地编程Agent赛道。
- 保留现有会话导航、Chat/画布双视图、底部编辑器、节点、分支和聚焦阅读骨架。
- 首次引导通过真实任务发生，不使用连续弹窗式产品旅游。
- 默认体验简单，高级模型参数、分析、上下文和工具轨迹渐进展开。
- 视觉结果由背景、组件材质、氛围主题、区域覆盖、运行退让和设备策略共同解析。
- 内容优先、中心安静、专注退让、低频运动、状态诚实、永久降级是硬原则。
- 颜色/壁纸背景与Opaque/毛玻璃/Liquid Glass必须自由组合；Classic Opaque永久保留。
- DOM负责可访问内容和业务交互；Pixi/WebGPU负责背景/氛围；Shader不得读取业务事实。
- Liquid Glass、毛玻璃、天气和云的原型来源分别冻结为Ybouane、Glacé、React Weather Effects和Drei Cloud。
- 流式打字只属于展示层；用户离开底部后锁定阅读位置；“定位到最新”和画布“定位当前节点”只由用户主动触发。
- M2 首批阅读字体预设确认为 Caveat、Nunito、Lato、Cormorant、寒蝉半圆体（ChillRoundM）、小赖字体（Xiaolai）和得意黑（Smiley Sans）；官方固定版本资源、OFL、元数据和 SHA-256 已归档，但尚未完成工程接线与正式安装包验收。
- 创始人已于 2026-08-30 确认七款字体切换后的真实阅读与画布问答效果满意；功能视觉预验收通过，最终 clean Release 回归仍待完成。
- 本地呈现模型是独立的阅读增强角色：取得用户明确同意后，为已完成正文提出术语、实体和金句标注；标注不改 raw Markdown、不进入 Chat ModelRun、不自动写入 RAG 或知识事实。产品方向已确认，实施里程碑待门禁。
- 员工06由创始人直管；员工03/04可先做带 `UI-HANDOFF-06` 的真实功能占位。

## 2. 已经原型验证，但不代表生产完成

- 四个选定视觉来源可在同一MindScape布局中组合。
- 输入、选字、阅读、设置和低性能状态可以触发天气/云退让或卸载。
- Ybouane每帧全区域截图曾把默认组合降到约7 FPS；适配层节流、轮转采样和天气资源复用后，当前独立原型稳定态已达到可评审水平。
- 雨滴可以用同一模拟和材质切换环境下层、组件表面和全局透明覆盖，不复制模拟且不拦截输入。
- 这些证据来自浏览器原型，不是正式Tauri生产Build的完成证明。

## 3. 尚未获准或明确后置

- `VIS-010～020` 的整体生产实现和对 `desktop/` 的视觉系统重构，仍需创始人明确开工与工程门禁。
- Wallpaper Engine Scene/Application专有运行时和“全部壁纸兼容”承诺。
- 任意用户WGSL/GLSL源码、第三方Shader下载和执行。
- 环境声音、真实天气API和定位权限。
- 没有明确产品价值的3D场景或让Three.js进入首包。
- 把每个L3按钮都做成独立材质编辑器。
- 复杂粒子、长轨迹连线和只为装饰的“思考中”动画。

## 4. 仍待确认或生产前必须冻结

- 最终品牌色、字体、间距、圆角、阴影、图标和Design Token。
- “轻柔 / 沉浸”预设的最终强度、恢复时间和磁吸/打字动效曲线。
- 氛围是否根据壁纸自动推荐、推荐后的覆盖方式和首批正式主题。
- 区域覆盖首版只到L2，还是允许少量L3控件独立覆盖。
- Ybouane可归档许可证文本；Drei云纹理本地化；Snow/Fog完整生命周期验证。
- 正式生产性能预算、最低设备、DPI/窄窗矩阵和统一Tauri Build验收。
- 流式阅读/定位规格的实际代码接线与真实长回答验收。

## 5. 原始来源索引

| 主题 | 详细原文 |
| --- | --- |
| M1 Chat九态、保留/修改/后置 | [M1视觉与交互收口](../m1-chat-alpha-visual-closeout-20260818.md) |
| 当前生产UI保留与迁移边界 | [当前正式界面审计](../current-ui-audit-20260819.md) |
| 业务、专注、设备和Shader场景矩阵 | [视觉状态与场景矩阵](../appearance-state-and-scene-matrix-20260819.md) |
| 分层视觉、材质、氛围、Wallpaper与技术方向 | [分层视觉与氛围系统](../layered-visual-and-atmosphere-system-20260819.md) |
| 四个选定原仓、参数、证据、许可证 | [VIS-008选定视觉源](../selected-visual-source-baseline-20260820.md) |
| 雨滴双引擎与层级契约 | [雨滴效果保存](../rain-effect-preservation-20260825.md) |
| 流式打字、滚动锁定和磁吸定位 | [流式呈现与会话定位](../streaming-reveal-scroll-and-conversation-locator.md) |
| Markdown 阅读与字体控制 | [M2 Markdown 阅读与字体偏好](05-markdown-reading-and-typography-m2.md) |
| 首批七款字体预设、回退与许可证门禁 | [M2 首批阅读字体预设目录](06-reading-font-preset-catalog-m2.md) |
| 本地模型术语、实体与金句标注 | [本地呈现智能与阅读标注层](07-local-presentation-intelligence-layer.md) |
| 员工06职责与沟通机制 | [视觉与交互设计工程师职责](../visual-interaction-design-engineer.md) |
| 工程占位与后续视觉交接 | [UI占位与员工06交接](../ui-placeholder-and-employee06-handoff.md) |
| 视觉运行时适配与性能风险 | [VIS-009工程交接](../../engineering/selected-visual-runtime-handoff-20260820.md) |
| 完整阶段计划与生产门禁 | [分层视觉执行计划](../../engineering/layered-visual-system-execution-plan-20260819.md) |
| 按时间追溯全部讨论 | [讨论日志](../../discussion-log.md) |

若摘要与原文冲突，先检查日期和状态：产品范围/正式决策高于视觉原型，已确认的新结论高于旧默认值，正式Tauri证据高于浏览器原型，生产代码事实高于目标架构描述。

# MindScape 视觉与交互设计中心

> 状态：当前统一入口  
> 汇总日期：2026-08-30  
> 范围：UI、视觉语言、动效、首次体验、Chat、无限画布、导入与模型控制的用户体验  
> 说明：本目录是讨论结论的摘要层；历史原文继续保留，不做破坏性移动

## 当前一句话方向

MindScape 要成为一个内容优先、状态诚实、可简单也可深入的 Chat AI 认知工作空间：背景创造环境，卡片建立工作区，氛围让空间活起来；一旦用户开始输入、阅读或思考，所有装饰必须退到内容之后。

## 阅读顺序

1. [产品体验与视觉总纲](01-product-experience-and-visual-language.md)：产品给人的整体感觉、布局、分层视觉和明确避免项。
2. [核心交互与动效规范](02-core-interaction-and-motion.md)：首次使用、Chat、模型参数、导入、画布、流式输出和磁吸定位。
3. [工程边界、性能与职责](03-engineering-boundaries-and-ownership.md)：当前/目标技术、DOM/Pixi/Shader边界、降级策略和人员归属。
4. [决策台账与原始来源](04-decision-register-and-sources.md)：哪些已确认、哪些只是原型、哪些被后置，以及详细原文入口。
5. [M2 Markdown 原生阅读与字体偏好](05-markdown-reading-and-typography-m2.md)：AI 输出的安全 Markdown 呈现、流式长文阅读、字体/字号/行高/宽度控制和验收矩阵。
6. [M2 首批阅读字体预设目录](06-reading-font-preset-catalog-m2.md)：七款已确认预设的名称、定位、回退、资源/许可证门禁和验收要求。
7. [本地呈现智能与阅读标注层](07-local-presentation-intelligence-layer.md)：用户授权的本地模型如何为术语、实体和金句生成可撤销展示标注，以及原文、知识和隐私边界。

## 当前状态

- 现有左侧会话区、Chat/画布双视图、底部编辑器、节点、聚焦阅读、设置和错误反馈是业务骨架，视觉升级不能推翻其状态语义。
- 分层视觉、Liquid Glass、毛玻璃、天气、云和雨滴已完成独立原型与来源选型，但不等于已经获准整体写入正式 `desktop/`。
- 正式工程当前仍以 Tauri v2、Rust、React 19、TypeScript、Vite、SQLite 和可访问 DOM 为主；PixiJS/WebGPU、天气和云属于目标视觉运行时与原型能力。
- UI 功能开发可以先做诚实的占位界面；最终视觉与交互由创始人直管的员工06确认和验收。

## 权威与更新规则

产品范围和领域事实始终由 `docs/v1-product-scope.md`、`docs/product-foundation.md`、`docs/decisions.md` 及正式契约决定，视觉稿不能反向改变它们。本目录负责提供最新、可读的体验摘要；原始设计文档和PEC用于追溯证据。

以后出现新的 UI、视觉、动效或交互结论时：

1. 更新本目录对应主题文件。
2. 在 `04-decision-register-and-sources.md` 标记“已确认 / 原型验证 / 待确认 / 后置”。
3. 同步写入 `docs/discussion-log.md`。
4. 若影响业务状态、里程碑、安全或技术依赖，再进入正式决策与工程门禁。

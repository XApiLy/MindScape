# VIS-009｜选定视觉运行时工程交接

> 状态：交接包已就绪，等待员工03/04书面评审  
> 日期：2026-08-20  
> 上游视觉基线：[VIS-008 选定视觉源基线](../design/selected-visual-source-baseline-20260820.md)  
> 生产门禁：未通过；本文不是开工指令

## 1. 固定实现边界

```text
AppearancePreferences
  background + globalMaterial + regionOverrides + weather + cloud + accessibility
             ↓ 纯解析，不读取业务事实
AppearanceRuntimeState
  focus/input/selection/dialog/window/performance/device
             ↓
BackgroundRoot → AtmosphereAdapter → SurfaceAdapter → 可访问 DOM 内容
```

视觉系统不得接收 Prompt、消息正文、Provider、API Key、ModelRun 终态或会话图作为材质/天气/Shader输入。Chat与画布只发布焦点和交互态，不由视觉层反向定义业务状态。

## 2. 员工04：工作区与 Surface 所有权

员工04负责 `VIS-010`、`VIS-014`、`VIS-016` 和 `VIS-018` 中的 DOM/工作区部分：

1. 建立单一 `AppearanceRoot` 和纯状态解析器；全局材质加区域覆盖只能在一处求值。
2. 建立 `Surface` 适配器，后端固定为：`ClassicOpaque`、`GlaceFrosted`、`YbouaneLiquid`。
3. Ybouane 使用隔离 root，满足直接子元素要求；清理实例、ResizeObserver 和 root `user-select` 副作用。
4. Glacé 保留其 SVG filter、边缘折射、blur、bezel、profile 与 sheen；内容仍为可选择、可聚焦的 DOM。
5. 将侧栏、Chat、编辑器、阅读区、设置、菜单和对话框迁移到同一 Surface 契约，不另建页面级玻璃实现。
6. 建立 React Weather 适配器：统一静态纹理形状、显式生命周期清理、暂停/恢复和错误回退；天气不进入首包阻塞路径。
7. 发布输入、选字、阅读、设置和对话框焦点信号给氛围运行时。

员工04不得复制原仓 Shader 后改名为 MindScape 自制材质，也不得让设置项先于真实能力出现。

## 3. 员工03：云与氛围运行时所有权

员工03负责 `VIS-015`、`VIS-017` 和 `VIS-019` 中的 Canvas/氛围部分：

1. 将 Drei `Cloud` / `Clouds` 作为唯一云实现，独立动态导入 Three/Fiber 块。
2. 云关闭、低性能、设备失败或资源失败时直接卸载 Canvas，不能只设为透明继续按帧渲染。
3. 将远程云纹理审核后版本化入包，提供资源丢失回退。
4. 维护统一氛围衰减器，消费员工04发布的焦点信号；不从 Chat/Canvas 业务状态自行推断。
5. 定义主画布低干扰区，雨滴、云团和高亮不得持续穿过输入、正文和节点操作柄。
6. 减少动态模式停止连续位移；低性能模式停用 React Weather 与 Drei，并允许静态背景继续工作。
7. 收集帧时间、显存/纹理预算、Canvas 数量与设备丢失证据；Three/Fiber 不得进入初始工作区首包。

## 4. 共享适配器契约

建议先冻结以下表现层语义，具体文件位置由员工03/04评审后确定：

```ts
type MaterialPreset = "opaque" | "glace" | "ybouane";
type AppearanceRegion = "sidebar" | "nodes" | "reader" | "composer" | "overlay";
type WeatherPreset = "off" | "drizzle" | "rain" | "storm" | "snow" | "fog";

interface FocusSignals {
  input: boolean;
  selection: boolean;
  reading: boolean;
  dialog: boolean;
  windowInactive: boolean;
}

interface RuntimePolicy {
  reducedMotion: boolean;
  lowPerformance: boolean;
  atmosphereGain: number;
}
```

解析规则：区域显式值优先于全局；低性能时 `ybouane → opaque`；Glacé 可在确认支持 SVG filter 的设备上继续，否则 `glace → opaque`；氛围 gain 只影响环境层，不改变 L2 内容对比基线。

## 5. 合并顺序

1. 员工04合并纯 `AppearancePreferences` / `AppearanceRuntimeState` 与解析测试。
2. 员工04合并三后端 `Surface` 适配器和 Classic 回退。
3. 员工03合并懒加载 Drei Cloud 与氛围衰减器。
4. 员工04合并 React Weather 生命周期适配器及工作区焦点信号。
5. 员工03接入画布安全区与性能策略。
6. 员工04迁移工作区区域并开放已实现的设置项。
7. 员工03、04、06使用同一 Tauri Release Build 完成 `VIS-019～020`。

任何步骤失败都应保持 Classic Opaque 与静态背景可用；不得以视觉故障阻塞现有 Chat Alpha。

## 6. 验收矩阵

| 维度 | 必测组合 |
| --- | --- |
| 视口 | 最低支持尺寸、1280×720、1600×1000、缩放与多显示器 DPI |
| 材质 | Opaque、Glacé、Ybouane；全局与四个区域覆盖 |
| 背景 | 纯色、渐变、图片、视频、失败回退 |
| 氛围 | off、drizzle、rain、storm、snow、fog、Drei Cloud |
| 专注 | 空闲、输入、选字、阅读、设置、对话框、失焦 |
| 策略 | 正常、减少动态、低性能、无 WebGPU/WebGL、设备丢失 |
| 回归 | 会话切换、发送、流式、停止、错误、恢复、节点拖拽与阅读 |

硬门禁：控制台无 MindScape 新增错误；第三方已知 warning 有明确处置；键盘焦点与文字选择可用；关闭/低性能时无隐藏 Canvas 继续渲染；Classic 回退不改变业务行为；同一个 `VIS-020` Build ID 完成验收。

## 7. 当前已知工程风险

- Ybouane 许可证文本尚待生产前确认。
- React Weather 原仓的 `RainEffect` 生命周期与 attribute 初始化需要适配器接管；原型已补齐 RAF/interval 清理、停止/恢复和未使用 attribute 保护，生产仍需对 Snow/Fog 做同等回归。
- Drei 动态块在原型构建中约 859KB，虽未进入主包，生产仍需预算、压缩和缓存评审。
- Ybouane 的环境重采样必须走适配器节流：默认 250ms，输入/设置/阅读 700ms；禁止把天气、云和静态壁纸重新标成 `data-dynamic` 后恢复每帧全区域重绘。
- 玻璃节流必须按面板轮转标记，不得把多个面板合并回同帧全量刷新；目标是保持单面板采样周期，同时把每帧渲染预算控制在可观测的 P95 帧间隙内。
- React Weather 的雨滴纹理应复用已分配 GPU 存储并使用 `texSubImage2D`；每帧不得重新上传固定全屏几何、查询 uniform location 或创建碰撞临时数组。氛围退让时应暂停动态循环但保留最后一帧，恢复焦点后再继续。
- Drei 云层必须保留原 DPR、段数、体积和速度；仅允许关闭无收益 MSAA、启用 `powerPreference: high-performance` 和视锥裁剪。关闭/低性能仍需卸载 Canvas。
- 首启必须采用分阶段启动：先让壁纸、DOM 与输入可用，再加载天气、Ybouane 和 Drei 动态块；原型时序为约 `1.2s / 1.5s / 1.8s`。Ybouane 的初始 DOM 捕获应在捕获之间让出空闲帧，并在生产门禁中单独记录最大启动 Long Task，不能把稳定态 FPS 当成启动预算。
- Drei 默认纹理是远程资源，生产必须本地化。
- 当前组合原型验证了 React Weather 的 Rain 路径；Snow/Fog 的完整生产生命周期仍需适配测试。

## 8. 当前动作

员工03、04现在只需书面确认所有权、依赖和不可实现项。M1 已 Go，但 `VIS-010～020` 仍受正式范围变更、影响评估与创始人开工指令约束；收到正式开工指令后再创建生产分支。

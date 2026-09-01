# MindScape VIS-008 选定视觉源基线

> 状态：创始人已确认，VIS-008 已冻结  
> 确认日期：2026-08-20  
> 维护人：员工06（视觉与交互设计工程师）  
> 评审原型：`http://127.0.0.1:4200/`  
> 生产状态：仅为独立原型与工程规格；未授权修改 `desktop/`

## 1. 冻结结论

以下四项是后续 MindScape 视觉系统的唯一目标来源，不再用员工06自行近似的 CSS、Shader 或其他仓库替代：

| 职责 | 选定来源 | 审核快照 | 在 MindScape 中的唯一含义 |
| --- | --- | --- | --- |
| Liquid Glass | Ybouane LiquidGlass | `5ebda520bebd` | 真实 WebGL 折射、模糊、色差、Fresnel、边缘高光与圆角/Z 曲率 |
| 毛玻璃 | Glacé | `c8f5a363ab2b` | SVG 尺寸位移图、三通道边缘折射、blur、bezel、profile 与 sheen |
| 天气 | React Weather Effects | `8326628e18cf` | 雨雪雾等天气主题、前后景粒子与天气预设 |
| 云 | Drei `Cloud` / `Clouds` | `ffa15b956e32` | Three.js 广告牌云层、云团生长、颜色、密度与低频漂移 |

选型的关键不是“看起来像”，而是由统一适配器直接消费选定原仓实现，并把它们放进 MindScape 的现有研究工作区布局。Shuding Liquid Glass、Specy、Three.js 体积云、Joshbrew Realtime Clouds 和员工06早期自制近似均不属于本次冻结方案。

## 2. 布局与层级映射

```text
L0      纯色 / 渐变 / 图片 / 视频 / 兼容 Wallpaper Engine 资源
L0.5    React Weather Effects + Drei Cloud
L1      轻微、完全可关闭的全局调色
L2      Ybouane / Glacé / Classic Opaque 主卡片
L3      按钮、输入框、标签等直接控件表面
L4      设置、聚焦阅读、菜单、通知等临时浮层
```

现有左侧会话区、中央 Canvas 节点、右侧阅读区、底部编辑器和外观设置入口必须保留。全局材质与区域覆盖分离：侧边栏、节点卡片、阅读区和编辑器均可选择跟随全局、Ybouane、Glacé 或 Opaque。壁纸、材质、天气和云彼此独立，不以任一项强迫其他项开启。

## 3. 视觉参数基线

### 3.1 Ybouane Liquid Glass

原型直接使用原仓 `src/index.ts`。当前平衡默认值为：`blurAmount=0.12`、`refraction=0.78`、`chromAberration=0.06`、`edgeHighlight=0.72`、`specular=0.46`、`fresnel=1.15`、`brightness=0.04`、`tintStrength=0.22`、`opacity=0.96`、`saturation=0.08`、`distortion=0.02`、`zRadius=38`，并提供 `cornerRadius=16～64` 与 `biconvex/dome` 曲面形态调节。设置面板变更通过 `data-config` 触发 Ybouane 官方增量刷新，不销毁并重建实例。

生产适配器必须隔离原仓“玻璃元素为 root 直接子元素”及初始化时修改 `user-select` 的约束，不能让它污染整个应用根节点。

### 3.2 Glacé 毛玻璃

原型直接使用原仓 `Glass`：暗色 tone、`radius=30`、`blur=18`、`fallbackBlur=26`、`saturation=155`，默认 `refract=false`，并以 Glacé 的 CSS 主题变量控制半透明底色、边框、高光与阴影（默认表面透明度 `0.44`）。毛玻璃默认只呈现柔和的背景模糊；设置面板可打开原仓 SVG 边缘折射，并继续调节 `refract` 位移、`aberration`、`bezel`、`profile`、`sheen`、色调和半透明度。这样不会把 Glacé 默认误认成 Ybouane 式强透镜。

### 3.3 React Weather Effects

首个组合基线直接加载原仓 `RainEffect`，默认使用 `drizzle`、强度 `0.62`。Rain、Storm 可供沉浸评审；Snow、Fog 已在 `4190/4193` 原仓对照台完成来源确认，生产接入时仍归同一天气适配器。

### 3.4 Drei Cloud

云层直接加载原仓 `Cloud.tsx`，通过两组云团形成远近层次；默认总强度 `0.50`，用户可调。Drei/Three/Fiber 必须作为独立懒加载块，关闭或低性能回退时不得创建 Canvas。

## 4. 氛围退让与回退

- 空闲状态允许天气和云建立空间感，但内容中心保持安静。
- 输入聚焦、选择文字、设置打开、聚焦阅读或窗口失焦时，天气与云主动衰减。
- 减少动态停止云的连续漂移及非必要动效。
- 低性能模式停用天气和 Drei Canvas，并将 Ybouane 区域解析为 `Classic Opaque`。
- 任一第三方效果加载、Shader、纹理或设备失败，不得阻断 Chat、画布、输入或阅读。
- `Classic Opaque` 是永久回退，不得因透明视觉路线被删除。

## 5. 原型与真实浏览器证据

组合原型位于 `tools/selected-visual-baseline`，Vite alias 直接指向四个本地原仓源文件，没有复制后再按主观理解改写效果。真实浏览器终验结果：

- 1600×1000：Ybouane 5 个区域实例、天气 1 个 Canvas、Drei 1 个 Canvas，零浏览器错误。
- 1280×720：Canvas 均匹配视口，页面宽高无溢出，设置面板可滚动。
- 输入聚焦：天气透明度由 `0.62` 降至 `0.1116`，云由 `0.50` 降至 `0.09`。
- 低性能回退：天气 Canvas、Drei Canvas 和 Ybouane Canvas 均为 0，6 个主表面回退为 Opaque。
- 性能修正前：1280×720 默认组合约 7 FPS；瓶颈是 Ybouane 对标记为动态的环境层进行全区域每帧截图/合成。
- 性能修正后：适配层移除环境层的动态标记，改用 Ybouane `markChanged()` 250ms 采样（输入/设置/阅读时 700ms）。后续 VIS-008 优化将六块玻璃改为按面板轮转标记，保持每块相同采样周期，避免同帧突发；React Weather 的原尺寸纹理使用稳定分配后的 `texSubImage2D`，雨滴模拟去掉每帧临时数组和几何缓冲分配，Drei 保留原 DPR/段数并启用视锥裁剪。
- 当前原型复测（全新浏览器会话，1280×720，2.5s 预热 + 两轮 5s）：默认组合约 114 FPS、P95 帧间隙约 19.5–20.5ms、最大约 25–29ms，无稳定态 Long Task；输入态约 240 FPS，P95 约 4.4ms。连续重载 3 次保持 1 个天气 Canvas、8 个总 Canvas。首启分阶段把天气、Ybouane 和 Drei 分别推迟到 1.2s / 1.5s / 1.8s，先释放 DOM 输入响应；初始化捕获仍需生产预算。
- 历史基线曾产生 4 条 React Weather WebGL attribute warning；适配层现已对未使用 attribute 做显式保护，当前浏览器复测为 0 errors / 0 warnings。生产合入仍需保留该保护并跑 Snow/Fog 路径回归。

截图证据位于 `output/playwright/selected-visual-baseline/`。

## 6. 许可与生产前置条件

Glacé、React Weather Effects 与 Drei 为 MIT，移植时必须保留原始许可证及版权声明。Ybouane 的包元数据声明 MIT，但审核快照根目录缺少独立 `LICENSE`；生产采用前必须取得或确认可归档的许可证文本。

Drei 默认云纹理来自远程资源，生产构建必须改为审核、版本化并随包分发的本地资产。React Weather 原仓面向 Next.js 的静态图片 import 和组件清理假设必须由适配器显式处理。

## 7. 门禁

本文件完成 `VIS-008`，并为 `VIS-009` 提供视觉输入。M1 已于 2026-08-20 Go，但这只满足 D-045 的第一项条件，不等于视觉生产开工。员工03、04只能先评审交接和做影响分析；在正式范围变更、工程影响评估和创始人开工指令全部通过之前，不得把该系统写入 `desktop/`。

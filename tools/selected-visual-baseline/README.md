# MindScape Selected Visual Baseline

这是创始人确认 `VIS-008` 后的独立视觉基线，不是正式桌面实现。

## 已锁定来源

- Liquid Glass：Ybouane LiquidGlass，直接从 `example/liquid glass/liquidglass-main/src` 编译。
- 毛玻璃：Glacé，直接从 `example/liquid glass/glace-main/src` 编译。
- 天气：React Weather Effects，当前组合基线直接加载原仓 `RainEffect`；Snow/Fog 继续在 4190 原仓对照台验收。
- 云：Drei `Cloud.tsx`，直接从 `example/drei-master/src/core/Cloud.tsx` 编译。

Vite alias 指向本地原仓源文件，没有复制一份再自行改写 Shader。

React Weather 原仓依赖 Next.js 静态图片 import 的 `{ src }` 形状；Vite 配置中的 `mindscape-next-image-shape-compat` 只在编译时把两张雨滴纹理 URL 包装成同一数据形状，不修改雨滴算法、Shader或天气参数。

## 启动

```powershell
cd D:\Project\MindScape\tools\selected-visual-baseline
pnpm install
powershell -ExecutionPolicy Bypass -File .\scripts\start.ps1
```

打开 `http://127.0.0.1:4200/`。

停止：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\stop.ps1
```

## 评审顺序

1. 全局切换 Ybouane、Glacé 与 Classic Opaque。
2. 使用区域覆盖验证侧边栏、节点、阅读区和编辑器可以采用不同材质。
3. 聚焦输入、打开设置或进入阅读，确认天气与云主动退让。
4. 开启减少动态和低性能回退；低性能模式卸载天气与 Drei Canvas，并把 Ybouane 区域回退到 Opaque。

## 性能基线

原始组合在 1280×720 浏览器里测得约 7 FPS，原因是 Ybouane 把 `data-dynamic` 环境层当成每帧变化源，并对全部玻璃区域重复截图和 WebGL 合成。当前适配层移除了静态/独立环境的动态标记，改用 Ybouane `markChanged()` 以 250ms 采样；输入、设置和阅读时采样放宽到 700ms。

修正后又补了一轮游戏式帧预算优化。全新浏览器会话在 1280×720 组合下以 `2.5s` 预热 + `2 × 5s` 采样稳定得到约 `114 FPS`，P95 帧间隙约 `19.5–20.5ms`、最大约 `25–29ms`，采样期间没有 Long Task。输入聚焦时天气与云保持原定退让透明度 `0.1116` / `0.09`，动态循环暂停，输入态约 `240 FPS`、P95 帧间隙约 `4.4ms`；失焦后循环恢复。

首启采用分阶段启动：前 `1.2s` 保证壁纸、DOM 和输入先可用，天气随后加载，Ybouane 玻璃在 `1.5s` 后启动，Drei 云在 `1.8s` 后启动；最终仍保持 1 个天气 Canvas、1 个云 Canvas 和 6 个玻璃 Canvas。首启 Long Task 被推迟到效果初始化窗口（约 `325ms` / `279ms` 单次捕获），不再阻塞最初的输入响应；生产集成仍需把这两次初始化捕获纳入启动预算或迁移到原生渲染线程。

本轮没有降低雨滴数量、纹理尺寸、云层段数、Drei `Cloud` 参数或任何 Shader 画质参数：

- Ybouane 玻璃刷新从一次全局批量标记改为按玻璃面板轮转标记，空闲时每块保持 `250ms` 更新周期，输入/设置/阅读时保持 `700ms` 周期，避免同一帧六块玻璃同时做截图与 WebGL 合成。
- React Weather 保持原算法，去除每帧的几何缓冲分配、uniform 查询和水滴碰撞 `slice()` 分配；水滴纹理使用稳定分配后的 `texSubImage2D` 更新；补齐 RAF 停止/恢复与卸载清理。
- Drei Canvas 保持原 DPR、体积、段数与运动速度；关闭无收益的 MSAA、启用高性能 GPU 提示和视锥裁剪；氛围退让时使用 demand 渲染而不是继续空转。

连续重载 3 次后页面仍保持 `1` 个天气 Canvas、`8` 个总 Canvas（包含 6 个玻璃画布），控制台 `0 errors / 0 warnings`。优化目标是消除长帧和泄漏，不是用降分辨率或减少粒子换取平均 FPS。

## 已知原仓边界

- Ybouane 要求玻璃元素是 root 的直接子元素，且初始化时会设置 root 的 `user-select`；正式适配器需要隔离这一约束。
- React Weather Effects 的 `RainEffect` 缺少完整清理函数，因此本原型切换雨型时重载页面，避免遗留 WebGL 与 interval。
- React Weather 原仓当前会产生 4 条 WebGL attribute warning；组合原型没有新增浏览器 error，生产适配前仍需处理这些 warning。
- Drei Cloud 默认纹理来自其原仓远程资源；正式实现需要审核后版本化入包。
- Drei/Three/Fiber 保持独立懒加载块；关闭云或低性能回退时不创建 Canvas。
- 本原型不是 M1 验收 Build，不授权修改 `desktop/`。

浏览器截图与终验记录位于 `output/playwright/selected-visual-baseline/`，工程交接见 `docs/engineering/selected-visual-runtime-handoff-20260820.md`。

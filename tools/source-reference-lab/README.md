# MindScape Source Reference Lab

这是原仓库效果对照工具，不是 MindScape 最终 UI，也不把多个实现融合成新材质。

## 对照内容

- Glacé：直接运行 `example/liquid glass/glace-main/site`。
- Shuding Liquid Glass：测试场景加载未修改的 `liquid-glass.js`。
- Ybouane LiquidGlass：直接运行仓库自带 `site/index.html`。
- React Weather Effects：直接运行仓库的 Next.js Demo。
- Drei / Three.js：使用官方 Storybook 和官方 Examples。
- Joshbrew Realtime Clouds：直接运行仓库 WebGPU Demo。

## 启动

首次准备：

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\source-reference-lab\scripts\prepare-references.ps1
```

启动全部本地页面：

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\source-reference-lab\scripts\start-references.ps1
```

打开 `http://127.0.0.1:4190/`。

端口：4190 对照台、4191 Glacé、4192 Ybouane、4193 Weather、4194 Realtime Clouds。

## 判断规则

先判断原始观感，再判断交互反馈，最后判断性能边界。在创始人明确选择以前，不把任何效果写入 `desktop/`。

## 原仓运行说明

- React Weather Effects 使用原仓开发服务器。原仓生产构建的 `/snow` 与 `/fog` 路由会返回 500，因此对照台不使用 `next start` 冒充可用版本。
- React Weather Effects 的雨景在浏览器控制台会出现原仓自身的 WebGL attribute 警告；对照台不修改原代码，也不会隐藏这些警告。
- Joshbrew Realtime Clouds 首次进入需要数秒初始化 worker 与云层采样，并依赖 WebGPU / OffscreenCanvas；初始化期间的空白不是对照台替换或降级后的画面。
- Drei 与 Three.js 项目通过其官方在线页面加载，因此需要联网；其他标记为“本地原仓”的项目均从 `example/` 独立运行。

## 许可说明

Glacé、Shuding、React Weather Effects、Drei、Three.js 与 Joshbrew 仓库均包含 MIT 许可。Ybouane 的包元数据声明 MIT，但当前仓库根目录没有独立 `LICENSE` 文件，正式移植前需要再次确认。

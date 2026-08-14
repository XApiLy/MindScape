# MindScape Desktop

MindScape 第一版桌面客户端。当前垂直切片包括无限会话画布、外部会话导入、多厂商模型适配和基础 Chat。

## 本地运行

```bash
pnpm install
pnpm dev
```

浏览器开发地址为 `http://127.0.0.1:1420/`。

运行 Tauri 桌面窗口：

```bash
pnpm tauri dev
```

验证生产构建：

```bash
pnpm build
cd src-tauri
cargo check
```

## 模块边界

- `src/components/canvas`：无限画布、会话节点、分支与连线。
- `src/components/dialogs`：外部会话导入和模型供应商配置。
- `src/services/importer.ts`：Markdown、JSON、JSONL、纯文本归一化。
- `src/services/providers.ts`：OpenAI-compatible、Anthropic、Gemini 流式协议适配。
- `src/store/workspaceStore.ts`：画布与会话状态、浏览器本地持久化边界。
- `src/types/workspace.ts`：跨模块共享契约。

前端组件只依赖统一的 `ProviderConfig` 和流式文本回调，不应直接绑定某个厂商的响应结构。导入器必须保留原始会话来源，派生分析不得覆盖原文。

## 当前限制

- API Key 只保留在当前运行内存中；生产版需接入操作系统安全凭据或后端代理。
- 浏览器开发模式可能被供应商 CORS 阻止，桌面版应通过受控请求层调用。
- 当前持久化使用 Zustand localStorage；SQLite、事件溯源和分析版本仍待接入。
- 通用导入已可用，Claude 与 Codex 的真实导出格式仍需样本驱动的专用适配器。
- 主前端包约 573 KB，后续按画布、导入器和供应商设置拆分。

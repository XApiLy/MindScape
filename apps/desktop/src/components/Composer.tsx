import { useMemo, useRef, useState } from "react";
import { Globe2, Lightbulb, Send, Settings2, Square } from "lucide-react";
import { demoResponse, streamProviderChat } from "../services/providers";
import { toMessagesForNode, useWorkspaceStore } from "../store/workspaceStore";

type ComposerProps = {
  onProviders: () => void;
};

export function Composer({ onProviders }: ComposerProps) {
  const [prompt, setPrompt] = useState("");
  const [running, setRunning] = useState(false);
  const abortRef = useRef<AbortController | null>(null);
  const nodes = useWorkspaceStore((state) => state.nodes);
  const selectedNodeId = useWorkspaceStore((state) => state.selectedNodeId);
  const providerConfigs = useWorkspaceStore((state) => state.providerConfigs);
  const activeProviderId = useWorkspaceStore((state) => state.activeProviderId);
  const setActiveProvider = useWorkspaceStore((state) => state.setActiveProvider);
  const addPromptNode = useWorkspaceStore((state) => state.addPromptNode);
  const updateNodeContent = useWorkspaceStore((state) => state.updateNodeContent);

  const provider = useMemo(
    () => providerConfigs.find((item) => item.id === activeProviderId) ?? providerConfigs[0],
    [activeProviderId, providerConfigs],
  );

  const submit = async () => {
    const nextPrompt = prompt.trim();
    if (!nextPrompt || running || !provider) return;
    setPrompt("");
    setRunning(true);
    const nodeId = addPromptNode(nextPrompt, provider.model, selectedNodeId);
    const parent = nodes.find((node) => node.id === selectedNodeId);
    const messages = [
      {
        id: "system-ms",
        role: "system" as const,
        content: "你是 MindScape 中的探索助手。回答应清晰、可追溯，并指出不确定内容。",
        createdAt: new Date().toISOString(),
      },
      ...(parent ? toMessagesForNode(parent) : []),
      {
        id: `${nodeId}-prompt`,
        role: "user" as const,
        content: nextPrompt,
        createdAt: new Date().toISOString(),
      },
    ];
    let content = "";

    try {
      if (!provider.apiKey) {
        await new Promise((resolve) => window.setTimeout(resolve, 620));
        content = demoResponse(nextPrompt);
        updateNodeContent(nodeId, content);
      } else {
        abortRef.current = new AbortController();
        await streamProviderChat(provider, messages, {
          signal: abortRef.current.signal,
          onChunk: (chunk) => {
            content += chunk;
            updateNodeContent(nodeId, content, "thinking");
          },
        });
        updateNodeContent(nodeId, content || "模型没有返回文本内容。", "ready");
      }
    } catch (error) {
      updateNodeContent(
        nodeId,
        `调用失败：${error instanceof Error ? error.message : "未知错误"}\n\n你可以检查模型接口设置，或继续使用本地演示模式。`,
        "error",
      );
    } finally {
      abortRef.current = null;
      setRunning(false);
    }
  };

  return (
    <div className="composer-wrap">
      <div className="composer">
        <select
          value={activeProviderId}
          onChange={(event) => setActiveProvider(event.target.value)}
          aria-label="选择模型"
        >
          {providerConfigs.filter((item) => item.enabled).map((item) => (
            <option key={item.id} value={item.id}>
              {item.model}
            </option>
          ))}
        </select>
        <textarea
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void submit();
            }
          }}
          rows={1}
          placeholder="探索一个问题…（Enter 发送，Shift+Enter 换行）"
        />
        <button className="composer-icon" title="提示辅助" aria-label="提示辅助">
          <Lightbulb size={17} />
        </button>
        <button className="composer-icon" title="联网工具" aria-label="联网工具">
          <Globe2 size={17} />
        </button>
        <button className="composer-icon" title="模型设置" aria-label="模型设置" onClick={onProviders}>
          <Settings2 size={17} />
        </button>
        <button
          className={`send-button ${running ? "is-running" : ""}`}
          type="button"
          title={running ? "停止生成" : "发送"}
          aria-label={running ? "停止生成" : "发送"}
          onClick={() => {
            if (running) {
              abortRef.current?.abort();
              setRunning(false);
            } else {
              void submit();
            }
          }}
        >
          {running ? <Square size={16} /> : <Send size={17} />}
        </button>
      </div>
      {!provider?.apiKey ? <span className="demo-badge">演示模式 · 配置 API Key 后启用真实模型</span> : null}
    </div>
  );
}

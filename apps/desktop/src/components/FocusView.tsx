import { ArrowDown, ArrowRight, Bookmark, GitBranch, Network, X } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useWorkspaceStore } from "../store/workspaceStore";
import type { ConversationNode } from "../types/workspace";

export function FocusView({ node }: { node: ConversationNode }) {
  const focusNode = useWorkspaceStore((state) => state.focusNode);
  const branchFrom = useWorkspaceStore((state) => state.branchFrom);

  return (
    <div className="focus-backdrop" role="dialog" aria-modal="true" aria-label="聚焦阅读">
      <section className="focus-panel">
        <header className="focus-header">
          <div>
            <h2>{node.data.title}</h2>
            <div className="focus-meta">
              <span>#{node.id.slice(-6)}</span>
              <span>{node.data.model}</span>
              <span className="mainline-chip">主线</span>
            </div>
          </div>
          <div className="focus-actions">
            <button title="收藏" aria-label="收藏"><Bookmark size={16} /></button>
            <button title="返回画布" aria-label="返回画布"><Network size={16} /></button>
            <button title="关闭" aria-label="关闭" onClick={() => focusNode(null)}><X size={17} /></button>
          </div>
        </header>
        <div className="focus-scroll">
          <p className="focus-prompt">{node.data.prompt}</p>
          <div className="focus-reasoning"><span className="status-dot" />{node.data.reasoningLabel}</div>
          <div className="markdown-body focus-markdown">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{node.data.content}</ReactMarkdown>
          </div>
        </div>
        <footer className="focus-footer">
          <span>衍生新视角</span>
          <div>
            <button onClick={() => branchFrom(node.id, "deep")}><ArrowRight size={14} />深入</button>
            <button onClick={() => branchFrom(node.id, "parallel")}><GitBranch size={14} />发散</button>
            <button onClick={() => branchFrom(node.id, "alternate")}><ArrowDown size={14} />换角度</button>
          </div>
        </footer>
      </section>
    </div>
  );
}

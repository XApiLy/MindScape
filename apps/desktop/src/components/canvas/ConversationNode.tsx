import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { ArrowDown, ArrowRight, Copy, Expand, GitBranch, LoaderCircle, X } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useWorkspaceStore } from "../../store/workspaceStore";
import type { ConversationNode } from "../../types/workspace";

function ConversationNodeComponent({ id, data, selected }: NodeProps<ConversationNode>) {
  const branchFrom = useWorkspaceStore((state) => state.branchFrom);
  const focusNode = useWorkspaceStore((state) => state.focusNode);

  return (
    <article className={`conversation-card branch-${data.branchKind} ${selected ? "selected" : ""}`}>
      <Handle type="target" position={Position.Left} className="node-handle" />
      <header className="card-header">
        <div className="card-meta">
          <span className="node-id">#{id.slice(-6)}</span>
          <span>{data.model}</span>
        </div>
        <div className="card-actions">
          <button title="复制内容" aria-label="复制内容" onClick={() => navigator.clipboard?.writeText(data.content)}>
            <Copy size={13} />
          </button>
          <button title="聚焦阅读" aria-label="聚焦阅读" onClick={() => focusNode(id)}>
            <Expand size={13} />
          </button>
          <button title="关闭卡片" aria-label="关闭卡片">
            <X size={13} />
          </button>
        </div>
      </header>

      <div className="card-body">
        <div className="card-title-row">
          <h2>{data.title}</h2>
          <time>{data.createdAt}</time>
        </div>
        {data.prompt ? <p className="card-prompt">{data.prompt}</p> : null}
        <div className={`markdown-body ${data.status === "thinking" ? "is-thinking" : ""}`}>
          {data.status === "thinking" ? <LoaderCircle className="spin" size={17} /> : null}
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{data.content}</ReactMarkdown>
        </div>
        {data.reasoningLabel ? (
          <button className="reasoning-strip" type="button">
            <span className="status-dot" />
            {data.reasoningLabel}
            <ArrowRight size={13} />
          </button>
        ) : null}
        <div className="tag-row">
          {data.tags.map((tag) => (
            <span key={tag}>{tag}</span>
          ))}
        </div>
      </div>

      <footer className="card-footer">
        <button type="button" onClick={() => branchFrom(id, "deep")}>
          <ArrowRight size={14} />
          深入
        </button>
        <button type="button" onClick={() => branchFrom(id, "parallel")}>
          <GitBranch size={14} />
          发散
        </button>
        <button type="button" onClick={() => branchFrom(id, "alternate")}>
          <ArrowDown size={14} />
          换角度
        </button>
      </footer>
      <Handle type="source" position={Position.Right} className="node-handle" />
    </article>
  );
}

export const ConversationNodeView = memo(ConversationNodeComponent);

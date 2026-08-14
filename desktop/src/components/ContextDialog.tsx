import { Eye, FileQuestion, MinusCircle, X } from "lucide-react";
import type { ContentBlock, ContextSnapshot } from "../domain";

type ContextDialogProps = {
  open: boolean;
  loading: boolean;
  snapshot: ContextSnapshot | null;
  error: string | null;
  onClose: () => void;
};

function blocksToPlainText(blocks: ContentBlock[]) {
  return blocks
    .map((block) => {
      if (block.type === "text") return block.text;
      if (block.type === "code") return block.code;
      if (block.type === "link") return block.label ?? block.url;
      if (block.type === "attachmentRef") return `[附件：${block.displayName}]`;
      if (block.type === "toolCallRef") return `[工具调用：${block.toolRunId}]`;
      if (block.type === "toolResultRef") return `[工具结果：${block.toolRunId}]`;
      return `[暂不支持的内容：${block.originalType}]`;
    })
    .join("\n");
}

export function ContextDialog({ open, loading, snapshot, error, onClose }: ContextDialogProps) {
  if (!open) return null;

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="settings-dialog context-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="context-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="dialog-header">
          <div>
            <span className="eyebrow">ACTUAL CONTEXT SNAPSHOT</span>
            <h2 id="context-title">本轮实际使用的上下文</h2>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label="关闭上下文详情">
            <X aria-hidden="true" />
          </button>
        </header>

        <div className="context-content">
          {loading ? <p className="context-status">正在读取冻结快照…</p> : null}
          {error ? (
            <div className="context-unavailable"><FileQuestion aria-hidden="true" /><p>{error}</p></div>
          ) : null}
          {snapshot ? (
            <>
              <div className="context-summary">
                <span>协议 {snapshot.systemContractVersion}</span>
                <span>预计 {snapshot.estimatedTokens} tokens</span>
                <span>{snapshot.branchType}</span>
              </div>
              <div className="context-section">
                <h3><Eye aria-hidden="true" />已引用消息</h3>
                {snapshot.selectedMessages.length ? snapshot.selectedMessages.map((message) => (
                  <article key={message.messageId}>
                    <span>{message.role} · {message.sourceNodeId}</span>
                    <p>{blocksToPlainText(message.contentBlocks)}</p>
                  </article>
                )) : <p className="context-status">本轮没有继承历史消息。</p>}
              </div>
              <div className="context-section">
                <h3><MinusCircle aria-hidden="true" />已排除内容</h3>
                {snapshot.omittedMessages.length ? snapshot.omittedMessages.map((message) => (
                  <article key={message.messageId}>
                    <span>{message.messageId}</span>
                    <p>{message.reason}</p>
                  </article>
                )) : <p className="context-status">没有需要说明的排除项。</p>}
              </div>
            </>
          ) : null}
        </div>
      </section>
    </div>
  );
}

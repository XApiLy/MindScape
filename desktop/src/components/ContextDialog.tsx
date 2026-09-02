import { Eye, FileQuestion, FileText, ListChecks, MinusCircle, ReceiptText, ShieldCheck, X } from "lucide-react";
import { projectContextBill } from "../app/contextBill";
import type { ContextSnapshot } from "../domain";

type ContextDialogProps = {
  open: boolean;
  loading: boolean;
  snapshot: ContextSnapshot | null;
  error: string | null;
  onClose: () => void;
};

export function ContextDialog({ open, loading, snapshot, error, onClose }: ContextDialogProps) {
  if (!open) return null;
  const bill = snapshot ? projectContextBill(snapshot) : null;

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
          {bill ? (
            <>
              {/*
                UI-HANDOFF-06
                位置：冻结上下文详情弹窗的账单、来源、约束与排除项区域
                用途：展示本轮真实 ContextSnapshot 的继承范围、来源证据和预算边界
                数据/IPC：父级通过 get_context_snapshot 获取快照；projectContextBill 负责纯投影，组件不读文件或 SQLite
                状态：加载显示读取提示；空白显示“没有继承/引用/约束”；错误显示安全错误；停止/恢复由快照状态和既有运行卡片承载
                交互约束：保持 dialog 语义、关闭按钮焦点和可滚动键盘路径；不展示 Key、原始请求或完整推理
                可替换范围：员工06可替换账单网格、分组卡片、折叠方式和动效
                不可改变：ContextSnapshot 字段、来源/排除语义、get_context_snapshot 调用、状态文案选择器和安全边界
              */}
              <div className="context-ledger" aria-label="上下文账单">
                <span><small>上下文估算</small><strong>{bill.estimatedTokens} tokens</strong></span>
                <span><small>历史消息</small><strong>{bill.metrics.messages}</strong></span>
                <span><small>外部来源</small><strong>{bill.metrics.importSources}</strong></span>
                <span><small>显式约束</small><strong>{bill.metrics.constraints}</strong></span>
                <span><small>排除项</small><strong>{bill.metrics.omitted}</strong></span>
              </div>
              <div className="context-manifest-meta">
                <span>{bill.branchLabel}</span>
                <span>协议 {bill.protocolVersion}</span>
              </div>
              <div className="context-section">
                <h3><ReceiptText aria-hidden="true" />本轮输入</h3>
                <article className="context-current-input"><p>{bill.currentInput}</p></article>
              </div>
              <div className="context-section">
                <h3><Eye aria-hidden="true" />已引用消息</h3>
                {bill.messages.length ? bill.messages.map((message) => (
                  <article key={message.id}>
                    <span>{message.roleLabel} · {message.sourceNodeId}</span>
                    <p>{message.text}</p>
                  </article>
                )) : <p className="context-status">本轮没有继承历史消息。</p>}
              </div>
              <div className="context-section">
                <h3><FileText aria-hidden="true" />外部来源</h3>
                {bill.importSources.length ? bill.importSources.map((source) => (
                  <article key={source.id}>
                    <span>{source.sourceKind} · {source.targetLabel}</span>
                    <p>{source.excerpt ?? "该引用没有可展示的摘录。"}</p>
                  </article>
                )) : <p className="context-status">本轮没有引用导入原文、附件或工具结果。</p>}
              </div>
              <div className="context-section">
                <h3><ListChecks aria-hidden="true" />显式约束</h3>
                {bill.constraints.length ? bill.constraints.map((constraint) => (
                  <article key={constraint.id}>
                    <span>{constraint.userConfirmed ? "用户已确认" : "尚未确认"} · {constraint.evidenceCount} 条证据</span>
                    <p>{constraint.text}</p>
                  </article>
                )) : <p className="context-status">本轮没有显式约束。</p>}
              </div>
              <div className="context-section">
                <h3><MinusCircle aria-hidden="true" />已排除内容</h3>
                {bill.omitted.length ? bill.omitted.map((message) => (
                  <article key={message.messageId}>
                    <span>{message.messageId}</span>
                    <p>{message.reason}</p>
                  </article>
                )) : <p className="context-status">没有需要说明的排除项。</p>}
              </div>
              <div className="context-budget-boundary">
                <ShieldCheck aria-hidden="true" />
                <div><strong>预算边界</strong><p>{bill.budgetNotice}</p></div>
              </div>
            </>
          ) : null}
        </div>
      </section>
    </div>
  );
}

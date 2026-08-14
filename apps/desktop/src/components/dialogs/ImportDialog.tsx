import { useRef, useState } from "react";
import { Check, FileJson, FileText, Upload, X } from "lucide-react";
import { importConversationFile } from "../../services/importer";
import { useWorkspaceStore } from "../../store/workspaceStore";
import type { AnalysisLevel, ImportedConversation } from "../../types/workspace";

type ImportDialogProps = {
  open: boolean;
  onClose: () => void;
};

const levels: Array<{ id: AnalysisLevel; title: string; description: string; cost: string }> = [
  { id: "raw", title: "原样继续", description: "不做语义分析，完整保留原始消息。", cost: "无前置消耗" },
  { id: "quick", title: "快速识别", description: "提取目标、最后进展、约束与下一步。", cost: "推荐" },
  { id: "detailed", title: "详细分析", description: "理解完整脉络、决策、分歧与工具调用。", cost: "较高消耗" },
];

export function ImportDialog({ open, onClose }: ImportDialogProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [level, setLevel] = useState<AnalysisLevel>("quick");
  const [preview, setPreview] = useState<ImportedConversation | null>(null);
  const [error, setError] = useState("");
  const importConversation = useWorkspaceStore((state) => state.importConversation);

  if (!open) return null;

  const readFile = async (file?: File) => {
    if (!file) return;
    setError("");
    try {
      setPreview(await importConversationFile(file));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "无法读取这个文件");
    }
  };

  return (
    <div className="dialog-backdrop" role="dialog" aria-modal="true" aria-label="导入外部会话">
      <section className="dialog-panel import-dialog">
        <header className="dialog-header">
          <div>
            <span className="eyebrow">CONVERSATION IMPORT</span>
            <h2>导入外部 AI 会话</h2>
            <p>支持 Markdown、JSON、JSONL 与纯文本。原始文件不会被分析结果覆盖。</p>
          </div>
          <button className="icon-button" title="关闭" aria-label="关闭" onClick={onClose}><X size={17} /></button>
        </header>

        <button className="drop-zone" type="button" onClick={() => inputRef.current?.click()}>
          <Upload size={24} />
          <strong>{preview ? preview.title : "选择或拖入会话文件"}</strong>
          <span>{preview ? `${preview.messages.length} 条消息 · ${preview.source}` : ".md · .json · .jsonl · .txt"}</span>
          <input
            ref={inputRef}
            type="file"
            accept=".md,.markdown,.json,.jsonl,.txt"
            onChange={(event) => void readFile(event.target.files?.[0])}
          />
        </button>

        {preview ? (
          <div className="import-preview">
            <div><FileText size={16} /><span>用户消息</span><strong>{preview.messages.filter((item) => item.role === "user").length}</strong></div>
            <div><FileJson size={16} /><span>AI 消息</span><strong>{preview.messages.filter((item) => item.role === "assistant").length}</strong></div>
            <div><Check size={16} /><span>完整性</span><strong>{preview.warnings.length ? "需检查" : "正常"}</strong></div>
          </div>
        ) : null}

        <div className="analysis-levels">
          <div className="section-title-row">
            <strong>如何接续？</strong>
            <span>随时可以删除分析层，原文始终保留</span>
          </div>
          {levels.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`analysis-option ${level === item.id ? "selected" : ""}`}
              onClick={() => setLevel(item.id)}
            >
              <span className="radio-mark">{level === item.id ? <Check size={13} /> : null}</span>
              <span><strong>{item.title}</strong><small>{item.description}</small></span>
              <em>{item.cost}</em>
            </button>
          ))}
        </div>

        {error ? <p className="dialog-error">{error}</p> : null}
        <footer className="dialog-footer">
          <button className="secondary-button" type="button" onClick={onClose}>取消</button>
          <button
            className="accent-button"
            type="button"
            disabled={!preview}
            onClick={() => {
              if (!preview) return;
              importConversation(preview, level);
              onClose();
            }}
          >
            导入并继续
          </button>
        </footer>
      </section>
    </div>
  );
}

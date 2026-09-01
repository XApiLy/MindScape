import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  Check,
  Copy,
  Database,
  FileText,
  FileUp,
  LoaderCircle,
  LockKeyhole,
  TextCursorInput,
  X,
} from "lucide-react";
import {
  formatImportBytes,
  importFormatLabel,
  inspectImportFile,
  inspectPastedConversation,
  type ImportIntakeCandidate,
} from "../app/importIntake";
import {
  importedRoleLabel,
  importMessageMarkdown,
  IMPORT_PREVIEW_PAGE_SIZE,
  nextImportPreviewCount,
  rawImportContentIntegrityIssue,
} from "../app/importPreview";
import type {
  GenericImportCommandResult,
  ImportBundleQueryProjection,
  ImportedMessage,
  ImportSource,
  RawImportContentProjection,
} from "../domain";
import { renderedMarkdownText, SafeMarkdown } from "./SafeMarkdown";

type ImportMode = "file" | "paste";
type MessagePreviewMode = "rendered" | "raw";
type ImportPreviewMode = MessagePreviewMode | "source";
type RawImportContentLoadState =
  | { kind: "unavailable" }
  | { kind: "loading" }
  | { kind: "ready"; projection: RawImportContentProjection }
  | { kind: "error"; message: string };
const PASTED_IMPORT_FILE_NAME = "pasted-session.txt";

type ImportIntakeDialogProps = {
  open: boolean;
  onClose: () => void;
  onImportGenericFile?: (
    originalFileName: string,
    payload: number[],
  ) => Promise<GenericImportCommandResult>;
  importSources?: readonly ImportSource[];
  importSourcesLoading?: boolean;
  onLoadImportBundle?: (sourceId: string) => Promise<ImportBundleQueryProjection>;
  onLoadRawImportContent?: (sourceId: string) => Promise<RawImportContentProjection>;
};

function CandidateBill({ candidate }: { candidate: ImportIntakeCandidate }) {
  const ready = candidate.issues.length === 0;
  return (
    <section className={`import-candidate-bill${ready ? " is-ready" : " is-blocked"}`} aria-live="polite">
      <header>
        <span className="import-file-mark" aria-hidden="true"><FileText /></span>
        <div>
          <strong>{candidate.displayName}</strong>
          <small>{candidate.kind === "file" ? "本地文件" : "临时粘贴内容"}</small>
        </div>
        <span className="import-readiness">
          {ready ? <Check aria-hidden="true" /> : <AlertTriangle aria-hidden="true" />}
          {ready ? "可交给本地解析" : "需要处理"}
        </span>
      </header>
      <div className="import-source-ledger" aria-label="原文保真预检">
        <span><small>格式</small><strong>{importFormatLabel(candidate.format)}</strong></span>
        <span><small>大小</small><strong>{formatImportBytes(candidate.sizeBytes)}</strong></span>
        <span><small>当前处理</small><strong>本地内核解析</strong></span>
      </div>
      {candidate.issues.length ? (
        <ul className="import-issue-list">
          {candidate.issues.map((issue) => <li key={issue.code}>{issue.message}</li>)}
        </ul>
      ) : (
        <p className="import-ready-note">原文只会交给本地内核保存与解析，不调用模型，也不执行历史指令。</p>
      )}
    </section>
  );
}

function ImportResultSummary({ result }: { result: GenericImportCommandResult }) {
  return (
    <section className="import-command-result" aria-live="polite">
      <strong>{result.duplicate ? "来源已存在，已复用原文指纹" : "已生成本地导入预览"}</strong>
      <span>{result.revision.status} · {result.report.messageCount} 条消息 · {result.report.warnings.length} 条警告 · {result.report.errors.length} 条错误</span>
      <small>{result.source.originalFileName ?? "未命名来源"} · revision {result.revision.id}</small>
    </section>
  );
}

function ImportMessagePreview({
  message,
  mode,
}: {
  message: ImportedMessage;
  mode: MessagePreviewMode;
}) {
  const [copied, setCopied] = useState(false);
  const renderedRef = useRef<HTMLDivElement>(null);
  const markdown = useMemo(() => importMessageMarkdown(message), [message]);

  const copyVisibleContent = async () => {
    const content = mode === "raw" ? markdown : renderedMarkdownText(renderedRef.current);
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      setCopied(false);
    }
  };

  return (
    <article className="import-preview-message">
      <header>
        <span>
          <strong>{importedRoleLabel(message.role)}</strong>
          <small>{message.sourceLocator}</small>
        </span>
        <button
          type="button"
          disabled={!markdown}
          onClick={() => void copyVisibleContent()}
          aria-label={mode === "raw" ? "复制该消息的 Markdown 原文" : "复制该消息的阅读文本"}
        >
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          {copied ? "已复制" : mode === "raw" ? "复制 Markdown" : "复制阅读文本"}
        </button>
      </header>
      {markdown ? mode === "raw" ? (
        <pre className="import-preview-raw" tabIndex={0}>{markdown}</pre>
      ) : (
        <SafeMarkdown markdown={markdown} contentRef={renderedRef} className="import-preview-rendered" />
      ) : (
        <p className="import-preview-empty">该消息没有可见正文；原始记录仍保留在本地导入事实中。</p>
      )}
    </article>
  );
}

function RawImportSourcePreview({
  state,
  onRetry,
}: {
  state: RawImportContentLoadState;
  onRetry: () => void;
}) {
  const [copied, setCopied] = useState(false);

  if (state.kind === "loading") {
    return <p className="import-source-raw-state" role="status"><LoaderCircle className="spin" aria-hidden="true" />正在通过本地内核校验并读取受控原文…</p>;
  }
  if (state.kind === "unavailable") {
    return <p className="import-source-raw-state">当前运行环境未提供受控原文读取；不会尝试拼接本地路径。</p>;
  }
  if (state.kind === "error") {
    return (
      <div className="import-source-raw-state is-error">
        <p role="alert">受控原文读取失败：{state.message}</p>
        <button type="button" onClick={onRetry}><LoaderCircle aria-hidden="true" />重新读取</button>
      </div>
    );
  }

  const { projection } = state;
  const visibleBytes = new TextEncoder().encode(projection.content).byteLength;
  const copySource = async () => {
    if (!projection.content) return;
    try {
      await navigator.clipboard.writeText(projection.content);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="import-source-raw">
      <header>
        <div>
          <strong>{projection.truncated ? "受控原文预览（已截断）" : "受控原文（完整）"}</strong>
          <span>
            {formatImportBytes(visibleBytes)} 可见 / {formatImportBytes(projection.byteLength)} 完整
          </span>
          <code title={projection.contentHash}>SHA-256 {projection.contentHash.slice(0, 16)}…</code>
        </div>
        <button type="button" disabled={!projection.content} onClick={() => void copySource()}>
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          {copied ? "已复制" : projection.truncated ? "复制当前预览" : "复制源文件文本"}
        </button>
      </header>
      {projection.truncated ? (
        <p className="import-preview-boundary">内容超过内核预览上限；此处只展示并复制已校验的前段，完整原文仍留在本地内容寻址存储中。</p>
      ) : null}
      {projection.content ? (
        <pre className="import-preview-raw import-source-raw-content" tabIndex={0}>{projection.content}</pre>
      ) : (
        <p className="import-preview-empty">受控来源没有可显示的 UTF-8 文本。</p>
      )}
    </div>
  );
}

function ImportBundlePreview({
  bundle,
  rawContentState,
  onRetryRawContent,
}: {
  bundle: ImportBundleQueryProjection;
  rawContentState: RawImportContentLoadState;
  onRetryRawContent: () => void;
}) {
  const [previewMode, setPreviewMode] = useState<ImportPreviewMode>("rendered");
  const [visibleMessageCount, setVisibleMessageCount] = useState(IMPORT_PREVIEW_PAGE_SIZE);
  const issues = [...bundle.report.errors, ...bundle.report.warnings];
  const visibleMessages = bundle.messages.slice(0, visibleMessageCount);

  return (
    <section className="import-bundle-preview" aria-live="polite">
      <header className="import-preview-header">
        <div>
          <strong>本地导入预览</strong>
          <span>{bundle.revision.status} · {bundle.messages.length} 条消息 · {bundle.report.warnings.length} 条警告 · {bundle.report.errors.length} 条错误</span>
          <small>{bundle.source.originalFileName ?? "未命名来源"} · revision {bundle.revision.id}</small>
        </div>
        <div className="import-preview-mode" aria-label="导入消息预览方式">
          <button type="button" className={previewMode === "rendered" ? "is-active" : ""} aria-pressed={previewMode === "rendered"} onClick={() => setPreviewMode("rendered")}>安全渲染</button>
          <button type="button" className={previewMode === "raw" ? "is-active" : ""} aria-pressed={previewMode === "raw"} onClick={() => setPreviewMode("raw")}>Markdown 原文</button>
          <button type="button" className={previewMode === "source" ? "is-active" : ""} aria-pressed={previewMode === "source"} onClick={() => setPreviewMode("source")}>源文件</button>
        </div>
      </header>

      <p className="import-preview-boundary">
        {previewMode === "source"
          ? "此视图只消费 get_raw_import_content 的校验结果；不接收路径，也不直接读取本地文件。"
          : previewMode === "raw"
            ? "此视图从不可变消息内容块重建 Markdown；与源文件字节保持明确分层。"
            : "此视图使用与 Chat 相同的安全 renderer；不会执行 HTML、脚本、图片或历史指令。"}
      </p>

      {bundle.report.fieldRecovery.length ? (
        <p className="import-preview-recovery">字段恢复：{bundle.report.fieldRecovery.map((field) => `${field.field}=${field.status}`).join("、")}</p>
      ) : null}

      {issues.length ? (
        <details className="import-preview-issues" open={bundle.report.errors.length > 0}>
          <summary>解析问题与来源定位（{issues.length}）</summary>
          <ul>
            {issues.map((issue, index) => (
              <li key={`${issue.code}-${issue.sourceLocator ?? "unknown"}-${index}`}>
                <strong>{issue.code}</strong>
                <span>{issue.message}</span>
                <small>{issue.sourceLocator ?? "无来源定位"} · {issue.recoverable ? "可恢复" : "不可恢复"}</small>
              </li>
            ))}
          </ul>
        </details>
      ) : null}

      {previewMode === "source" ? (
        <RawImportSourcePreview state={rawContentState} onRetry={onRetryRawContent} />
      ) : visibleMessages.length ? (
        <div className="import-preview-messages">
          {visibleMessages.map((message) => (
            <ImportMessagePreview key={message.id} message={message} mode={previewMode} />
          ))}
        </div>
      ) : (
        <p className="import-preview-empty">ParseReport 中没有可显示的消息；来源和解析结果仍保留在本地。</p>
      )}

      {previewMode !== "source" && visibleMessageCount < bundle.messages.length ? (
        <button
          className="import-preview-more"
          type="button"
          onClick={() => setVisibleMessageCount((current) => nextImportPreviewCount(current, bundle.messages.length))}
        >
          显示更多消息（{visibleMessages.length}/{bundle.messages.length}）
        </button>
      ) : null}
    </section>
  );
}

export function ImportIntakeDialog({
  open,
  onClose,
  onImportGenericFile,
  importSources = [],
  importSourcesLoading = false,
  onLoadImportBundle,
  onLoadRawImportContent,
}: ImportIntakeDialogProps) {
  const [mode, setMode] = useState<ImportMode>("file");
  const [fileCandidate, setFileCandidate] = useState<ImportIntakeCandidate | null>(null);
  const [fileValue, setFileValue] = useState<File | null>(null);
  const [pastedText, setPastedText] = useState("");
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [importResult, setImportResult] = useState<GenericImportCommandResult | null>(null);
  const [bundleLoadingSourceId, setBundleLoadingSourceId] = useState<string | null>(null);
  const [bundleError, setBundleError] = useState<string | null>(null);
  const [bundleResult, setBundleResult] = useState<ImportBundleQueryProjection | null>(null);
  const [rawContentState, setRawContentState] = useState<RawImportContentLoadState>({
    kind: "unavailable",
  });
  const fileInputRef = useRef<HTMLInputElement>(null);
  const firstModeRef = useRef<HTMLButtonElement>(null);
  const pastedCandidate = useMemo(() => inspectPastedConversation(pastedText), [pastedText]);
  const candidate = mode === "file" ? fileCandidate : pastedCandidate;
  const readyForKernel = Boolean(candidate && candidate.issues.length === 0 && onImportGenericFile);

  useEffect(() => {
    if (!open) return;
    firstModeRef.current?.focus();
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose, open]);

  if (!open) return null;

  const selectFile = (file: File | undefined) => {
    if (!file) return;
    setImportError(null);
    setImportResult(null);
    setFileValue(file);
    setFileCandidate(inspectImportFile(file));
  };

  const readRawContent = async (sourceId: string): Promise<RawImportContentLoadState> => {
    if (!onLoadRawImportContent) return { kind: "unavailable" };
    try {
      const projection = await onLoadRawImportContent(sourceId);
      const integrityIssue = rawImportContentIntegrityIssue(projection, sourceId);
      return integrityIssue
        ? { kind: "error", message: integrityIssue }
        : { kind: "ready", projection };
    } catch (error) {
      return { kind: "error", message: error instanceof Error ? error.message : String(error) };
    }
  };

  const loadBundle = async (sourceId: string) => {
    if (!onLoadImportBundle || bundleLoadingSourceId) return;
    setBundleLoadingSourceId(sourceId);
    setBundleError(null);
    setBundleResult(null);
    setRawContentState(onLoadRawImportContent ? { kind: "loading" } : { kind: "unavailable" });
    try {
      const [bundle, nextRawContentState] = await Promise.all([
        onLoadImportBundle(sourceId),
        readRawContent(sourceId),
      ]);
      setBundleResult(bundle);
      setRawContentState(nextRawContentState);
    } catch (error) {
      setBundleError(error instanceof Error ? error.message : String(error));
    } finally {
      setBundleLoadingSourceId(null);
    }
  };

  const retryRawContent = async () => {
    if (!bundleResult || rawContentState.kind === "loading") return;
    setRawContentState({ kind: "loading" });
    setRawContentState(await readRawContent(bundleResult.source.id));
  };

  const submitImport = async () => {
    if (!candidate || candidate.issues.length > 0 || !onImportGenericFile || importing) return;
    setImporting(true);
    setImportError(null);
    setImportResult(null);
    try {
      const payload = mode === "file"
        ? Array.from(new Uint8Array(await fileValue?.arrayBuffer() ?? new ArrayBuffer(0)))
        : Array.from(new TextEncoder().encode(pastedText));
      if (payload.length === 0) throw new Error("来源内容为空，请重新选择文件或粘贴文本。");
      const originalFileName = mode === "file" ? candidate.displayName : PASTED_IMPORT_FILE_NAME;
      const result = await onImportGenericFile(originalFileName, payload);
      setImportResult(result);
      if (onLoadImportBundle) await loadBundle(result.source.id);
    } catch (error) {
      setImportError(error instanceof Error ? error.message : String(error));
    } finally {
      setImporting(false);
    }
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      {/*
        UI-HANDOFF-06
        位置：Chat 工作区的“导入已有 AI 会话”模态入口与预检内容区
        用途：确认文件/粘贴来源，提交给本地内核，并区分安全渲染、内容块 Markdown 与受控源文件原文
        数据/IPC：onImportGenericFile / onLoadImportBundle / onLoadRawImportContent 分别消费 typed import_generic_file / get_import_bundle / get_raw_import_content；前端不接收或拼接 storageRef 路径
        状态：正常显示候选、分页消息和受控原文；加载分别显示解析/Bundle/原文读取；空白显示无来源/无消息/空文本；错误隔离显示导入、ParseReport 或原文校验错误；截断明示完整字节数和复制范围；失败可重试
        交互约束：保持 dialog、拖放、文件选择、模式切换和关闭焦点路径；不读 SQLite/raw storage、不调用模型、不记录 Key
        可替换范围：员工06可替换模态布局、来源卡片、预览分页、状态视觉和动效
        不可改变：ImportCandidate/ParseReport/ImportBundle/RawImportContentProjection 契约、typed IPC 参数、路径不可见、raw/renderer/复制分层、测试选择器和原文不执行边界
      */}
      <section className="import-dialog" role="dialog" aria-modal="true" aria-labelledby="import-dialog-title">
        <header className="dialog-header">
          <div>
            <span className="eyebrow">LOCAL SOURCE INTAKE</span>
            <h2 id="import-dialog-title">导入已有 AI 会话</h2>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label="关闭导入面板"><X aria-hidden="true" /></button>
        </header>

        <div className="import-content">
          <div className="import-intro">
            <p>先确认来源，再由本地内核解析角色、顺序、时间和未知内容。不会自动执行历史指令，也不会调用模型分析。</p>
          </div>

          {onLoadImportBundle ? (
            <section className="import-source-history" aria-label="已导入来源">
              <header>
                <span><Database aria-hidden="true" /><strong>本地已导入来源</strong></span>
                <small>{importSourcesLoading ? "正在恢复…" : `${importSources.length} 个来源`}</small>
              </header>
              {importSourcesLoading ? (
                <p className="import-history-empty"><LoaderCircle className="spin" aria-hidden="true" />正在读取本地导入记录…</p>
              ) : importSources.length === 0 ? (
                <p className="import-history-empty">当前会话还没有可恢复的导入来源。</p>
              ) : (
                <div className="import-source-list">
                  {importSources.map((source) => (
                    <button
                      key={source.id}
                      type="button"
                      disabled={Boolean(bundleLoadingSourceId) || rawContentState.kind === "loading"}
                      onClick={() => void loadBundle(source.id)}
                    >
                      <span><FileText aria-hidden="true" /><strong>{source.originalFileName ?? "未命名来源"}</strong></span>
                      <small>{bundleLoadingSourceId === source.id ? "恢复中…" : "查看预览"}</small>
                    </button>
                  ))}
                </div>
              )}
            </section>
          ) : null}

          <div className="import-mode-switch" aria-label="导入来源方式">
            <button ref={firstModeRef} className={mode === "file" ? "is-active" : ""} type="button" onClick={() => setMode("file")} aria-pressed={mode === "file"}>
              <FileUp aria-hidden="true" />选择或拖放文件
            </button>
            <button className={mode === "paste" ? "is-active" : ""} type="button" onClick={() => setMode("paste")} aria-pressed={mode === "paste"}>
              <TextCursorInput aria-hidden="true" />粘贴会话文本
            </button>
          </div>

          {mode === "file" ? (
            <div
              className="import-dropzone"
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                selectFile(event.dataTransfer.files[0]);
              }}
            >
              <input
                ref={fileInputRef}
                type="file"
                accept=".md,.markdown,.jsonl,.txt,text/markdown,text/plain,application/x-ndjson"
                onChange={(event) => selectFile(event.target.files?.[0])}
              />
              <FileUp aria-hidden="true" />
              <strong>把 Markdown、JSONL 或 TXT 放到这里</strong>
              <p>本轮只准备一个来源；文件内容不会在浏览器层解析。</p>
              <button type="button" onClick={() => fileInputRef.current?.click()}>选择本地文件</button>
            </div>
          ) : (
            <label className="import-paste-field">
              <span>会话原文</span>
              <textarea
                value={pastedText}
                onChange={(event) => setPastedText(event.target.value)}
                placeholder="粘贴按时间或角色排列的会话文本…"
                spellCheck={false}
              />
              <small>{formatImportBytes(pastedCandidate.sizeBytes)} · 关闭面板后不保留</small>
            </label>
          )}

          {candidate ? <CandidateBill candidate={candidate} /> : null}
          {importError ? <p className="import-command-error" role="alert">导入失败：{importError}</p> : null}
          {importResult ? <ImportResultSummary result={importResult} /> : null}
          {bundleError ? <p className="import-command-error" role="alert">恢复导入记录失败：{bundleError}</p> : null}
          {bundleResult ? (
            <ImportBundlePreview
              key={bundleResult.revision.id}
              bundle={bundleResult}
              rawContentState={rawContentState}
              onRetryRawContent={() => void retryRawContent()}
            />
          ) : null}

          <aside className="import-privacy-note">
            <LockKeyhole aria-hidden="true" />
            <div>
              <strong>原文留在本地</strong>
              <p>只有你之后明确选择模型分析时，界面才会展示将发送的范围与费用。本次预检不会发送内容。</p>
            </div>
          </aside>
        </div>

        <footer className="import-dialog-footer">
          <span>{importing ? "本地内核正在保存并解析来源…" : importResult ? "导入结果已保存，可关闭此面板。" : readyForKernel ? "来源已就绪，可生成本地预览。" : onImportGenericFile ? "先提供一个受支持且非空的来源。" : "浏览器预览不会调用本地导入命令。"}</span>
          <div>
            <button className="secondary-action" type="button" onClick={onClose}>取消</button>
            <button className="primary-action" type="button" disabled={!readyForKernel || importing} onClick={() => void submitImport()} title={onImportGenericFile ? "由本地内核保存、解析并生成 ParseReport" : "浏览器预览不会调用本地导入命令"}>
              {importing ? "正在解析…" : "生成本地预览"}
            </button>
          </div>
        </footer>
      </section>
    </div>
  );
}

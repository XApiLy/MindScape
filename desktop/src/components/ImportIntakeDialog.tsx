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
  SearchCheck,
  ShieldCheck,
  Sparkles,
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
import { commandErrorMessage } from "../app/commandErrorPresentation";
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
  ImportKnowledgeEntityProposal,
  ImportKnowledgeProposalBatchProjection,
  ImportKnowledgeProposalDiscoveryProjection,
  ImportKnowledgeProposalDiscoveryQuery,
  ImportKnowledgeProposalRequestInput,
  ImportKnowledgeProposalReviewCommandInput,
  ImportKnowledgeProposalReviewProjection,
  ImportedMessage,
  ImportSource,
  KnowledgeEntityKind,
  RawImportContentProjection,
} from "../domain";
import {
  buildImportKnowledgeProposalRequest,
  buildImportKnowledgeProposalReview,
} from "../app/importKnowledgeProposal";
import type { ImportKnowledgeProposalTargetOption } from "../app/importKnowledgeProposalTargets";
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

const KNOWLEDGE_KIND_LABEL: Record<KnowledgeEntityKind, string> = {
  goal: "目标",
  decision: "决策",
  constraint: "约束",
  question: "问题",
  source: "来源",
  project: "项目",
  topic: "主题",
};

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
  proposalTargetOptions?: readonly ImportKnowledgeProposalTargetOption[];
  onRequestImportKnowledgeProposals?: (
    input: ImportKnowledgeProposalRequestInput,
  ) => Promise<ImportKnowledgeProposalBatchProjection>;
  onDiscoverImportKnowledgeProposals?: (
    query: ImportKnowledgeProposalDiscoveryQuery,
  ) => Promise<ImportKnowledgeProposalDiscoveryProjection>;
  onReviewImportKnowledgeProposal?: (
    input: ImportKnowledgeProposalReviewCommandInput,
  ) => Promise<ImportKnowledgeProposalReviewProjection>;
  onListImportKnowledgeProposalReviews?: (
    requestId: string,
  ) => Promise<ImportKnowledgeProposalReviewProjection[]>;
};

type ImportProposalRequestState =
  | { kind: "idle" }
  | { kind: "pending"; input: ImportKnowledgeProposalRequestInput }
  | { kind: "ready"; input: ImportKnowledgeProposalRequestInput; batch: ImportKnowledgeProposalBatchProjection }
  | { kind: "error"; input: ImportKnowledgeProposalRequestInput; message: string };

type ImportProposalReviewState =
  | { kind: "idle" }
  | { kind: "pending"; input: ImportKnowledgeProposalReviewCommandInput }
  | { kind: "success"; projection: ImportKnowledgeProposalReviewProjection }
  | { kind: "error"; input: ImportKnowledgeProposalReviewCommandInput; message: string };

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
  selected,
  onSelectedChange,
}: {
  message: ImportedMessage;
  mode: MessagePreviewMode;
  selected: boolean;
  onSelectedChange: (selected: boolean) => void;
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
          <input
            type="checkbox"
            checked={selected}
            onChange={(event) => onSelectedChange(event.target.checked)}
            aria-label={`选择来源消息：${message.sourceLocator}`}
          />
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

function ImportKnowledgeProposalReviewCard({
  proposal,
  existingReview,
  onReview,
  onCompleted,
}: {
  proposal: ImportKnowledgeEntityProposal;
  existingReview: ImportKnowledgeProposalReviewProjection | null;
  onReview?: (input: ImportKnowledgeProposalReviewCommandInput) => Promise<ImportKnowledgeProposalReviewProjection>;
  onCompleted: (projection: ImportKnowledgeProposalReviewProjection) => void;
}) {
  const [kind, setKind] = useState<KnowledgeEntityKind>(proposal.suggestedKind);
  const [name, setName] = useState(proposal.suggestedName);
  const [aliases, setAliases] = useState(proposal.suggestedAliases.join("，"));
  const [rejectReason, setRejectReason] = useState("");
  const [reviewState, setReviewState] = useState<ImportProposalReviewState>({ kind: "idle" });
  const finalReview = reviewState.kind === "success" ? reviewState.projection : existingReview;
  const busy = reviewState.kind === "pending";

  const review = async (
    action: "confirm" | "reject",
    retryInput?: ImportKnowledgeProposalReviewCommandInput,
  ) => {
    if (!onReview || busy || finalReview) return;
    const input = retryInput ?? buildImportKnowledgeProposalReview(
      proposal,
      action === "confirm"
        ? { action: "confirm", kind, name, aliases: aliases.split(/[，,]/) }
        : { action: "reject", reason: rejectReason },
      `import-knowledge-decision-${crypto.randomUUID()}`,
      new Date().toISOString(),
    );
    if (input.choice.action === "confirm" && !input.choice.name) return;

    setReviewState({ kind: "pending", input });
    try {
      const projection = await onReview(input);
      setReviewState({ kind: "success", projection });
      onCompleted(projection);
    } catch (error) {
      setReviewState({
        kind: "error",
        input,
        message: commandErrorMessage(error),
      });
    }
  };

  return (
    <article className={`import-proposal-card${finalReview ? " is-reviewed" : ""}`}>
      <header>
        <span><SearchCheck aria-hidden="true" /><strong>知识建议</strong></span>
        <small>revision {proposal.proposalRevision} · {proposal.evidence.length} 条来源</small>
      </header>
      <div className="import-proposal-fields">
        <label>
          <span>类型</span>
          <select value={kind} disabled={Boolean(finalReview) || busy} onChange={(event) => setKind(event.target.value as KnowledgeEntityKind)}>
            {(Object.entries(KNOWLEDGE_KIND_LABEL) as Array<[KnowledgeEntityKind, string]>).map(([value, label]) => (
              <option key={value} value={value}>{label}</option>
            ))}
          </select>
        </label>
        <label className="is-wide">
          <span>名称</span>
          <input value={name} disabled={Boolean(finalReview) || busy} onChange={(event) => setName(event.target.value)} />
        </label>
        <label className="is-wide">
          <span>别名</span>
          <input value={aliases} disabled={Boolean(finalReview) || busy} onChange={(event) => setAliases(event.target.value)} placeholder="使用逗号分隔" />
        </label>
      </div>
      <details className="import-proposal-evidence">
        <summary>查看不可修改的来源证据</summary>
        <ul>
          {proposal.evidence.map((evidence) => (
            <li key={evidence.id}>
              <small>{evidence.target.type === "importContent" ? evidence.target.locator : evidence.target.type}</small>
              <span>{evidence.excerpt ?? "该 EvidenceRef 没有可显示摘要。"}</span>
            </li>
          ))}
        </ul>
      </details>
      {finalReview ? (
        <p className="import-proposal-reviewed" role="status">
          <ShieldCheck aria-hidden="true" />
          {finalReview.action === "confirm"
            ? finalReview.entityStatus === "candidate" ? "已创建分支候选知识" : "已确认到会话知识"
            : "已否决此建议"}
        </p>
      ) : (
        <>
          <label className="import-proposal-reject-reason">
            <span>否决原因（可选）</span>
            <input value={rejectReason} disabled={busy} onChange={(event) => setRejectReason(event.target.value)} placeholder="例如：内容过时或不构成可复用知识" />
          </label>
          <div className="import-proposal-actions">
            <button type="button" disabled={!onReview || busy || !name.trim()} onClick={() => void review("confirm")}>
              {busy && reviewState.input.choice.action === "confirm" ? <LoaderCircle className="spin" aria-hidden="true" /> : <Check aria-hidden="true" />}
              确认并创建知识
            </button>
            <button className="is-reject" type="button" disabled={!onReview || busy} onClick={() => void review("reject")}>否决建议</button>
          </div>
        </>
      )}
      {reviewState.kind === "error" ? (
        <div className="import-proposal-error" role="alert">
          <span>审核失败：{reviewState.message}</span>
          <button type="button" onClick={() => void review(reviewState.input.choice.action, reviewState.input)}>使用同一决定重试</button>
        </div>
      ) : null}
    </article>
  );
}

function ImportKnowledgeProposalPanel({
  bundle,
  selectedMessageIds,
  targetOptions,
  onRequest,
  onDiscover,
  onReview,
  onListReviews,
}: {
  bundle: ImportBundleQueryProjection;
  selectedMessageIds: readonly string[];
  targetOptions: readonly ImportKnowledgeProposalTargetOption[];
  onRequest?: (input: ImportKnowledgeProposalRequestInput) => Promise<ImportKnowledgeProposalBatchProjection>;
  onDiscover?: (query: ImportKnowledgeProposalDiscoveryQuery) => Promise<ImportKnowledgeProposalDiscoveryProjection>;
  onReview?: (input: ImportKnowledgeProposalReviewCommandInput) => Promise<ImportKnowledgeProposalReviewProjection>;
  onListReviews?: (requestId: string) => Promise<ImportKnowledgeProposalReviewProjection[]>;
}) {
  const [targetId, setTargetId] = useState(targetOptions[0]?.id ?? "");
  const [requestState, setRequestState] = useState<ImportProposalRequestState>({ kind: "idle" });
  const [reviews, setReviews] = useState<ImportKnowledgeProposalReviewProjection[]>([]);
  const [discovery, setDiscovery] = useState<ImportKnowledgeProposalDiscoveryProjection | null>(null);
  const [discoveryState, setDiscoveryState] = useState<"idle" | "loading" | "error">("idle");
  const [reviewHistoryError, setReviewHistoryError] = useState<string | null>(null);
  const target = targetOptions.find((option) => option.id === targetId) ?? targetOptions[0];
  const busy = requestState.kind === "pending";
  const currentSelectionKey = [...selectedMessageIds].sort().join("\u0000");
  const persistedSelectionKey = requestState.kind === "ready"
    ? requestState.input.selectedMessageIds.join("\u0000")
    : null;
  const targetChanged = requestState.kind === "ready"
    ? JSON.stringify(requestState.input.targetScope) !== JSON.stringify(target?.scope)
    : false;
  const selectionChanged = requestState.kind === "ready"
    ? persistedSelectionKey !== currentSelectionKey || targetChanged
    : true;

  const loadReviews = async (requestId: string) => {
    if (!onListReviews) return;
    setReviewHistoryError(null);
    try {
      setReviews(await onListReviews(requestId));
    } catch (error) {
      setReviewHistoryError(commandErrorMessage(error));
    }
  };

  useEffect(() => {
    if (!onDiscover) return;
    let cancelled = false;
    setDiscoveryState("loading");
    void onDiscover({
      importSourceId: bundle.source.id,
      importRevisionId: bundle.revision.id,
      limit: 8,
    }).then((result) => {
      if (cancelled) return;
      setDiscovery(result);
      setDiscoveryState("idle");
    }).catch(() => {
      if (!cancelled) setDiscoveryState("error");
    });
    return () => { cancelled = true; };
  }, [bundle.revision.id, bundle.source.id, onDiscover]);

  const requestProposals = async (retryInput?: ImportKnowledgeProposalRequestInput) => {
    if (
      !onRequest
      || busy
      || (!retryInput && (!target || selectedMessageIds.length === 0))
    ) return;
    const input = retryInput ?? buildImportKnowledgeProposalRequest(
      bundle,
      selectedMessageIds,
      target!.scope,
      `import-knowledge-request-${crypto.randomUUID()}`,
      new Date().toISOString(),
    );
    setRequestState({ kind: "pending", input });
    try {
      const batch = await onRequest(input);
      setRequestState({ kind: "ready", input, batch });
      setReviews([]);
      void loadReviews(batch.requestId);
    } catch (error) {
      setRequestState({
        kind: "error",
        input,
        message: commandErrorMessage(error),
      });
    }
  };

  const batch = requestState.kind === "ready" ? requestState.batch : null;

  return (
    <section className="import-proposal-panel" aria-label="从导入来源生成知识建议">
      <header>
        <span><Sparkles aria-hidden="true" /><strong>从来源提炼知识</strong></span>
        <small>只有点击后才会启动分析</small>
      </header>
      <p>勾选下方消息并选择知识落点。内核会固定原文来源；建议审核前不会进入知识库、检索或分支回流。</p>
      <div className="import-proposal-request-controls">
        <label>
          <span>知识落点</span>
          <select value={target?.id ?? ""} disabled={busy || targetOptions.length === 0} onChange={(event) => setTargetId(event.target.value)}>
            {targetOptions.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
          </select>
        </label>
        <button
          type="button"
          disabled={!onRequest || !target || busy || selectedMessageIds.length === 0 || (requestState.kind === "ready" && !selectionChanged)}
          onClick={() => void requestProposals()}
        >
          {busy ? <LoaderCircle className="spin" aria-hidden="true" /> : <Sparkles aria-hidden="true" />}
          {busy
            ? "正在生成建议…"
            : requestState.kind === "ready"
              ? selectionChanged ? "按当前选择重新生成" : "建议已生成"
              : "生成知识建议"}
        </button>
      </div>
      <p className="import-proposal-selection-count">已选择 {selectedMessageIds.length} / {bundle.messages.length} 条来源消息</p>
      {!onRequest ? <p className="import-proposal-unavailable">本地内核尚未提供提案命令；当前只展示正式交互边界。</p> : null}
      {discoveryState === "loading" ? <p className="import-proposal-history" role="status">正在恢复此来源的提案记录…</p> : null}
      {discoveryState === "error" ? <p className="import-proposal-history-error" role="alert">提案记录恢复失败；仍可使用当前来源重新生成。</p> : null}
      {discoveryState === "idle" && discovery && discovery.items.length > 0 ? (
        <details className="import-proposal-history">
          <summary>已恢复 {discovery.items.length} 次提案请求</summary>
          <ul>
            {discovery.items.map((item) => (
              <li key={item.request.requestId}>
                <span>{item.state === "completed" ? "已生成" : "等待生成"} · {item.proposalCount} 条建议 · {item.reviewedCount} 条已审核</span>
                {item.state === "completed" && item.batch ? (
                  <button type="button" onClick={() => {
                    setRequestState({ kind: "ready", input: item.request, batch: item.batch! });
                    setReviews([]);
                    void loadReviews(item.request.requestId);
                  }}>恢复查看</button>
                ) : <button type="button" disabled>等待重试</button>}
              </li>
            ))}
          </ul>
        </details>
      ) : null}
      {requestState.kind === "error" ? (
        <div className="import-proposal-error" role="alert">
          <span>知识建议生成失败：{requestState.message}</span>
          <button type="button" onClick={() => void requestProposals(requestState.input)}>使用同一请求重试</button>
        </div>
      ) : null}
      {batch ? batch.proposals.length ? (
        <>
          {!onReview ? <p className="import-proposal-unavailable">建议已返回，但本地内核尚未提供审核命令。</p> : null}
          <div className="import-proposal-list">
            {batch.proposals.map((proposal) => (
              <ImportKnowledgeProposalReviewCard
                key={proposal.proposalId}
                proposal={proposal}
                existingReview={reviews.find((review) => review.proposalId === proposal.proposalId) ?? null}
                onReview={onReview}
                onCompleted={(projection) => setReviews((current) => [
                  projection,
                  ...current.filter((review) => review.proposalId !== projection.proposalId),
                ])}
              />
            ))}
          </div>
        </>
      ) : (
        <p className="import-proposal-unavailable">本次分析没有返回可审核建议；原文和选择保持不变。</p>
      ) : null}
      {reviewHistoryError ? <p className="import-proposal-history-error">审核记录恢复失败：{reviewHistoryError}</p> : null}
    </section>
  );
}

function ImportBundlePreview({
  bundle,
  rawContentState,
  onRetryRawContent,
  proposalTargetOptions,
  onRequestImportKnowledgeProposals,
  onDiscoverImportKnowledgeProposals,
  onReviewImportKnowledgeProposal,
  onListImportKnowledgeProposalReviews,
}: {
  bundle: ImportBundleQueryProjection;
  rawContentState: RawImportContentLoadState;
  onRetryRawContent: () => void;
  proposalTargetOptions: readonly ImportKnowledgeProposalTargetOption[];
  onRequestImportKnowledgeProposals?: (
    input: ImportKnowledgeProposalRequestInput,
  ) => Promise<ImportKnowledgeProposalBatchProjection>;
  onDiscoverImportKnowledgeProposals?: (
    query: ImportKnowledgeProposalDiscoveryQuery,
  ) => Promise<ImportKnowledgeProposalDiscoveryProjection>;
  onReviewImportKnowledgeProposal?: (
    input: ImportKnowledgeProposalReviewCommandInput,
  ) => Promise<ImportKnowledgeProposalReviewProjection>;
  onListImportKnowledgeProposalReviews?: (
    requestId: string,
  ) => Promise<ImportKnowledgeProposalReviewProjection[]>;
}) {
  const [previewMode, setPreviewMode] = useState<ImportPreviewMode>("rendered");
  const [visibleMessageCount, setVisibleMessageCount] = useState(IMPORT_PREVIEW_PAGE_SIZE);
  const [selectedMessageIds, setSelectedMessageIds] = useState<string[]>([]);
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

      {previewMode !== "source" ? (
        <ImportKnowledgeProposalPanel
          bundle={bundle}
          selectedMessageIds={selectedMessageIds}
          targetOptions={proposalTargetOptions}
          onRequest={onRequestImportKnowledgeProposals}
          onDiscover={onDiscoverImportKnowledgeProposals}
          onReview={onReviewImportKnowledgeProposal}
          onListReviews={onListImportKnowledgeProposalReviews}
        />
      ) : null}

      {previewMode === "source" ? (
        <p className="import-proposal-step-note">
          下一步：先切换到“安全渲染”或“Markdown 原文”，勾选下方消息后，才能生成知识建议。
        </p>
      ) : null}

      {previewMode === "source" ? (
        <RawImportSourcePreview state={rawContentState} onRetry={onRetryRawContent} />
      ) : visibleMessages.length ? (
        <div className="import-preview-messages">
          {visibleMessages.map((message) => (
            <ImportMessagePreview
              key={message.id}
              message={message}
              mode={previewMode}
              selected={selectedMessageIds.includes(message.id)}
              onSelectedChange={(selected) => setSelectedMessageIds((current) => selected
                ? [...current, message.id]
                : current.filter((messageId) => messageId !== message.id))}
            />
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
  proposalTargetOptions = [],
  onRequestImportKnowledgeProposals,
  onDiscoverImportKnowledgeProposals,
  onReviewImportKnowledgeProposal,
  onListImportKnowledgeProposalReviews,
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
      return { kind: "error", message: commandErrorMessage(error) };
    }
  };

  const loadBundle = async (sourceId: string) => {
    if (!onLoadImportBundle || bundleLoadingSourceId) return;
    setBundleLoadingSourceId(sourceId);
    setBundleError(null);
    setRawContentState(onLoadRawImportContent ? { kind: "loading" } : { kind: "unavailable" });
    try {
      const [bundle, nextRawContentState] = await Promise.all([
        onLoadImportBundle(sourceId),
        readRawContent(sourceId),
      ]);
      setBundleResult(bundle);
      setRawContentState(nextRawContentState);
    } catch (error) {
      setBundleError(commandErrorMessage(error));
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
      setImportError(commandErrorMessage(error));
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
        数据/IPC：除 import/get bundle/raw 外，显式消费 request/get/review/list import knowledge proposal typed IPC；前端不接收或拼接 storageRef、EvidenceRef 或 entity ID
        状态：覆盖来源选择、建议 pending/empty/error/success、审核 Confirm/Reject、同 requestId/decisionId 重试和记录恢复；失败状态彼此隔离
        交互约束：导入不自动分析；只有用户勾选消息并点击才请求建议；target scope 只来自权威 conversation/Active FocusFrame 投影；不读 SQLite/raw storage、不记录 Key
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
              proposalTargetOptions={proposalTargetOptions}
              onRequestImportKnowledgeProposals={onRequestImportKnowledgeProposals}
              onDiscoverImportKnowledgeProposals={onDiscoverImportKnowledgeProposals}
              onReviewImportKnowledgeProposal={onReviewImportKnowledgeProposal}
              onListImportKnowledgeProposalReviews={onListImportKnowledgeProposalReviews}
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

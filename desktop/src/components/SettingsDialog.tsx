import { useEffect, useRef, useState } from "react";
import {
  BookOpen,
  CheckCircle2,
  Download,
  KeyRound,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
  Server,
  ShieldCheck,
  Trash2,
  Wifi,
  X,
} from "lucide-react";
import { describeModelCapabilities, hasUsableCredential } from "../app/providerCatalog";
import {
  DEFAULT_READING_PREFERENCES,
  READING_PREFERENCE_LIMITS,
  READING_PRESET_VALUES,
  type ReadingPreferences,
} from "../app/readingPreferences";
import {
  BUILT_IN_READING_FONT_PRESETS,
  loadBuiltInReadingFont,
  type BuiltInReadingFontId,
} from "../app/readingFonts";
import type {
  ModelSelection,
  ProviderConnectionTestResult,
  ProviderDescriptor,
  SemanticModelPackStatus,
} from "../domain";
import { SafeMarkdown } from "./SafeMarkdown";

const READING_FONT_OPTIONS = [
  { value: "sans", label: "系统无衬线" },
  { value: "serif", label: "系统衬线" },
  { value: "accessible", label: "高可读" },
] as const;

const READING_FONT_GROUPS = [
  { id: "clear", label: "清晰正文" },
  { id: "personal", label: "轻松与个性阅读" },
  { id: "atmosphere", label: "标题与氛围" },
] as const;

const READING_SIZE_OPTIONS = [
  { value: "small", label: "小", size: READING_PRESET_VALUES.fontSizePx.small },
  { value: "standard", label: "标准", size: READING_PRESET_VALUES.fontSizePx.standard },
  { value: "large", label: "大", size: READING_PRESET_VALUES.fontSizePx.large },
  { value: "xlarge", label: "特大", size: READING_PRESET_VALUES.fontSizePx.xlarge },
] as const;

const READING_LINE_HEIGHT_OPTIONS = [
  { value: "compact", label: "紧凑", lineHeight: READING_PRESET_VALUES.lineHeightValue.compact },
  { value: "comfortable", label: "舒适", lineHeight: READING_PRESET_VALUES.lineHeightValue.comfortable },
  { value: "spacious", label: "宽松", lineHeight: READING_PRESET_VALUES.lineHeightValue.spacious },
] as const;

const READING_WIDTH_OPTIONS = [
  { value: "standard", label: "标准", width: READING_PRESET_VALUES.readingWidthCh.standard },
  { value: "wide", label: "宽屏", width: READING_PRESET_VALUES.readingWidthCh.wide },
] as const;

const READING_PREVIEW_MARKDOWN = `## 阅读预览 · Reading 2026

清晰的层级让**长文**更容易回看，也不会改变 \`raw_markdown\`。

### 小标题与列表节奏

- 标题、正文与列表共享同一段落间距基准
- 源码空行不会额外变成视觉空白行

---

> 字体服务阅读，原文始终保持不变。`;

type SettingsDialogProps = {
  open: boolean;
  loading: boolean;
  error: string | null;
  providers: ProviderDescriptor[];
  credentialStatus: Record<string, boolean>;
  selectedModel: ModelSelection | null;
  readingPreferences: ReadingPreferences;
  readingPreferencesSessionOnly: boolean;
  onClose: () => void;
  onRefresh: () => Promise<void>;
  onSelectModel: (selection: ModelSelection) => void;
  onSaveCredential: (providerId: string, secret: string) => Promise<void>;
  onDeleteCredential: (providerId: string) => Promise<void>;
  onTestConnection: (providerId: string) => Promise<ProviderConnectionTestResult>;
  onGetSemanticModelStatus: () => Promise<SemanticModelPackStatus>;
  onInstallSemanticModel: () => Promise<SemanticModelPackStatus>;
  onReadingPreferencesChange: (
    preferences: ReadingPreferences,
    scope: "workspace" | "session",
  ) => void;
};

export function SettingsDialog({
  open,
  loading,
  error,
  providers,
  credentialStatus,
  selectedModel,
  readingPreferences,
  readingPreferencesSessionOnly,
  onClose,
  onRefresh,
  onSelectModel,
  onSaveCredential,
  onDeleteCredential,
  onTestConnection,
  onGetSemanticModelStatus,
  onInstallSemanticModel,
  onReadingPreferencesChange,
}: SettingsDialogProps) {
  const credentialInputs = useRef<Record<string, HTMLInputElement | null>>({});
  const [busyProviderId, setBusyProviderId] = useState<string | null>(null);
  const [operationMessage, setOperationMessage] = useState<string | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [connectionResults, setConnectionResults] = useState<Record<string, ProviderConnectionTestResult>>({});
  const [fontAvailability, setFontAvailability] = useState<Partial<Record<
    BuiltInReadingFontId,
    "checking" | "available" | "fallback"
  >>>({});
  const [semanticStatus, setSemanticStatus] = useState<SemanticModelPackStatus | null>(null);
  const [semanticBusy, setSemanticBusy] = useState(false);
  const [semanticError, setSemanticError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose, open]);

  useEffect(() => {
    if (!open) return;
    let active = true;
    const checking = Object.fromEntries(
      BUILT_IN_READING_FONT_PRESETS.map(({ id }) => [id, "checking"]),
    ) as Record<BuiltInReadingFontId, "checking">;
    setFontAvailability(checking);

    if (!document.fonts) {
      setFontAvailability(Object.fromEntries(
        BUILT_IN_READING_FONT_PRESETS.map(({ id }) => [id, "fallback"]),
      ));
      return () => { active = false; };
    }

    void Promise.all(BUILT_IN_READING_FONT_PRESETS.map(async ({ id }) => [
      id,
      await loadBuiltInReadingFont(id, document.fonts),
    ] as const)).then((entries) => {
      if (active) setFontAvailability(Object.fromEntries(entries));
    });

    return () => { active = false; };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    let active = true;
    void onGetSemanticModelStatus().then(
      (status) => { if (active) setSemanticStatus(status); },
      (statusError) => {
        if (active) setSemanticError(statusError instanceof Error ? statusError.message : String(statusError));
      },
    );
    return () => { active = false; };
  }, [onGetSemanticModelStatus, open]);

  if (!open) return null;

  const saveCredential = async (provider: ProviderDescriptor) => {
    const input = credentialInputs.current[provider.id];
    const secret = input?.value.trim() ?? "";
    if (!secret) {
      setOperationError("请输入新的 API Key。现有 Key 不会回显。");
      return;
    }
    setBusyProviderId(provider.id);
    setOperationError(null);
    setOperationMessage(null);
    try {
      await onSaveCredential(provider.id, secret);
      if (input) input.value = "";
      setOperationMessage(`${provider.displayName} 凭据已安全写入操作系统。`);
    } catch (saveError) {
      setOperationError(saveError instanceof Error ? saveError.message : String(saveError));
    } finally {
      setBusyProviderId(null);
    }
  };

  const deleteCredential = async (provider: ProviderDescriptor) => {
    if (!window.confirm(`删除 ${provider.displayName} 的安全凭据？之后需要重新输入 Key 才能调用真实模型。`)) {
      return;
    }
    setBusyProviderId(provider.id);
    setOperationError(null);
    setOperationMessage(null);
    try {
      await onDeleteCredential(provider.id);
      setConnectionResults((current) => {
        const next = { ...current };
        delete next[provider.id];
        return next;
      });
      setOperationMessage(`${provider.displayName} 凭据已删除。`);
    } catch (deleteError) {
      setOperationError(deleteError instanceof Error ? deleteError.message : String(deleteError));
    } finally {
      setBusyProviderId(null);
    }
  };

  const testConnection = async (provider: ProviderDescriptor) => {
    setBusyProviderId(provider.id);
    setOperationError(null);
    setOperationMessage(null);
    try {
      const result = await onTestConnection(provider.id);
      setConnectionResults((current) => ({ ...current, [provider.id]: result }));
      const modelSummary = result.availableModels.length > 0
        ? `可用模型：${result.availableModels.join("、")}`
        : "厂商没有返回可用模型列表";
      setOperationMessage(`${provider.displayName} 连接成功。${modelSummary}。`);
    } catch (connectionError) {
      setConnectionResults((current) => {
        const next = { ...current };
        delete next[provider.id];
        return next;
      });
      setOperationError(connectionError instanceof Error ? connectionError.message : String(connectionError));
    } finally {
      setBusyProviderId(null);
    }
  };

  const updateReadingPreference = <Key extends keyof ReadingPreferences>(
    key: Key,
    value: ReadingPreferences[Key],
  ) => {
    onReadingPreferencesChange(
      { ...readingPreferences, [key]: value },
      readingPreferencesSessionOnly ? "session" : "workspace",
    );
  };

  const installSemanticModel = async () => {
    setSemanticBusy(true);
    setSemanticError(null);
    try {
      setSemanticStatus(await onInstallSemanticModel());
    } catch (installError) {
      setSemanticError(installError instanceof Error ? installError.message : String(installError));
    } finally {
      setSemanticBusy(false);
    }
  };

  const updateReadingPreferences = (patch: Partial<ReadingPreferences>) => {
    onReadingPreferencesChange(
      { ...readingPreferences, ...patch },
      readingPreferencesSessionOnly ? "session" : "workspace",
    );
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="dialog-header">
          <div>
            <span className="eyebrow">WORKSPACE SETTINGS</span>
            <h2 id="settings-title">模型与阅读设置</h2>
          </div>
          <div className="dialog-header-actions">
            <button className="icon-button" type="button" onClick={() => void onRefresh()} aria-label="刷新 Provider 状态" disabled={loading}>
              <RefreshCw className={loading ? "spin" : ""} aria-hidden="true" />
            </button>
            <button className="icon-button" type="button" onClick={onClose} aria-label="关闭模型设置">
              <X aria-hidden="true" />
            </button>
          </div>
        </header>

        <div className="settings-content">
          {/* B4 functional entry. Fixed model/revision/hash are owned by the Rust
              installer; this UI never accepts an arbitrary URL or local path. */}
          <section className="provider-card provider-config-card" aria-labelledby="semantic-model-title">
            <div className="provider-card-heading">
              <span className="provider-icon"><Server aria-hidden="true" /></span>
              <div>
                <strong id="semantic-model-title">本地语义检索模型</strong>
                <p>显式安装约 330 MB 的固定版本模型与推理运行库；安装后检索推理完全在本机执行。</p>
              </div>
              <span className="provider-status">
                {semanticStatus?.state === "ready" ? <CheckCircle2 aria-hidden="true" /> : null}
                {semanticStatus?.state === "ready" ? "384 维 · 已校验" : semanticStatus?.state === "corrupt" ? "校验失败" : "未安装"}
              </span>
            </div>
            <div className="credential-editor">
              <small>未安装、损坏或推理失败时，系统会明确停用向量检索并回退到全文与关系检索。</small>
              <div>
                <button type="button" disabled={semanticBusy || semanticStatus?.state === "ready"} onClick={() => void installSemanticModel()}>
                  {semanticBusy ? <LoaderCircle className="spin" aria-hidden="true" /> : <Download aria-hidden="true" />}
                  {semanticBusy ? "正在下载并校验…" : semanticStatus?.state === "corrupt" ? "重新安装" : "安装语义模型"}
                </button>
              </div>
              {semanticError ? <small className="connection-result is-error">{semanticError}</small> : null}
            </div>
          </section>
          {/*
            UI-HANDOFF-06
            位置：工作区设置中的阅读偏好面板
            用途：即时预览并保存阅读字体和排版；覆盖用户输入、已发送问题、Markdown 正文和聚焦阅读器
            数据：版本化 localStorage，按本地 workspace.id 隔离；session 模式不持久化
            状态：安全默认、工作区已保存、本次会话预览、存储失败回退均保持区分
            交互约束：按钮组键盘可达；固定白名单字体栈；不接收 URL/任意 CSS/文件路径，不改变会话或模型请求
            可替换范围：员工06可替换排版、预览和控件视觉；不可改变安全取值范围、workspace/session 语义和数据隔离
          */}
          <section className="reading-settings-card" aria-labelledby="reading-settings-title">
            <header>
              <span className="provider-icon"><BookOpen aria-hidden="true" /></span>
              <div>
                <strong id="reading-settings-title">长文阅读</strong>
                <p>统一调整你的输入、已发送问题、AI 正文和聚焦阅读；按钮、状态与代码仍使用界面字体。</p>
              </div>
              <span className="reading-scope-status">
                {readingPreferencesSessionOnly ? "仅本次会话" : "已保存到本地工作区"}
              </span>
            </header>

            <fieldset className="reading-font-picker">
              <legend>阅读字体</legend>
              <div className="reading-system-fonts">
                <div className="reading-option-group">
                  {READING_FONT_OPTIONS.map((option) => (
                    <button
                      type="button"
                      key={option.value}
                      className={readingPreferences.font === option.value ? "is-active" : ""}
                      aria-pressed={readingPreferences.font === option.value}
                      onClick={() => updateReadingPreference("font", option.value)}
                    >{option.label}</button>
                  ))}
                </div>
                <span>系统字体无需加载，始终可用。</span>
              </div>
              <div className="reading-font-groups">
                {READING_FONT_GROUPS.map((group) => (
                  <section key={group.id} aria-labelledby={`reading-font-group-${group.id}`}>
                    <h3 id={`reading-font-group-${group.id}`}>{group.label}</h3>
                    <div className="reading-font-card-grid">
                      {BUILT_IN_READING_FONT_PRESETS.filter((preset) => preset.group === group.id).map((preset) => {
                        const availability = fontAvailability[preset.id] ?? "checking";
                        const statusLabel = availability === "available"
                          ? "内置字体可用"
                          : availability === "fallback"
                            ? "使用安全回退"
                            : "正在验证字体";
                        return (
                          <button
                            type="button"
                            key={preset.id}
                            className={`reading-font-card${readingPreferences.font === preset.id ? " is-active" : ""}`}
                            data-font-preview={preset.id}
                            aria-pressed={readingPreferences.font === preset.id}
                            onClick={() => updateReadingPreference("font", preset.id)}
                          >
                            <span className="reading-font-card-heading">
                              <strong>{preset.label}</strong>
                              <small className={`reading-font-status is-${availability}`}>{statusLabel}</small>
                            </span>
                            <span className="reading-font-sample" lang="zh-CN">{preset.preview}</span>
                            <span className="reading-font-facts">
                              <small>{preset.purpose}</small>
                              <small>{preset.coverage}</small>
                              <small>{preset.weights}</small>
                            </span>
                            <span className="reading-font-fallback">{preset.fallbackSummary}</span>
                          </button>
                        );
                      })}
                    </div>
                  </section>
                ))}
              </div>
            </fieldset>

            <div className="reading-settings-grid">
              <fieldset>
                <legend>字号</legend>
                <div className="reading-option-group">
                  {READING_SIZE_OPTIONS.map((option) => (
                    <button
                      type="button"
                      key={option.value}
                      className={readingPreferences.fontSize === option.value ? "is-active" : ""}
                      aria-pressed={readingPreferences.fontSize === option.value}
                      onClick={() => updateReadingPreferences({ fontSize: option.value, fontSizePx: option.size })}
                    >{option.label}</button>
                  ))}
                </div>
              </fieldset>
              <fieldset>
                <legend>行高</legend>
                <div className="reading-option-group">
                  {READING_LINE_HEIGHT_OPTIONS.map((option) => (
                    <button
                      type="button"
                      key={option.value}
                      className={readingPreferences.lineHeight === option.value ? "is-active" : ""}
                      aria-pressed={readingPreferences.lineHeight === option.value}
                      onClick={() => updateReadingPreferences({
                        lineHeight: option.value,
                        lineHeightValue: option.lineHeight,
                      })}
                    >{option.label}</button>
                  ))}
                </div>
              </fieldset>
              <fieldset>
                <legend>阅读宽度</legend>
                <div className="reading-option-group">
                  {READING_WIDTH_OPTIONS.map((option) => (
                    <button
                      type="button"
                      key={option.value}
                      className={readingPreferences.width === option.value ? "is-active" : ""}
                      aria-pressed={readingPreferences.width === option.value}
                      onClick={() => updateReadingPreferences({ width: option.value, readingWidthCh: option.width })}
                    >{option.label}</button>
                  ))}
                </div>
              </fieldset>
            </div>

            <div className="reading-custom-controls" aria-label="自定义阅读排版">
              <label className="reading-range-control">
                <span><strong>字号</strong><output>{readingPreferences.fontSizePx}px</output></span>
                <input
                  type="range"
                  min={READING_PREFERENCE_LIMITS.fontSizePx.min}
                  max={READING_PREFERENCE_LIMITS.fontSizePx.max}
                  step={READING_PREFERENCE_LIMITS.fontSizePx.step}
                  value={readingPreferences.fontSizePx}
                  onChange={(event) => updateReadingPreferences({
                    fontSize: "custom",
                    fontSizePx: Number(event.currentTarget.value),
                  })}
                />
              </label>
              <label className="reading-range-control">
                <span><strong>行高</strong><output>{readingPreferences.lineHeightValue.toFixed(2)}</output></span>
                <input
                  type="range"
                  min={READING_PREFERENCE_LIMITS.lineHeightValue.min}
                  max={READING_PREFERENCE_LIMITS.lineHeightValue.max}
                  step={READING_PREFERENCE_LIMITS.lineHeightValue.step}
                  value={readingPreferences.lineHeightValue}
                  onChange={(event) => updateReadingPreferences({
                    lineHeight: "custom",
                    lineHeightValue: Number(event.currentTarget.value),
                  })}
                />
              </label>
              <label className="reading-range-control">
                <span><strong>字间距</strong><output>{readingPreferences.letterSpacingEm.toFixed(3)}em</output></span>
                <input
                  type="range"
                  min={READING_PREFERENCE_LIMITS.letterSpacingEm.min}
                  max={READING_PREFERENCE_LIMITS.letterSpacingEm.max}
                  step={READING_PREFERENCE_LIMITS.letterSpacingEm.step}
                  value={readingPreferences.letterSpacingEm}
                  onChange={(event) => updateReadingPreference(
                    "letterSpacingEm",
                    Number(event.currentTarget.value),
                  )}
                />
              </label>
              <label className="reading-range-control">
                <span><strong>段落间距</strong><output>{readingPreferences.paragraphSpacingEm.toFixed(1)}em</output></span>
                <input
                  type="range"
                  min={READING_PREFERENCE_LIMITS.paragraphSpacingEm.min}
                  max={READING_PREFERENCE_LIMITS.paragraphSpacingEm.max}
                  step={READING_PREFERENCE_LIMITS.paragraphSpacingEm.step}
                  value={readingPreferences.paragraphSpacingEm}
                  onChange={(event) => updateReadingPreference(
                    "paragraphSpacingEm",
                    Number(event.currentTarget.value),
                  )}
                />
              </label>
              <label className="reading-range-control is-wide">
                <span><strong>阅读宽度</strong><output>{readingPreferences.readingWidthCh} 字符</output></span>
                <input
                  type="range"
                  min={READING_PREFERENCE_LIMITS.readingWidthCh.min}
                  max={READING_PREFERENCE_LIMITS.readingWidthCh.max}
                  step={READING_PREFERENCE_LIMITS.readingWidthCh.step}
                  value={readingPreferences.readingWidthCh}
                  onChange={(event) => updateReadingPreferences({
                    width: "custom",
                    readingWidthCh: Number(event.currentTarget.value),
                  })}
                />
              </label>
            </div>

            <div className="reading-settings-preview" aria-label="阅读排版预览">
              <div className="reading-question-preview">
                <span>你的问题</span>
                <p>请帮我分析这段内容，并保留关键细节。</p>
              </div>
              <SafeMarkdown markdown={READING_PREVIEW_MARKDOWN} />
            </div>

            <footer className="reading-settings-actions">
              <button
                type="button"
                aria-pressed={readingPreferencesSessionOnly}
                onClick={() => onReadingPreferencesChange(
                  readingPreferences,
                  readingPreferencesSessionOnly ? "workspace" : "session",
                )}
              >{readingPreferencesSessionOnly ? "保存到本地工作区" : "仅本次会话预览"}</button>
              <button
                type="button"
                onClick={() => onReadingPreferencesChange(
                  DEFAULT_READING_PREFERENCES,
                  readingPreferencesSessionOnly ? "session" : "workspace",
                )}
              ><RotateCcw aria-hidden="true" />恢复默认</button>
              <button type="button" disabled title="等待受控字体目录与校验 IPC">导入本地字体 · 待接入</button>
            </footer>
          </section>

          {loading && providers.length === 0 ? (
            <div className="settings-state"><LoaderCircle className="spin" aria-hidden="true" />正在读取 Provider 注册表…</div>
          ) : null}
          {!loading && providers.length === 0 ? (
            <div className="settings-state is-error">没有发现已注册的 Provider，请检查桌面内核运行状态。</div>
          ) : null}

          {providers.map((provider) => {
            const hasCredential = hasUsableCredential(provider, credentialStatus);
            const isMock = !provider.credentialRequired && provider.id === "mock";
            const busy = busyProviderId === provider.id;
            const connectionResult = connectionResults[provider.id];
            return (
              <article className={`provider-card provider-config-card${hasCredential ? " is-active" : " is-pending"}`} key={provider.id}>
                <div className="provider-card-heading">
                  <span className="provider-icon">
                    {provider.credentialRequired ? <KeyRound aria-hidden="true" /> : <Server aria-hidden="true" />}
                  </span>
                  <div>
                    <strong>{provider.displayName}</strong>
                    <p>{isMock ? "本地测试 Provider，不联网、不计费。" : provider.defaultBaseUrl ?? "Provider 未声明默认端点。"}</p>
                  </div>
                  <span className="provider-status">
                    {hasCredential ? <CheckCircle2 aria-hidden="true" /> : null}
                    {isMock ? "本地测试" : hasCredential ? "凭据已配置" : "缺少 Key"}
                  </span>
                </div>

                <div className="provider-models" aria-label={`${provider.displayName} 模型`}>
                  {Object.keys(provider.models).sort().map((modelId) => {
                    const selected = selectedModel?.providerId === provider.id && selectedModel.modelId === modelId;
                    const capabilityBadges = describeModelCapabilities(provider.models[modelId]);
                    return (
                      <div className="provider-model-option" key={modelId}>
                        <button
                          className={selected ? "is-selected" : ""}
                          type="button"
                          disabled={!hasCredential}
                          onClick={() => onSelectModel({ providerId: provider.id, modelId })}
                        >
                          <span>{modelId}</span>
                          <small>{selected ? "下一次使用" : hasCredential ? "选择" : "先配置 Key"}</small>
                        </button>
                        {/*
                          UI-HANDOFF-06
                          位置：设置弹窗的 Provider 模型列表中“能力快照”折叠区域
                          用途：在选择模型前披露 reasoning、sampling、结构化输出和工具等真实能力边界
                          数据/IPC：providerCatalog 来自 listProviders 的结构化结果；describeModelCapabilities 只做纯展示投影
                          状态：正常显示能力徽章；加载/错误由设置弹窗既有状态承载；无 Key 时选择按钮保持禁用，不伪造可用能力
                          交互约束：details/summary 保持键盘展开，模型选择和 Key 输入焦点路径不变，Key 仅通过安全凭据命令处理
                          可替换范围：员工06可替换徽章布局、折叠视觉和提示动效
                          不可改变：providerCatalog 字段、listProviders 语义、选择禁用条件、测试选择器和凭据安全边界
                        */}
                        <details className="model-capability-details">
                          <summary>能力快照</summary>
                          <ul aria-label={`${modelId} 能力快照`}>
                            {capabilityBadges.map((badge) => (
                              <li className={`capability-badge is-${badge.tone}`} key={badge.label}>{badge.label}</li>
                            ))}
                          </ul>
                        </details>
                      </div>
                    );
                  })}
                </div>

                {provider.credentialRequired ? (
                  <div className="credential-editor">
                    <label htmlFor={`credential-${provider.id}`}>API Key</label>
                    <div>
                      <input
                        id={`credential-${provider.id}`}
                        ref={(element) => { credentialInputs.current[provider.id] = element; }}
                        type="password"
                        autoComplete="new-password"
                        spellCheck={false}
                        placeholder={hasCredential ? "输入新 Key 以替换（现有值不回显）" : "输入 API Key"}
                        disabled={busy}
                      />
                      <button type="button" disabled={busy} onClick={() => void saveCredential(provider)}>
                        {busy ? <LoaderCircle className="spin" aria-hidden="true" /> : <ShieldCheck aria-hidden="true" />}
                        {hasCredential ? "替换" : "安全保存"}
                      </button>
                      {hasCredential ? (
                        <button type="button" disabled={busy} onClick={() => void testConnection(provider)}>
                          <Wifi aria-hidden="true" />测试连接
                        </button>
                      ) : null}
                      {hasCredential ? (
                        <button className="danger-action" type="button" disabled={busy} onClick={() => void deleteCredential(provider)}>
                          <Trash2 aria-hidden="true" />删除
                        </button>
                      ) : null}
                    </div>
                    <small>凭据引用：{provider.id}/default。明文只传给操作系统安全凭据服务。</small>
                    {connectionResult ? (
                      <small className="connection-result">
                        最近验证：{new Date(connectionResult.checkedAt).toLocaleString()} · 已鉴权 · {connectionResult.availableModels.length} 个可用模型
                      </small>
                    ) : null}
                  </div>
                ) : null}
              </article>
            );
          })}

          {error || operationError ? <p className="settings-feedback is-error" role="alert">{operationError ?? error}</p> : null}
          {operationMessage ? <p className="settings-feedback is-success" role="status">{operationMessage}</p> : null}

          <div className="security-note">
            <ShieldCheck aria-hidden="true" />
            <div>
              <strong>Key 不会回显，也不会写入普通数据库</strong>
              <p>页面只查询“是否已配置”。保存后输入框立即清空，运行请求仅携带凭据引用。</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

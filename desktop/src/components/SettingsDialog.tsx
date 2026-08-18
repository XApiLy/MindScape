import { useEffect, useRef, useState } from "react";
import {
  CheckCircle2,
  KeyRound,
  LoaderCircle,
  RefreshCw,
  Server,
  ShieldCheck,
  Trash2,
  Wifi,
  X,
} from "lucide-react";
import { hasUsableCredential } from "../app/providerCatalog";
import type {
  ModelSelection,
  ProviderConnectionTestResult,
  ProviderDescriptor,
} from "../domain";

type SettingsDialogProps = {
  open: boolean;
  loading: boolean;
  error: string | null;
  providers: ProviderDescriptor[];
  credentialStatus: Record<string, boolean>;
  selectedModel: ModelSelection | null;
  onClose: () => void;
  onRefresh: () => Promise<void>;
  onSelectModel: (selection: ModelSelection) => void;
  onSaveCredential: (providerId: string, secret: string) => Promise<void>;
  onDeleteCredential: (providerId: string) => Promise<void>;
  onTestConnection: (providerId: string) => Promise<ProviderConnectionTestResult>;
};

export function SettingsDialog({
  open,
  loading,
  error,
  providers,
  credentialStatus,
  selectedModel,
  onClose,
  onRefresh,
  onSelectModel,
  onSaveCredential,
  onDeleteCredential,
  onTestConnection,
}: SettingsDialogProps) {
  const credentialInputs = useRef<Record<string, HTMLInputElement | null>>({});
  const [busyProviderId, setBusyProviderId] = useState<string | null>(null);
  const [operationMessage, setOperationMessage] = useState<string | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [connectionResults, setConnectionResults] = useState<Record<string, ProviderConnectionTestResult>>({});

  useEffect(() => {
    if (!open) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose, open]);

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
            <span className="eyebrow">MODEL SETTINGS</span>
            <h2 id="settings-title">模型与安全凭据</h2>
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
                    return (
                      <button
                        className={selected ? "is-selected" : ""}
                        type="button"
                        key={modelId}
                        disabled={!hasCredential}
                        onClick={() => onSelectModel({ providerId: provider.id, modelId })}
                      >
                        <span>{modelId}</span>
                        <small>{selected ? "下一次使用" : hasCredential ? "选择" : "先配置 Key"}</small>
                      </button>
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

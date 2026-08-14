import { useEffect, useMemo, useState } from "react";
import { Check, Cloud, Eye, EyeOff, KeyRound, Plus, X } from "lucide-react";
import { useWorkspaceStore } from "../../store/workspaceStore";
import type { ProviderConfig, ProviderKind } from "../../types/workspace";

type ProviderDialogProps = {
  open: boolean;
  onClose: () => void;
};

const providerLabels: Record<ProviderKind, string> = {
  openai: "OpenAI-compatible",
  anthropic: "Anthropic",
  gemini: "Gemini",
  deepseek: "DeepSeek",
  openrouter: "OpenRouter",
  custom: "自定义兼容接口",
};

export function ProviderDialog({ open, onClose }: ProviderDialogProps) {
  const configs = useWorkspaceStore((state) => state.providerConfigs);
  const activeProviderId = useWorkspaceStore((state) => state.activeProviderId);
  const setActiveProvider = useWorkspaceStore((state) => state.setActiveProvider);
  const upsertProvider = useWorkspaceStore((state) => state.upsertProvider);
  const [selectedId, setSelectedId] = useState(activeProviderId);
  const [draft, setDraft] = useState<ProviderConfig | null>(null);
  const [showKey, setShowKey] = useState(false);

  const selected = useMemo(() => configs.find((item) => item.id === selectedId) ?? configs[0], [configs, selectedId]);

  useEffect(() => {
    if (selected) setDraft({ ...selected });
  }, [selected]);

  if (!open || !draft) return null;

  const save = () => {
    upsertProvider(draft);
    setActiveProvider(draft.id);
    onClose();
  };

  return (
    <div className="dialog-backdrop" role="dialog" aria-modal="true" aria-label="模型接口设置">
      <section className="dialog-panel provider-dialog">
        <header className="dialog-header">
          <div>
            <span className="eyebrow">MODEL PROVIDERS</span>
            <h2>模型接口</h2>
            <p>统一管理各厂商模型。API Key 仅保存在当前运行内存中。</p>
          </div>
          <button className="icon-button" title="关闭" aria-label="关闭" onClick={onClose}><X size={17} /></button>
        </header>

        <div className="provider-layout">
          <aside className="provider-list">
            {configs.map((config) => (
              <button
                className={config.id === selectedId ? "active" : ""}
                type="button"
                key={config.id}
                onClick={() => setSelectedId(config.id)}
              >
                <Cloud size={16} />
                <span><strong>{config.name}</strong><small>{config.model}</small></span>
                {config.id === activeProviderId ? <Check size={14} /> : null}
              </button>
            ))}
            <button className="add-provider" type="button">
              <Plus size={15} />添加供应商
            </button>
          </aside>

          <div className="provider-form">
            <label>
              供应商
              <select
                value={draft.kind}
                onChange={(event) => setDraft({ ...draft, kind: event.target.value as ProviderKind })}
              >
                {Object.entries(providerLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
              </select>
            </label>
            <label>
              显示名称
              <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
            </label>
            <label>
              模型 ID
              <input value={draft.model} onChange={(event) => setDraft({ ...draft, model: event.target.value })} />
            </label>
            <label>
              API Base URL
              <input value={draft.baseUrl} onChange={(event) => setDraft({ ...draft, baseUrl: event.target.value })} />
            </label>
            <label>
              API Key
              <span className="secret-field">
                <KeyRound size={15} />
                <input
                  type={showKey ? "text" : "password"}
                  value={draft.apiKey}
                  placeholder="仅在当前运行期间保存"
                  onChange={(event) => setDraft({ ...draft, apiKey: event.target.value })}
                />
                <button type="button" title={showKey ? "隐藏" : "显示"} onClick={() => setShowKey((value) => !value)}>
                  {showKey ? <EyeOff size={15} /> : <Eye size={15} />}
                </button>
              </span>
            </label>
            <p className="form-note">浏览器开发模式可能受到供应商 CORS 限制；Tauri 桌面版后续将通过安全代理统一请求。</p>
          </div>
        </div>

        <footer className="dialog-footer">
          <button className="secondary-button" type="button" onClick={onClose}>取消</button>
          <button className="accent-button" type="button" onClick={save}>保存并启用</button>
        </footer>
      </section>
    </div>
  );
}

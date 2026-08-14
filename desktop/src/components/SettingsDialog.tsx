import { CheckCircle2, KeyRound, Server, ShieldCheck, X } from "lucide-react";

type SettingsDialogProps = {
  open: boolean;
  onClose: () => void;
};

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  if (!open) return null;

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
          <button className="icon-button" type="button" onClick={onClose} aria-label="关闭模型设置">
            <X aria-hidden="true" />
          </button>
        </header>

        <div className="settings-content">
          <article className="provider-card is-active">
            <span className="provider-icon"><Server aria-hidden="true" /></span>
            <div>
              <strong>本地模拟 Provider</strong>
              <p>用于验证统一流式事件、慢响应和前端状态；不联网、不计费。</p>
            </div>
            <span className="provider-status"><CheckCircle2 aria-hidden="true" />可用</span>
          </article>

          <article className="provider-card is-pending">
            <span className="provider-icon"><KeyRound aria-hidden="true" /></span>
            <div>
              <strong>真实 Provider</strong>
              <p>等待安全凭据命令和 Provider 注册表接入后开放配置。</p>
            </div>
            <span className="provider-status">待接入</span>
          </article>

          <div className="security-note">
            <ShieldCheck aria-hidden="true" />
            <div>
              <strong>不会在此页面保存明文 API Key</strong>
              <p>正式版本只把密钥交给操作系统安全凭据，数据库和前端状态仅保存凭据引用。</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

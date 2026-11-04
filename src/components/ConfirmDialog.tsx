import { useT } from '../i18n';

interface ConfirmStripProps {
  title: string;
  message: string;
  confirmLabel: string;
  /** 破坏性操作（删除证书等）确认按钮走 --danger。 */
  danger?: boolean;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * 应用内确认条。规范 v2 §7 禁止 window.confirm；用在签名对话框内部，
 * 不再叠一层模态（两层 overlay 会让 Esc / 点背景同时关掉外层对话框）。
 * 视觉复用既有 `.redact-warning` 危险提示样式与 tokens，不新建一套。
 */
export function ConfirmStrip({
  title,
  message,
  confirmLabel,
  danger = false,
  busy = false,
  onCancel,
  onConfirm,
}: ConfirmStripProps) {
  const t = useT();
  return (
    <div className="redact-warning" role="group" aria-label={title}>
      <strong>{title}</strong>
      <p>{message}</p>
      <div className="row">
        <button className="btn btn-sm" onClick={onCancel} disabled={busy}>
          {t('dsign.cancel')}
        </button>
        <button
          className={`btn btn-sm ${danger ? 'btn-danger' : 'btn-primary'}`}
          onClick={onConfirm}
          disabled={busy}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  );
}

import { useT } from '../i18n';

interface RedactBarProps {
  count: number;
  onApply: () => void;
  onCancel: () => void;
}

/**
 * Floating bar shown while redaction rectangles are pending, mirroring the
 * FindBar's position. Stays visible even after the selection mode is exited
 * (Esc / toolbar toggle) — the marked rectangles, and the choice to apply or
 * cancel them, must not be silently lost.
 */
export function RedactBar({ count, onApply, onCancel }: RedactBarProps) {
  const t = useT();
  return (
    <div className="redactbar" role="toolbar" aria-label={t('redact.toolbar')}>
      <span className="redactbar-count">{t('redact.barCount', { count })}</span>
      <button className="btn" onClick={onCancel} disabled={count === 0}>
        {t('redact.cancel')}
      </button>
      <button className="btn btn-danger" onClick={onApply} disabled={count === 0}>
        {t('redact.apply')}
      </button>
    </div>
  );
}

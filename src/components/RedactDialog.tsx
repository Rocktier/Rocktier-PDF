import { useState } from 'react';
import { useT } from '../i18n';
import { useDialogA11y } from '../hooks/useDialog';

interface RedactDialogProps {
  /** How many rectangles are about to be applied. */
  count: number;
  onClose: () => void;
  onConfirm: () => Promise<void>;
}

/**
 * The irreversible-step confirmation for redaction. The warning must exist in
 * words, not just in colour — colour alone carries nothing to screen readers
 * and prints as nothing on paper, and a legal/finance user's judgement of this
 * product hinges on being told plainly, before it happens, that the text is
 * permanently gone.
 */
export function RedactDialog({ count, onClose, onConfirm }: RedactDialogProps) {
  const t = useT();
  const { overlayProps, dialogProps } = useDialogA11y(onClose);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const confirm = async () => {
    setBusy(true);
    setError(null);
    try {
      await onConfirm();
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" {...overlayProps}>
      <div className="modal redact-modal" {...dialogProps} aria-label={t('redact.confirmTitle')}>
        <div className="modal-header">
          <h3>{t('redact.confirmTitle')}</h3>
        </div>

        <div className="modal-body">
          <p>{t('redact.confirmBody', { count })}</p>
          <div className="redact-warning" role="alert">
            <strong>{t('redact.confirmWarningTitle')}</strong>
            <p>{t('redact.confirmWarning')}</p>
          </div>
          {error ? (
            <p className="hint" style={{ color: 'var(--danger)' }}>
              {error}
            </p>
          ) : null}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            {t('redact.cancel')}
          </button>
          <button className="btn btn-danger" onClick={confirm} disabled={busy}>
            {t('redact.confirmRun')}
          </button>
        </div>
      </div>
    </div>
  );
}

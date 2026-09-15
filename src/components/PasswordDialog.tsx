import { useState } from 'react';
import { useT } from '../i18n';

interface PasswordDialogProps {
  /** True when a password was already tried and rejected. */
  incorrect: boolean;
  onClose: () => void;
  /** Returns true when the password unlocked the document. */
  onSubmit: (password: string) => Promise<boolean>;
}

/** Prompt for the password of an encrypted PDF. */
export function PasswordDialog({ incorrect, onClose, onSubmit }: PasswordDialogProps) {
  const t = useT();
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);
  const [wrong, setWrong] = useState(incorrect);

  const submit = async () => {
    if (!password) return;
    setBusy(true);
    const ok = await onSubmit(password);
    setBusy(false);
    setWrong(!ok);
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div
        className="modal"
        style={{ width: 'min(380px, 92vw)' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h3>{t('password.title')}</h3>
        </div>

        <div className="modal-body">
          <p className="hint">{t('password.hint')}</p>
          <div className="field">
            <label>{t('password.label')}</label>
            <input
              className="input"
              type="password"
              value={password}
              autoFocus
              onChange={(e) => {
                setPassword(e.target.value);
                setWrong(false);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void submit();
              }}
            />
          </div>
          {wrong ? (
            <p className="hint" style={{ color: 'var(--danger)' }}>
              {t('password.wrong')}
            </p>
          ) : null}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            {t('password.cancel')}
          </button>
          <button className="btn btn-primary" onClick={submit} disabled={busy || !password}>
            {t('password.open')}
          </button>
        </div>
      </div>
    </div>
  );
}

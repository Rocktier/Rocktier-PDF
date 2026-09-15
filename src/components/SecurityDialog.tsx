import { useState } from 'react';
import { useT } from '../i18n';

type Mode = 'set' | 'remove';

interface SecurityDialogProps {
  onClose: () => void;
  onRun: (mode: Mode, password: string, ownerPassword: string) => Promise<void>;
}

/** Set or remove a PDF's password protection (writes a *copy*). */
export function SecurityDialog({ onClose, onRun }: SecurityDialogProps) {
  const t = useT();
  const [mode, setMode] = useState<Mode>('set');
  const [password, setPassword] = useState('');
  const [owner, setOwner] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    if (!password) {
      setError(t('security.needPassword'));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await onRun(mode, password, owner);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div className="modal" style={{ width: 'min(440px, 92vw)' }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{t('security.title')}</h3>
        </div>

        <div className="modal-body">
          <div className="field">
            <label>{t('security.mode')}</label>
            <div className="row">
              <button
                className={`btn${mode === 'set' ? ' btn-primary' : ''}`}
                onClick={() => setMode('set')}
              >
                {t('security.set')}
              </button>
              <button
                className={`btn${mode === 'remove' ? ' btn-primary' : ''}`}
                onClick={() => setMode('remove')}
              >
                {t('security.remove')}
              </button>
            </div>
          </div>

          <div className="field">
            <label>{t('security.password')}</label>
            <input
              className="input"
              type="password"
              value={password}
              autoFocus
              onChange={(e) => setPassword(e.target.value)}
            />
          </div>

          {mode === 'set' ? (
            <div className="field">
              <label>{t('security.ownerPassword')}</label>
              <input
                className="input"
                type="password"
                value={owner}
                onChange={(e) => setOwner(e.target.value)}
              />
              <p className="hint">{t('security.ownerHint')}</p>
            </div>
          ) : null}

          {error ? (
            <p className="hint" style={{ color: 'var(--danger)' }}>
              {error}
            </p>
          ) : null}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            {t('security.cancel')}
          </button>
          <button className="btn btn-primary" onClick={run} disabled={busy || !password}>
            {t('security.run')}
          </button>
        </div>
      </div>
    </div>
  );
}

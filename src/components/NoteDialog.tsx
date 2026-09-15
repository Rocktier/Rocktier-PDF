import { useState } from 'react';
import { useT } from '../i18n';

interface NoteDialogProps {
  onClose: () => void;
  onRun: (text: string) => Promise<void>;
}

/** Small dialog for typing the body of a sticky note. */
export function NoteDialog({ onClose, onRun }: NoteDialogProps) {
  const t = useT();
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    if (!text.trim()) {
      setError(t('note.placeholder'));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await onRun(text.trim());
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{t('note.title')}</h3>
        </div>

        <div className="modal-body">
          <div className="field">
            <label>{t('note.label')}</label>
            <textarea
              className="input"
              style={{ height: 96, padding: 8, resize: 'vertical', fontFamily: 'inherit' }}
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={t('note.placeholder')}
              autoFocus
            />
          </div>
          {error ? (
            <p className="hint" style={{ color: 'var(--danger)' }}>
              {error}
            </p>
          ) : null}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            {t('note.cancel')}
          </button>
          <button className="btn btn-primary" onClick={run} disabled={busy}>
            {t('note.run')}
          </button>
        </div>
      </div>
    </div>
  );
}

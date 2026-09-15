import { useState } from 'react';
import { useT } from '../i18n';
import type { StampKind } from '../types';

interface StampDialogProps {
  onClose: () => void;
  onRun: (
    kind: StampKind,
    text: string,
    fontSize: number,
    margin: number,
    opacity: number
  ) => Promise<void>;
}

/** Page numbers + text watermark, both applied to every page. */
export function StampDialog({ onClose, onRun }: StampDialogProps) {
  const t = useT();
  const [kind, setKind] = useState<StampKind>('pageNumbers');
  const [text, setText] = useState('');
  const [fontSize, setFontSize] = useState(48);
  const [margin, setMargin] = useState(28);
  const [opacity, setOpacity] = useState(0.15);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    if (kind === 'watermark' && !text.trim()) {
      setError(t('stamp.needText'));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await onRun(kind, text.trim(), fontSize, margin, opacity);
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
          <h3>{t('stamp.title')}</h3>
        </div>

        <div className="modal-body">
          <div className="field">
            <label>{t('stamp.kind')}</label>
            <div className="row">
              <button
                className={`btn${kind === 'pageNumbers' ? ' btn-primary' : ''}`}
                onClick={() => setKind('pageNumbers')}
              >
                {t('stamp.pageNumbers')}
              </button>
              <button
                className={`btn${kind === 'watermark' ? ' btn-primary' : ''}`}
                onClick={() => setKind('watermark')}
              >
                {t('stamp.watermark')}
              </button>
            </div>
          </div>

          {kind === 'watermark' ? (
            <div className="field">
              <label>{t('stamp.text')}</label>
              <input
                className="input"
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder={t('stamp.textPlaceholder')}
                autoFocus
              />
            </div>
          ) : null}

          <div className="field">
            <label>{t('stamp.fontSize')}</label>
            <input
              className="input"
              type="number"
              min={6}
              max={200}
              value={fontSize}
              onChange={(e) => setFontSize(Number(e.target.value))}
            />
          </div>

          {kind === 'pageNumbers' ? (
            <div className="field">
              <label>{t('stamp.margin')}</label>
              <input
                className="input"
                type="number"
                min={4}
                max={200}
                value={margin}
                onChange={(e) => setMargin(Number(e.target.value))}
              />
            </div>
          ) : (
            <div className="field">
              <label>{t('stamp.opacity')}</label>
              <input
                className="input"
                type="number"
                min={5}
                max={100}
                value={Math.round(opacity * 100)}
                onChange={(e) => setOpacity(Number(e.target.value) / 100)}
              />
            </div>
          )}

          {error ? (
            <p className="hint" style={{ color: 'var(--danger)' }}>
              {error}
            </p>
          ) : null}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            {t('stamp.cancel')}
          </button>
          <button className="btn btn-primary" onClick={run} disabled={busy}>
            {t('stamp.run')}
          </button>
        </div>
      </div>
    </div>
  );
}

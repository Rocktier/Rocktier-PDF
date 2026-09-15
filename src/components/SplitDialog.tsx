import { useState } from 'react';
import { useT } from '../i18n';
import { pickDirectory } from '../services/engine';
import type { SplitMode } from '../types';
import { Modal } from './Modal';

interface SplitDialogProps {
  pageCount: number;
  onClose: () => void;
  onRun: (outputDir: string, mode: SplitMode) => Promise<string[]>;
}

type ModeKind = SplitMode['kind'];

export function SplitDialog({ pageCount, onClose, onRun }: SplitDialogProps) {
  const t = useT();
  const [kind, setKind] = useState<ModeKind>('everyPage');
  const [n, setN] = useState('2');
  const [ranges, setRanges] = useState('');
  const [output, setOutput] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const buildMode = (): SplitMode => {
    if (kind === 'everyN') return { kind: 'everyN', n: Math.max(1, parseInt(n, 10) || 1) };
    if (kind === 'ranges') return { kind: 'ranges', ranges };
    return { kind: 'everyPage' };
  };

  const chooseOutput = async () => {
    const dir = await pickDirectory();
    if (dir) setOutput(dir);
  };

  const run = async () => {
    if (!output) {
      setError(t('split.needOutput'));
      return;
    }
    setBusy(true);
    setError(null);
    const paths = await onRun(output, buildMode());
    setBusy(false);
    if (paths.length > 0) onClose();
    else setError(t('split.failed'));
  };

  return (
    <Modal
      title={t('split.title')}
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            {t('split.cancel')}
          </button>
          <button className="btn btn-primary" onClick={run} disabled={busy}>
            {busy ? <span className="spinner" /> : null}
            {t('split.run')}
          </button>
        </>
      }
    >
      <div className="field">
        <label>{t('split.mode')}</label>
        <div className="row">
          <button
            className={`btn btn-sm${kind === 'everyPage' ? ' btn-primary' : ''}`}
            onClick={() => setKind('everyPage')}
          >
            {t('split.everyPage')}
          </button>
          <button
            className={`btn btn-sm${kind === 'everyN' ? ' btn-primary' : ''}`}
            onClick={() => setKind('everyN')}
          >
            {t('split.everyN')}
          </button>
          <button
            className={`btn btn-sm${kind === 'ranges' ? ' btn-primary' : ''}`}
            onClick={() => setKind('ranges')}
          >
            {t('split.ranges')}
          </button>
        </div>
      </div>

      {kind === 'everyN' ? (
        <div className="field">
          <label>{t('split.nPages')}</label>
          <input
            className="input"
            value={n}
            onChange={(e) => setN(e.target.value.replace(/[^\d]/g, ''))}
            inputMode="numeric"
          />
        </div>
      ) : null}

      {kind === 'ranges' ? (
        <div className="field">
          <label>{t('split.ranges')}</label>
          <input
            className="input"
            value={ranges}
            onChange={(e) => setRanges(e.target.value)}
            placeholder={t('split.rangesPlaceholder')}
          />
          <p className="hint">1 – {pageCount}</p>
        </div>
      ) : null}

      <div className="field">
        <label>{t('split.output')}</label>
        <div className="row">
          <input
            className="input"
            style={{ flex: 1 }}
            value={output}
            readOnly
            placeholder="…"
            onClick={chooseOutput}
          />
          <button className="btn" onClick={chooseOutput} disabled={busy}>
            {t('split.choose')}
          </button>
        </div>
      </div>

      {error ? <p className="hint" style={{ color: 'var(--danger)' }}>{error}</p> : null}
    </Modal>
  );
}

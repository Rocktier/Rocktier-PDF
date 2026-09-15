import { useState } from 'react';
import { useT } from '../i18n';
import { fileStem, pickPdfs, pickSavePath } from '../services/engine';
import { IconPlus, IconTrash } from './Logo';
import { Modal } from './Modal';

interface MergeDialogProps {
  onClose: () => void;
  onRun: (paths: string[], outputPath: string) => Promise<string | null>;
}

export function MergeDialog({ onClose, onRun }: MergeDialogProps) {
  const t = useT();
  const [files, setFiles] = useState<string[]>([]);
  const [output, setOutput] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const add = async () => {
    const picked = await pickPdfs();
    if (!picked || picked.length === 0) return;
    setFiles((prev) => [...prev, ...picked]);
    setError(null);
  };

  const chooseOutput = async () => {
    const name = files.length > 0 ? `${fileStem(files[0])}-merged.pdf` : 'merged.pdf';
    const path = await pickSavePath(name);
    if (path) setOutput(path);
  };

  const run = async () => {
    if (files.length < 2) {
      setError('Pick at least two PDFs to merge');
      return;
    }
    if (!output) {
      setError('Choose where to save the merged file');
      return;
    }
    setBusy(true);
    setError(null);
    const result = await onRun(files, output);
    setBusy(false);
    if (result) onClose();
    else setError('Merge failed');
  };

  return (
    <Modal
      title={t('merge.title')}
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            {t('merge.cancel')}
          </button>
          <button className="btn btn-primary" onClick={run} disabled={busy || files.length < 2}>
            {busy ? <span className="spinner" /> : null}
            {t('merge.run')}
          </button>
        </>
      }
    >
      {files.length === 0 ? (
        <p className="hint">{t('merge.empty')}</p>
      ) : (
        files.map((path, index) => (
          <div className="file-row" key={`${path}-${index}`}>
            <span className="name">{fileStem(path)}.pdf</span>
            <button
              className="btn btn-icon btn-sm"
              onClick={() => setFiles((prev) => prev.filter((_, i) => i !== index))}
              aria-label="Remove"
            >
              <IconTrash size={14} />
            </button>
          </div>
        ))
      )}

      <button className="btn" onClick={add} disabled={busy}>
        <IconPlus />
        {t('merge.add')}
      </button>

      <div className="field">
        <label>{t('merge.output')}</label>
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
      <p className="hint">{t('merge.hint')}</p>
    </Modal>
  );
}

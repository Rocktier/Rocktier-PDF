import { useState } from 'react';
import { pickSavePath } from '../services/engine';
import { Modal } from './Modal';

// TODO(i18n)：这三个标签与说明目前是英文常量。家族其余对话框都走 i18n，
// 这里先落功能，随后把键补进各语言文件（键名建议 compress.profiles.{web,balanced,archive}）。
const PROFILES = [
  {
    id: 'web' as const,
    label: 'Smallest (web)',
    desc: 'Lowest quality. For sending by mail or embedding in a web page.',
  },
  {
    id: 'balanced' as const,
    label: 'Balanced',
    desc: 'Good quality at a much smaller size. The default.',
  },
  {
    id: 'archive' as const,
    label: 'Highest quality (archive)',
    desc: 'Keeps more detail. Use when the document may be printed.',
  },
];

export type CompressProfile = (typeof PROFILES)[number]['id'];

interface CompressDialogProps {
  defaultName: string;
  onClose: () => void;
  onRun: (profile: CompressProfile, output: string) => Promise<boolean>;
}

/**
 * Compress to a copy. The output path is chosen up front and shown, because
 * compression is lossy — the user should be able to see exactly which new file
 * they are about to get, and never wonder whether their original was touched.
 */
export function CompressDialog({ defaultName, onClose, onRun }: CompressDialogProps) {
  const [profile, setProfile] = useState<CompressProfile>('balanced');
  const [output, setOutput] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const chooseOutput = async () => {
    const path = await pickSavePath(defaultName);
    if (path) setOutput(path);
  };

  const run = async () => {
    if (!output) {
      setError('Choose where to save the compressed copy first.');
      return;
    }
    setBusy(true);
    setError(null);
    const ok = await onRun(profile, output);
    setBusy(false);
    if (ok) onClose();
    else setError('Compression failed. The original file was not modified.');
  };

  return (
    <Modal
      title="Compress PDF"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button type="button" onClick={() => void run()} disabled={busy || !output}>
            {busy ? 'Compressing…' : 'Compress'}
          </button>
        </>
      }
    >
      <div className="field">
        {PROFILES.map((p) => (
          <label key={p.id} className="row">
            <input
              type="radio"
              name="compress-profile"
              checked={profile === p.id}
              onChange={() => setProfile(p.id)}
            />
            <span>
              <strong>{p.label}</strong>
              <br />
              <small>{p.desc}</small>
            </span>
          </label>
        ))}
      </div>

      <div className="field">
        <button type="button" onClick={() => void chooseOutput()}>
          Choose output…
        </button>
        {output ? <code>{output}</code> : <small>No destination chosen yet.</small>}
      </div>

      {error ? <p className="error">{error}</p> : null}
      <p>
        <small>
          Writes a new file. Your original is never modified, and the text layer stays
          exactly as it was.
        </small>
      </p>
    </Modal>
  );
}

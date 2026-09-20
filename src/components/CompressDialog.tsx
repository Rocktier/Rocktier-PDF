import { useState } from 'react';
import { useT } from '../i18n';
import { pickSavePath } from '../services/engine';
import { Modal } from './Modal';

export type CompressProfile = 'web' | 'balanced' | 'archive';

const PROFILE_IDS: CompressProfile[] = ['web', 'balanced', 'archive'];

interface CompressDialogProps {
  defaultName: string;
  onClose: () => void;
  onRun: (profile: CompressProfile, output: string) => Promise<boolean>;
}

/**
 * 压缩到一份副本。输出路径由用户先选并展示出来——压缩是有损操作，用户应当
 * 清楚自己将得到哪个新文件，而不是事后怀疑原件有没有被动过。
 */
export function CompressDialog({ defaultName, onClose, onRun }: CompressDialogProps) {
  const t = useT();
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
      setError(t('compress.needOutput'));
      return;
    }
    setBusy(true);
    setError(null);
    const ok = await onRun(profile, output);
    setBusy(false);
    if (ok) onClose();
    else setError(t('compress.failed'));
  };

  return (
    <Modal
      title={t('compress.title')}
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} disabled={busy}>
            {t('compress.cancel')}
          </button>
          <button type="button" onClick={() => void run()} disabled={busy || !output}>
            {busy ? t('compress.busy') : t('compress.run')}
          </button>
        </>
      }
    >
      <div className="field">
        {PROFILE_IDS.map((id) => (
          <label key={id} className="row">
            <input
              type="radio"
              name="compress-profile"
              checked={profile === id}
              onChange={() => setProfile(id)}
            />
            <span>
              <strong>{t(`compress.profiles.${id}.label`)}</strong>
              <br />
              <small>{t(`compress.profiles.${id}.desc`)}</small>
            </span>
          </label>
        ))}
      </div>

      <div className="field">
        <button type="button" onClick={() => void chooseOutput()}>
          {t('compress.choose')}
        </button>
        {output ? <code>{output}</code> : <small>{t('compress.noDestination')}</small>}
      </div>

      {error ? <p className="error">{error}</p> : null}
      <p>
        <small>{t('compress.note')}</small>
      </p>
    </Modal>
  );
}

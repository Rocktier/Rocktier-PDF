import { useEffect, useState } from 'react';
import { useT } from '../i18n';
import { onCompressProgress, pickSavePath, type CompressProgress } from '../services/engine';
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
 *
 * 进度分两段，因为两段的可观测程度不同：
 * - qpdf 结构优化：它不输出任何进度，所以这一段只能是**不定**进度条；
 * - 图像重压：这段是我们的循环，知道总数，所以给**确定**进度与 "已完成/总数"。
 * 不去合成一个跨两段的百分比 —— 那需要编数字。
 */
export function CompressDialog({ defaultName, onClose, onRun }: CompressDialogProps) {
  const t = useT();
  const [profile, setProfile] = useState<CompressProfile>('balanced');
  const [output, setOutput] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<CompressProgress | null>(null);

  // 只在对话框存活期间监听；关闭即解绑（与 App.tsx 订阅菜单事件同一写法，
  // 包括"卸载后才拿到 unlisten"的那一步）。
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void onCompressProgress((p) => setProgress(p)).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

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
    setProgress(null);
    const ok = await onRun(profile, output);
    setBusy(false);
    setProgress(null);
    if (ok) onClose();
    else setError(t('compress.failed'));
  };

  const total = progress?.total ?? 0;
  const done = progress?.done ?? 0;
  const stage = progress?.stage ?? 'optimize';

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
              disabled={busy}
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
        <button type="button" onClick={() => void chooseOutput()} disabled={busy}>
          {t('compress.choose')}
        </button>
        {output ? <code>{output}</code> : <small>{t('compress.noDestination')}</small>}
      </div>

      {busy ? (
        <div className="field" aria-live="polite">
          <div className="row" style={{ justifyContent: 'space-between' }}>
            <span>{t(`compress.stages.${stage}`)}</span>
            {total > 0 ? (
              <span className="mono muted">
                {done}/{total}
              </span>
            ) : null}
          </div>
          {total > 0 ? (
            <progress max={total} value={done} style={{ width: '100%' }} />
          ) : (
            <progress style={{ width: '100%' }} />
          )}
        </div>
      ) : null}

      {error ? <p className="error">{error}</p> : null}
      <p>
        <small>{t('compress.note')}</small>
      </p>
    </Modal>
  );
}

import { useT } from '../i18n';
// 检查更新对话框。入口只在帮助菜单「检查更新…」—— 点击才联网，
// 启动时、工作中、后台都不检查，见 services/updates.ts 头注。
// 结果一律在这里展示：右下角的常驻悬浮入口已移除（fixed 定位不稳定，
// 容易盖住底部内容，Compressor v0.1.10 实测复现），全部收进菜单 + 弹框。
import { Modal } from './Modal';
import type { UpdateResult, UpdateState } from '../services/updates';

interface UpdateDialogProps {
  state: UpdateState;
  result: UpdateResult | null;
  /** 当前安装版本，取自 package.json（与 tag 同源）。 */
  currentVersion: string;
  onClose: () => void;
}

/** 打开发布页：桌面端走 Rust 白名单入口，浏览器 dev 直接开新窗口。 */
function open(url: string) {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    import('@tauri-apps/api/core').then(({ invoke }) => invoke('open_url', { url }).catch(() => {}));
  } else {
    window.open(url, '_blank', 'noopener');
  }
}

export function UpdateDialog({ state, result, currentVersion, onClose }: UpdateDialogProps) {
  const t = useT();

  return (
    <Modal title={t('update.check')} onClose={onClose} footer={
      <>
        <button className="btn" onClick={onClose}>{t('update.close')}</button>
        <span style={{ flex: 1 }} />
        {state === 'failed' && (
          <button className="btn" onClick={() => open('https://github.com/Rocktier/Rocktier-PDF/releases/latest')}>
            {t('update.openReleases')}
          </button>
        )}
        {state === 'available' && result && (
          <button className="btn btn-primary" onClick={() => open(result.url)}>
            {t('update.download', { v: result.latest })}
          </button>
        )}
      </>
    }>
      {state === 'checking' && (
        <p>
          {t('update.checking')}
          <br />
          <small>{t('update.privacyNote')}</small>
        </p>
      )}
      {state === 'uptodate' && (
        <p>
          {t('update.upToDate')}
          <br />
          <small>v{currentVersion}</small>
        </p>
      )}
      {state === 'failed' && (
        <p>
          {t('update.failed')}
          <br />
          <small>{t('update.openReleasesHint')}</small>
        </p>
      )}
      {state === 'available' && result && (
        <p>
          {t('update.available', { v: result.latest })}
          <br />
          <small>v{currentVersion} → v{result.latest}</small>
        </p>
      )}
    </Modal>
  );
}

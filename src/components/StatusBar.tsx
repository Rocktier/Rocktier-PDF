import { useT } from '../i18n';
import { formatBytes, type LicenseInfo } from '../services/engine';
import { localizeError } from '../services/errorText';
import type { DocumentInfo } from '../types';

interface StatusBarProps {
  doc: DocumentInfo | null;
  busy: boolean;
  error: string | null;
  selectedCount: number;
  /** 许可状态：用于显示试用剩余天数、以及作为进入"许可与激活"的入口。 */
  license?: LicenseInfo | null;
  onLicenseClick?: () => void;
  /** 已签名（只读）文档：另存一份剥离签名的可编辑副本。 */
  onSaveEditableCopy: () => void;
}

export function StatusBar({ doc, busy, error, selectedCount, license, onLicenseClick, onSaveEditableCopy }: StatusBarProps) {
  const t = useT();

  /* 只在直链版且尚未买断时提示 —— 商店版由商店收款，这里再提一句"试用/购买"既多余，
     又容易在审核眼里变成"引导外部购买"。 */
  const showLicenseChip =
    !!license && license.channel === 'direct' && license.status !== 'licensed';

  const state = error ? 'error' : busy ? 'busy' : 'ready';
  // Rust 把错误拍平为英文原文返回；这里做一层映射，转成当前语言的措辞。
  // 查不到映射时回退原文 —— 规范要求失败时展示底层真实错误文本，不隐藏细节。
  const label = error
    ? localizeError(error, t)
    : busy
      ? t('status.working')
      : t('status.ready');

  return (
    <div className="statusbar" role="status" aria-live="polite">
      <span className={`status-dot ${state}`} />
      <span>{label}</span>

      {doc ? (
        <>
          <span className="sep">|</span>
          <span>{doc.name}</span>
          <span className="sep">|</span>
          <span>
            {doc.pageCount} {t('status.pages')}
          </span>
          <span className="sep">|</span>
          <span>{formatBytes(doc.fileSize)}</span>
          {selectedCount > 0 ? (
            <>
              <span className="sep">|</span>
              <span>
                {selectedCount} {t('rail.page')}
              </span>
            </>
          ) : null}
          {doc.dirty ? (
            <>
              <span className="sep">|</span>
              <span style={{ color: 'var(--warn)' }}>{t('status.unsaved')}</span>
            </>
          ) : null}
          {/* 只读徽标：规则由后端裁定（ensure_editable / SIGNED_READ_ONLY），这里只是
              把它的结论显示出来，并给出唯一的合法出路 —— 另存剥离签名的副本。 */}
          {doc.readOnly ? (
            <>
              <span className="sep">|</span>
              <span style={{ color: 'var(--warn)' }}>{t('dsignReady.signed')}</span>
              <button type="button" className="license-chip" onClick={onSaveEditableCopy}>
                {t('dsignReady.saveCopy')}
              </button>
            </>
          ) : null}
        </>
      ) : null}

      <div style={{ flex: 1 }} />
      {showLicenseChip ? (
        <button type="button" className="license-chip" onClick={onLicenseClick}>
          {license.status === 'expired'
            ? t('license.expiredChip')
            : t('license.trialChip', { days: license.daysLeft })}
        </button>
      ) : null}
      <span>{t('privacy.badge')}</span>
    </div>
  );
}

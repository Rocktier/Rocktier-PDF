import { useT } from '../i18n';
import { formatBytes, type LicenseInfo } from '../services/engine';
import { localizeError } from '../services/errorText';
import type { DocumentInfo } from '../types';

interface StatusBarProps {
  doc: DocumentInfo | null;
  busy: boolean;
  error: string | null;
  selectedCount: number;
  /** 当前可见页（0 起）与缩放：状态栏只说真实值。 */
  current: number;
  zoom: number;
  /** 许可状态：用于显示试用剩余天数、以及作为进入"许可与激活"的入口。 */
  license?: LicenseInfo | null;
  onLicenseClick?: () => void;
}

/**
 * 状态栏：进行中才放呼吸红点，旁边永远有文字；静止态只有一个中性文字。
 * 右侧是真实的页码、缩放、文件大小，不编造版本或页数。
 */
export function StatusBar({
  doc,
  busy,
  error,
  selectedCount,
  current,
  zoom,
  license,
  onLicenseClick,
}: StatusBarProps) {
  const t = useT();

  /* 只在直链版且尚未买断时提示 —— 商店版由商店收款，这里再提一句"试用/购买"既多余，
     又容易在审核眼里变成"引导外部购买"。 */
  const showLicenseChip =
    !!license && license.channel === 'direct' && license.status !== 'licensed';

  // Rust 把错误拍平为英文原文返回；这里做一层映射，转成当前语言的措辞。
  // 查不到映射时回退原文 —— 规范要求失败时展示底层真实错误文本，不隐藏细节。
  const label = error
    ? localizeError(error, t)
    : busy
      ? t('status.working')
      : t('status.ready');

  const page = doc ? Math.min(Math.max(current + 1, 1), doc.pageCount) : 0;

  return (
    <div className="statusbar" role="status" aria-live="polite">
      {busy ? <span className="dot-live" aria-hidden="true" /> : null}
      <span className={error ? 'status-text error' : 'status-text'}>{label}</span>

      {doc?.dirty ? <span className="status-text warn">{t('status.unsaved')}</span> : null}
      {doc?.readOnly ? <span className="status-text warn">{t('dsignReady.signed')}</span> : null}
      {selectedCount > 0 ? (
        <span className="status-meta">
          {selectedCount} {t('rail.page')}
        </span>
      ) : null}

      <span className="statusbar-spacer" />

      {doc ? (
        <span className="statusbar-meta">
          {t('status.pageOf', { page, total: doc.pageCount })} · {Math.round(zoom * 100)}% ·{' '}
          {formatBytes(doc.fileSize)}
        </span>
      ) : null}

      {showLicenseChip ? (
        <button type="button" className="badge-license" onClick={onLicenseClick}>
          {license.status === 'expired'
            ? t('license.expiredChip')
            : t('license.trialChip', { days: license.daysLeft })}
        </button>
      ) : null}
      <span className="status-meta">{t('privacy.badge')}</span>
    </div>
  );
}

import { useState } from 'react';
import { useT } from '../i18n';
import { activate, openUrl, type LicenseInfo } from '../services/engine';
import { Modal } from './Modal';

interface LicenseDialogProps {
  info: LicenseInfo | null;
  onRefresh: () => void;
  onClose: () => void;
}

/** 产品页：购买与试用说明的唯一入口，官方站点上的文案以此为准。 */
const BUY_URL = 'https://rocktier.com/pdf-squeeze';

/**
 * 许可与激活。
 *
 * 三种状态对应三套文案，其中 `store` 渠道**不显示激活码输入框** —— 商店版的付费由
 * 微软代收、授权也由商店判定，在这里再摆一个输入框只会让人以为要在别处再买一次
 * （而且会给商店审核留下"引导外部购买"的口实）。
 */
export function LicenseDialog({ info, onRefresh, onClose }: LicenseDialogProps) {
  const t = useT();
  const [code, setCode] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isStore = info?.channel === 'store';
  const licensed = info?.status === 'licensed';
  /* 没有公钥就没人激活得了。如实说明，而不是让付过钱的用户看到"激活码未被接受"。 */
  const canActivate = info?.activationConfigured !== false;

  const submit = async () => {
    if (!code.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await activate(code);
      setCode('');
      onRefresh();
    } catch (e) {
      // 把失败原因照实说清：区分"码不对"和"没连上网"，用户才知道下一步做什么。
      const detail = e instanceof Error ? e.message : String(e);
      setError(detail === 'offline' ? t('license.offline') : t('license.invalid'));
    } finally {
      setBusy(false);
    }
  };

  const statusLine = () => {
    if (!info) return t('license.loading');
    if (licensed) {
      return info.product === 'FL'
        ? t('license.licensedFamily')
        : t('license.licensed');
    }
    if (info.status === 'expired') return t('license.expired');
    return t('license.trialLeft', { days: info.daysLeft });
  };

  return (
    <Modal
      title={t('license.title')}
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose}>
            {t('license.close')}
          </button>
          {!licensed && !isStore ? (
            <button type="button" onClick={() => void openUrl(BUY_URL)}>
              {t('license.buy')}
            </button>
          ) : null}
          {!licensed && !isStore && canActivate ? (
            <button type="button" onClick={() => void submit()} disabled={busy || !code.trim()}>
              {busy ? t('license.activating') : t('license.activate')}
            </button>
          ) : null}
        </>
      }
    >
      <p>{statusLine()}</p>

      {licensed ? (
        <p>
          <small>{t('license.licensedNote')}</small>
        </p>
      ) : !canActivate ? (
        /* 这个构建没有验签公钥：任何回执都验不过。与其让买家以为码错了，不如说清。 */
        <p>
          <small>{t('license.notConfigured')}</small>
        </p>
      ) : isStore ? (
        /* 商店版：说明授权由商店负责，并指向商店页面，不提供任何站外购买入口。 */
        <p>
          <small>{t('license.storeNote')}</small>
        </p>
      ) : (
        <>
          <div className="field">
            <label htmlFor="license-code">{t('license.codeLabel')}</label>
            <input
              id="license-code"
              type="text"
              value={code}
              spellCheck={false}
              autoComplete="off"
              placeholder={t('license.codePlaceholder')}
              onChange={(e) => setCode(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void submit();
              }}
              disabled={busy}
            />
          </div>
          {error ? <p className="error">{error}</p> : null}
          <p>
            <small>{t('license.whereToFind')}</small>
          </p>
          <p>
            <small>{t('license.privacyNote')}</small>
          </p>
        </>
      )}
    </Modal>
  );
}

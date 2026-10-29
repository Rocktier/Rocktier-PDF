import { useCallback, useEffect, useState } from 'react';
import { useT } from '../i18n';
import { useDialogA11y } from '../hooks/useDialog';
import { localizeError } from '../services/errorText';
import { getDefaultCert, listCerts } from '../services/sign';
import type { CertInfo, DocumentInfo } from '../types';
import { SignTabCerts } from './sign/SignTabCerts';
import { SignTabSign } from './sign/SignTabSign';
import { SignTabVerify } from './sign/SignTabVerify';

export type SignTab = 'certs' | 'sign' | 'verify';

export interface SignPlacement {
  page: number;
  x: number;
  y: number;
}

interface SignDialogProps {
  doc: DocumentInfo;
  tab: SignTab;
  onTab: (tab: SignTab) => void;
  placement: SignPlacement | null;
  engineAvailable: boolean;
  onRequestPlacement: () => void;
  onSaveFirst: () => Promise<boolean>;
  onSigned: (info: DocumentInfo) => void;
  onClose: () => void;
}

export function SignDialog(props: SignDialogProps) {
  const { tab, onTab, engineAvailable, onClose, doc, placement, onRequestPlacement, onSaveFirst, onSigned } = props;
  const t = useT();
  const { overlayProps, dialogProps } = useDialogA11y(onClose);
  const [certs, setCerts] = useState<CertInfo[]>([]);
  const [defaultCert, setDefaultCert] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setCerts(await listCerts());
      setDefaultCert(await getDefaultCert());
    } catch (e) {
      setError(localizeError(String(e), t));
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="overlay" {...overlayProps}>
      <div
        className="modal"
        style={{ width: 'min(720px, 94vw)' }}
        {...dialogProps}
        aria-label={t('dsign.title')}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h3>{t('dsign.title')}</h3>
          <button className="btn btn-icon" onClick={onClose} aria-label={t('dsign.close')}>
            ×
          </button>
        </div>

        <div className="row" role="tablist" style={{ padding: '0 16px' }}>
          {(['certs', 'sign', 'verify'] as SignTab[]).map((key) => (
            <button
              key={key}
              role="tab"
              aria-selected={tab === key}
              className={`btn${tab === key ? ' btn-primary' : ''}`}
              onClick={() => onTab(key)}
            >
              {t(`dsign.tab.${key}`)}
            </button>
          ))}
        </div>

        {!engineAvailable ? (
          <p className="error" style={{ margin: '12px 16px' }}>
            {t('dsign.engineMissing')}
          </p>
        ) : null}

        <div className="modal-body">
          {error ? <div className="error">{error}</div> : null}
          {/* 证书页和签名页保持挂载、用 hidden 隐藏：条件卸载会清掉用户已填的
              签名输入（签署人姓名 / 输出路径 / 外观图片）。hidden 属性对屏幕
              阅读器等价于移除，无需额外 aria 处理。验证页无用户状态，维持条件
              挂载（也避免打开对话框时自动跑最多 60s 的主线程验证）。 */}
          <div hidden={tab !== 'certs'}>
            <SignTabCerts
              certs={certs}
              defaultCert={defaultCert}
              busy={busy}
              setBusy={setBusy}
              onChanged={load}
              onError={setError}
            />
          </div>
          <div hidden={tab !== 'sign'}>
            <SignTabSign
              doc={doc}
              placement={placement}
              certs={certs}
              defaultCert={defaultCert}
              busy={busy}
              setBusy={setBusy}
              onRequestPlacement={onRequestPlacement}
              onSaveFirst={onSaveFirst}
              onSigned={onSigned}
              onError={setError}
            />
          </div>
          {tab === 'verify' ? <SignTabVerify path={doc.path} onError={setError} /> : null}
        </div>
      </div>
    </div>
  );
}

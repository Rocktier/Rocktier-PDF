import { useCallback, useEffect, useState } from 'react';
import { useT } from '../../i18n';
import { localizeError } from '../../services/errorText';
import { verifyPdf } from '../../services/sign';
import type { VerifyReport } from '../../types';

interface Props {
  path: string;
  onError: (message: string | null) => void;
}

export function SignTabVerify({ path, onError }: Props) {
  const t = useT();
  const [report, setReport] = useState<VerifyReport | null>(null);
  const [engineError, setEngineError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = useCallback(async () => {
    if (!path) return;
    setBusy(true);
    setEngineError(null);
    onError(null);
    try {
      setReport(await verifyPdf(path));
    } catch (e) {
      setReport(null);
      setEngineError(localizeError(String(e), t));
    } finally {
      setBusy(false);
    }
  }, [onError, path, t]);

  useEffect(() => {
    void run();
  }, [run]);

  const note = (() => {
    if (!report) return null;
    switch (report.verdict) {
      case 'VALID':
        return t('dsign.verify.validNote');
      case 'UNTRUSTED':
        return t('dsign.verify.untrustedNote');
      case 'INVALID':
        return t('dsign.verify.invalidNote');
      case 'NO_SIGNATURES':
        return t('dsign.verify.noneNote');
      default:
        return null;
    }
  })();

  // 只有 INVALID 是故障色。UNTRUSTED（自签名/私有 CA）与 NO_SIGNATURES 都是
  // 正常结论，用普通提示色，不要把它们画成错误 —— 那会教用户忽略红色。
  const tone = report?.verdict === 'INVALID' ? 'error' : 'hint';

  return (
    <div>
      <div className="row">
        <button className="btn" disabled={busy || !path} onClick={() => void run()}>
          {busy ? t('dsign.verify.running') : t('dsign.verify.run')}
        </button>
      </div>

      {engineError ? <div className="error">{engineError}</div> : null}

      {report ? (
        <>
          <div className={tone}>{report.summary}</div>
          {note ? <p className="muted">{note}</p> : null}
          {report.signers.length > 0 ? (
            <table className="verify-table">
              <thead>
                <tr>
                  <th>{t('dsign.verify.signer')}</th>
                  <th>{t('dsign.verify.time')}</th>
                  <th>{t('dsign.verify.statusLabel')}</th>
                  <th>{t('dsign.verify.trusted')}</th>
                </tr>
              </thead>
              <tbody>
                {report.signers.map((signer, index) => (
                  <tr key={`${signer.name}:${index}`}>
                    <td>{signer.name || '—'}</td>
                    <td>{signer.signatureTime || '—'}</td>
                    <td>{t(`dsign.verify.status.${signer.status}`)}</td>
                    <td>{signer.trustedIssuer ? t('dsign.verify.yes') : t('dsign.verify.no')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : null}
        </>
      ) : null}
    </div>
  );
}

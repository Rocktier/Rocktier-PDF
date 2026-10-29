import { useState } from 'react';
import { useT } from '../../i18n';
import { localizeError } from '../../services/errorText';
import {
  allowExpiredCert,
  deleteCert,
  generateCert,
  importCert,
  inspectCertFiles,
  readAsLatin1,
  setDefaultCert,
} from '../../services/sign';
import type { CertInfo, CertMeta } from '../../types';

interface Props {
  certs: CertInfo[];
  defaultCert: string | null;
  busy: boolean;
  setBusy: (busy: boolean) => void;
  onChanged: () => Promise<void>;
  onError: (message: string | null) => void;
}

export function SignTabCerts({ certs, defaultCert, busy, setBusy, onChanged, onError }: Props) {
  const t = useT();
  const [newName, setNewName] = useState('');
  const [keyName, setKeyName] = useState<string | null>(null);
  const [keyData, setKeyData] = useState<string | null>(null);
  const [certName, setCertName] = useState<string | null>(null);
  const [certData, setCertData] = useState<string | null>(null);
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [preview, setPreview] = useState<CertMeta | null>(null);

  const pickFile = async (
    file: File | null,
    set: (name: string | null, data: string | null) => void
  ) => {
    setPreview(null);
    if (!file) {
      set(null, null);
      return;
    }
    set(file.name, await readAsLatin1(file));
    if (/\.(p12|pfx)$/i.test(file.name)) setShowPassword(true);
  };

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    onError(null);
    try {
      await fn();
    } catch (e) {
      onError(localizeError(String(e), t));
    } finally {
      setBusy(false);
    }
  };

  const handlePreview = () =>
    run(async () => {
      if (!keyData) return;
      try {
        const meta = await inspectCertFiles(keyData, certData ?? undefined, password || undefined);
        setPreview(meta);
        setShowPassword(false);
        if (!newName.trim()) setNewName(meta.cn);
      } catch (e) {
        const raw = String(e);
        setPreview(null);
        if (raw.includes('CERT_IMPORT_PASSWORD_WRONG')) {
          setShowPassword(true);
          onError(t('dsign.cert.error.passwordWrong'));
        } else if (raw.includes('CERT_IMPORT_PASSWORD_REQUIRED')) {
          setShowPassword(true);
          onError(t('dsign.cert.error.passwordRequired'));
        } else if (raw.includes('CERT_KEY_MISMATCH')) {
          onError(t('dsign.cert.error.keyMismatch'));
        } else {
          onError(localizeError(raw, t));
        }
      }
    });

  const handleImport = () =>
    run(async () => {
      const name = newName.trim();
      if (!name || !keyData || !preview) return;
      const first = certs.length === 0;
      await importCert(name, keyData, certData ?? undefined, password || undefined);
      if (first) await setDefaultCert(name);
      setKeyName(null);
      setKeyData(null);
      setCertName(null);
      setCertData(null);
      setPassword('');
      setShowPassword(false);
      setPreview(null);
      setNewName('');
      await onChanged();
    });

  const handleGenerate = () =>
    run(async () => {
      const name = newName.trim();
      if (!name) return;
      await generateCert(name);
      setNewName('');
      await onChanged();
    });

  const handleDelete = (name: string) =>
    run(async () => {
      if (!window.confirm(t('dsign.cert.deleteConfirm', { name }))) return;
      await deleteCert(name);
      await onChanged();
    });

  return (
    <div>
      <div className="field">
        <label>{t('dsign.cert.heading')}</label>
        <p className="muted">{t('dsign.cert.intro')}</p>
        <div className="row">
          <input
            className="input"
            value={newName}
            placeholder={t('dsign.cert.namePlaceholder')}
            onChange={(e) => setNewName(e.target.value)}
          />
          <button className="btn" disabled={!newName.trim() || busy} onClick={() => void handleGenerate()}>
            {t('dsign.cert.generate')}
          </button>
        </div>
      </div>

      <div className="field">
        <label>{t('dsign.cert.importKey')}</label>
        <div className="row">
          <input
            type="file"
            accept=".pem,.key,.p12,.pfx,.der,.crt,.cer"
            aria-label={t('dsign.cert.importKey')}
            title={keyName ?? t('dsign.cert.importKey')}
            onChange={(e) =>
              void pickFile(e.target.files?.[0] ?? null, (n, d) => {
                setKeyName(n);
                setKeyData(d);
              })
            }
          />
          <input
            type="file"
            accept=".pem,.crt,.cer,.der"
            aria-label={t('dsign.cert.importCert')}
            title={certName ?? t('dsign.cert.importCert')}
            onChange={(e) =>
              void pickFile(e.target.files?.[0] ?? null, (n, d) => {
                setCertName(n);
                setCertData(d);
              })
            }
          />
        </div>
        {showPassword ? (
          <input
            className="input"
            type="password"
            placeholder={t('dsign.cert.password')}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        ) : null}
        <div className="row">
          <button className="btn" disabled={!keyData || busy} onClick={() => void handlePreview()}>
            {t('dsign.cert.preview')}
          </button>
          <button
            className="btn btn-primary"
            disabled={!preview || !newName.trim() || busy}
            onClick={() => void handleImport()}
          >
            {t('dsign.cert.import')}
          </button>
        </div>
        {preview ? (
          <p className="muted">
            {preview.subject} · {t('dsign.cert.issuer')}: {preview.issuer} ·{' '}
            {t('dsign.cert.validUntil')}: {new Date(preview.notAfter).toLocaleDateString()} ·{' '}
            {preview.keyType} · {t('dsign.cert.chain')}: {preview.chainLen}
          </p>
        ) : null}
      </div>

      {certs.length === 0 ? (
        <p className="muted">{t('dsign.cert.empty')}</p>
      ) : (
        <ul className="cert-list" style={{ listStyle: 'none', padding: 0 }}>
          {certs.map((cert) => {
            const isDefault = defaultCert === cert.name;
            return (
              <li key={cert.name} className="cert-row">
                <strong>{cert.name}</strong>
                {isDefault ? <span className="cert-badge">{t('dsign.cert.default')}</span> : null}
                {cert.meta?.expired ? (
                  <span className="cert-badge">{t('dsign.cert.expired')}</span>
                ) : null}
                <span className="muted">
                  {cert.meta?.notAfter
                    ? `${t('dsign.cert.validUntil')}: ${new Date(cert.meta.notAfter).toLocaleDateString()}`
                    : ''}
                </span>
                <span style={{ flex: 1 }} />
                {cert.meta?.expired && !cert.meta.allowExpired ? (
                  <button
                    className="btn btn-sm"
                    disabled={busy}
                    onClick={() => void run(async () => {
                      await allowExpiredCert(cert.name);
                      await onChanged();
                    })}
                  >
                    {t('dsign.cert.allowExpired')}
                  </button>
                ) : null}
                {!isDefault ? (
                  <button
                    className="btn btn-sm"
                    disabled={busy}
                    onClick={() => void run(async () => {
                      await setDefaultCert(cert.name);
                      await onChanged();
                    })}
                  >
                    {t('dsign.cert.setDefault')}
                  </button>
                ) : null}
                <button className="btn btn-sm" disabled={busy} onClick={() => void handleDelete(cert.name)}>
                  {t('dsign.cert.delete')}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

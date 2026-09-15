import { useT } from '../i18n';
import { formatBytes } from '../services/engine';
import type { DocumentInfo } from '../types';

interface StatusBarProps {
  doc: DocumentInfo | null;
  busy: boolean;
  error: string | null;
  selectedCount: number;
}

export function StatusBar({ doc, busy, error, selectedCount }: StatusBarProps) {
  const t = useT();

  const state = error ? 'error' : busy ? 'busy' : 'ready';
  const label = error ?? (busy ? t('status.working') : t('status.ready'));

  return (
    <div className="statusbar">
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
        </>
      ) : null}

      <div style={{ flex: 1 }} />
      <span>{t('privacy.badge')}</span>
    </div>
  );
}

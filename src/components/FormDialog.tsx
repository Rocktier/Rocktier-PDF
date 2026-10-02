import { useEffect, useState } from 'react';
import { useT } from '../i18n';
import { useDialogA11y } from '../hooks/useDialog';
import { listFormFields, setFormValues } from '../services/engine';
import type { FormFieldInfo, DocumentInfo } from '../types';

interface FormDialogProps {
  onClose: () => void;
  /** Called after a successful write so the viewer can adopt the new doc state. */
  onApplied: (info: DocumentInfo) => void;
}

/** Lists every fillable AcroForm field and writes the edits back. */
export function FormDialog({ onClose, onApplied }: FormDialogProps) {
  const t = useT();
  const { overlayProps, dialogProps } = useDialogA11y(onClose);
  const [fields, setFields] = useState<FormFieldInfo[]>([]);
  const [draft, setDraft] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listFormFields()
      .then((list) => {
        setFields(list);
        const initial: Record<string, string> = {};
        for (const field of list) initial[field.name] = field.value;
        setDraft(initial);
      })
      .catch((e) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, []);

  const apply = async () => {
    const changed: [string, string][] = fields
      .filter((f) => draft[f.name] !== f.value)
      .map((f) => [f.name, draft[f.name]]);

    if (changed.length === 0) {
      onClose();
      return;
    }

    setBusy(true);
    setError(null);
    try {
      const info = await setFormValues(changed);
      onApplied(info);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const editable = (kind: string) => kind === 'text' || kind === 'checkbox' || kind === 'radio';

  return (
    <div className="overlay" {...overlayProps}>
      <div className="modal" {...dialogProps} aria-label={t('form.title')} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{t('form.title')}</h3>
        </div>

        <div className="modal-body">
          {loading ? <p className="hint">{t('form.loading')}</p> : null}
          {!loading && fields.length === 0 ? <p className="hint">{t('form.empty')}</p> : null}

          {fields.map((field, i) => (
            <div className="field" key={`${field.page}:${field.name}:${i}`}>
              <label>
                {field.name}
                <span className="muted" style={{ marginLeft: 6, textTransform: 'none' }}>
                  {t('form.page', { n: field.page + 1 })}
                </span>
              </label>

              {field.kind === 'checkbox' || field.kind === 'radio' ? (
                <input
                  type="checkbox"
                  style={{ justifySelf: 'start' }}
                  checked={draft[field.name] === 'true'}
                  onChange={(e) =>
                    setDraft({ ...draft, [field.name]: e.target.checked ? 'true' : 'false' })
                  }
                />
              ) : field.kind === 'text' ? (
                <input
                  className="input"
                  value={draft[field.name] ?? ''}
                  onChange={(e) => setDraft({ ...draft, [field.name]: e.target.value })}
                />
              ) : (
                <p className="hint">{editable(field.kind) ? draft[field.name] : draft[field.name] || '—'}</p>
              )}
            </div>
          ))}

          {error ? (
            <p className="hint" style={{ color: 'var(--danger)' }}>
              {error}
            </p>
          ) : null}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose} disabled={busy}>
            {t('form.cancel')}
          </button>
          <button
            className="btn btn-primary"
            onClick={apply}
            disabled={busy || loading || fields.length === 0}
          >
            {t('form.apply')}
          </button>
        </div>
      </div>
    </div>
  );
}

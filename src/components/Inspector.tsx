import { useT } from '../i18n';
import { formatBytes } from '../services/engine';
import type { DocumentInfo } from '../types';
import {
  IconCompress,
  IconCopy,
  IconExportImages,
  IconExtract,
  IconForm,
  IconImagesToPdf,
  IconLock,
  IconMarkdown,
  IconSign,
  IconStamp,
  IconTrash,
} from './Logo';

interface InspectorProps {
  doc: DocumentInfo;
  /** 当前可见页（0 起），只用于显示这一页的真实尺寸。 */
  current: number;
  selectedCount: number;
  busy: boolean;
  signEngineAvailable: boolean;
  onSign: () => void;
  onVerify: () => void;
  onSaveEditableCopy: () => void;
  onDelete: () => void;
  onExtract: () => void;
  onCopyText: () => void;
  onCompress: () => void;
  onSecurity: () => void;
  onForm: () => void;
  onMarkdown: () => void;
  onStamp: () => void;
  onExportImages: () => void;
  onImagesToPdf: () => void;
}

/**
 * 右侧检查器：只陈述真实状态（页数、大小、当前页尺寸、只读/可编辑、
 * 有没有数字签名），并把次级工具收在这里。签名/验证的入口在这里，
 * 但流程仍由 App 打开原来的签名对话框 —— 这里不碰任何签名逻辑。
 */
export function Inspector({
  doc,
  current,
  selectedCount,
  busy,
  signEngineAvailable,
  onSign,
  onVerify,
  onSaveEditableCopy,
  onDelete,
  onExtract,
  onCopyText,
  onCompress,
  onSecurity,
  onForm,
  onMarkdown,
  onStamp,
  onExportImages,
  onImagesToPdf,
}: InspectorProps) {
  const t = useT();
  const page = doc.pages[Math.min(Math.max(current, 0), doc.pages.length - 1)];
  const engineTitle = signEngineAvailable ? undefined : t('dsign.engineMissing');

  return (
    <aside className="inspector" aria-label={t('inspector.label')}>
      <section>
        <h2>{t('inspector.document')}</h2>
        <dl className="kv">
          <div className="kv-row">
            <dt>{t('inspector.pages')}</dt>
            <dd>{doc.pageCount}</dd>
          </div>
          <div className="kv-row">
            <dt>{t('inspector.size')}</dt>
            <dd>{formatBytes(doc.fileSize)}</dd>
          </div>
          <div className="kv-row">
            <dt>{t('inspector.pageSize')}</dt>
            <dd>{page ? `${Math.round(page.width)} × ${Math.round(page.height)} pt` : '—'}</dd>
          </div>
          <div className="kv-row">
            <dt>{t('inspector.state')}</dt>
            <dd>{doc.readOnly ? t('inspector.readOnly') : t('inspector.editable')}</dd>
          </div>
        </dl>
      </section>

      <div className="inspector-rule" />

      <section>
        <h2>{t('inspector.signature')}</h2>
        <div className="signcard">
          <div className="signcard-head">
            <IconSign />
            <span>{t('inspector.signature')}</span>
            <span className="signcard-badge">
              {doc.hasCryptographicSignature
                ? t('inspector.signedBadge')
                : t('inspector.unsignedBadge')}
            </span>
          </div>
          <p>
            {doc.hasCryptographicSignature
              ? t('inspector.signedBody')
              : t('inspector.unsignedBody')}
          </p>
          <div className="signcard-actions">
            <button
              type="button"
              className="btn btn-sm btn-primary"
              onClick={onSign}
              disabled={busy || !signEngineAvailable}
              title={engineTitle}
            >
              {t('inspector.sign')}
            </button>
            <button
              type="button"
              className="btn btn-sm"
              onClick={onVerify}
              disabled={busy || !signEngineAvailable}
              title={engineTitle}
            >
              {t('inspector.verify')}
            </button>
            {doc.readOnly ? (
              <button
                type="button"
                className="btn btn-sm"
                onClick={onSaveEditableCopy}
                disabled={busy}
              >
                {t('dsignReady.saveCopy')}
              </button>
            ) : null}
          </div>
        </div>
        <p className="inspector-note">{t('inspector.offlineNote')}</p>
      </section>

      <div className="inspector-rule" />

      <section>
        <h2>{t('inspector.selection')}</h2>
        <p className="inspector-note">
          {selectedCount > 0
            ? t('inspector.selectionCount', { count: selectedCount })
            : t('inspector.noSelection')}
        </p>
        <div className="inspector-actions">
          <button
            type="button"
            className="btn btn-sm"
            onClick={onExtract}
            disabled={busy || selectedCount === 0}
          >
            <IconExtract />
            {t('toolbar.extractPages')}
          </button>
          <button
            type="button"
            className="btn btn-sm"
            onClick={onCopyText}
            disabled={busy || selectedCount === 0}
          >
            <IconCopy />
            {t('toolbar.copyText')}
          </button>
          <button
            type="button"
            className="btn btn-sm btn-danger"
            onClick={onDelete}
            disabled={busy || selectedCount === 0}
          >
            <IconTrash />
            {t('toolbar.delete')}
          </button>
        </div>
      </section>

      <div className="inspector-rule" />

      <section>
        <h2>{t('inspector.tools')}</h2>
        <div className="tool-list">
          <button className="tool-link" type="button" onClick={onCompress} disabled={busy}>
            <IconCompress />
            <span>{t('toolbar.compress')}</span>
          </button>
          <button className="tool-link" type="button" onClick={onSecurity} disabled={busy}>
            <IconLock />
            <span>{t('toolbar.security')}</span>
          </button>
          <button className="tool-link" type="button" onClick={onForm} disabled={busy}>
            <IconForm />
            <span>{t('toolbar.fillForm')}</span>
          </button>
          <button className="tool-link" type="button" onClick={onMarkdown} disabled={busy}>
            <IconMarkdown />
            <span>{t('toolbar.markdown')}</span>
          </button>
          <button className="tool-link" type="button" onClick={onStamp} disabled={busy}>
            <IconStamp />
            <span>{t('toolbar.stamp')}</span>
          </button>
          <button className="tool-link" type="button" onClick={onExportImages} disabled={busy}>
            <IconExportImages />
            <span>{t('toolbar.exportImages')}</span>
          </button>
          <button className="tool-link" type="button" onClick={onImagesToPdf} disabled={busy}>
            <IconImagesToPdf />
            <span>{t('toolbar.imagesToPdf')}</span>
          </button>
        </div>
      </section>
    </aside>
  );
}

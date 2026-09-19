import { useI18n, useT } from '../i18n';
import type { AnnotTool, DocumentInfo } from '../types';
import {
  IconCopy,
  IconExportImages,
  IconExtract,
  IconForm,
  IconHighlight,
  IconImagesToPdf,
  IconLock,
  IconMerge,
  IconMinus,
  IconMoon,
  IconNote,
  IconOpen,
  IconPlus,
  IconRedo,
  IconRotateLeft,
  IconRotateRight,
  IconSave,
  IconUndo,
  IconSign,
  IconSplit,
  IconStamp,
  IconStrikeout,
  IconSun,
  IconTrash,
  IconUnderline,
  Logo,
} from './Logo';

interface ToolbarProps {
  doc: DocumentInfo | null;
  busy: boolean;
  selectedCount: number;
  zoom: number;
  theme: 'dark' | 'light';
  markupTool: AnnotTool | null;
  onMarkupTool: (tool: AnnotTool | null) => void;
  onOpen: () => void;
  onSave: () => void;
  onSaveAs: () => void;
  onMerge: () => void;
  onSplit: () => void;
  onExtract: () => void;
  onCopyText: () => void;
  onStamp: () => void;
  onExportImages: () => void;
  onImagesToPdf: () => void;
  onSign: () => void;
  onSecurity: () => void;
  onCompress: () => void;
  onForm: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onRotate: (degrees: number) => void;
  onDelete: () => void;
  onZoom: (zoom: number) => void;
  onToggleTheme: () => void;
}

const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3];

export function Toolbar({
  doc,
  busy,
  selectedCount,
  zoom,
  theme,
  markupTool,
  onMarkupTool,
  onOpen,
  onSave,
  onSaveAs,
  onMerge,
  onSplit,
  onExtract,
  onCopyText,
  onStamp,
  onExportImages,
  onImagesToPdf,
  onSign,
  onSecurity,
  onCompress,
  onForm,
  onUndo,
  onRedo,
  onRotate,
  onDelete,
  onZoom,
  onToggleTheme,
}: ToolbarProps) {
  const t = useT();
  const { lang, setLang } = useI18n();
  const hasSelection = selectedCount > 0;

  const stepZoom = (direction: 1 | -1) => {
    const index = ZOOM_STEPS.findIndex((z) => Math.abs(z - zoom) < 0.001);
    if (index === -1) {
      // Current zoom is off-preset: snap to the nearest step in that direction.
      const next = direction === 1 ? ZOOM_STEPS.find((z) => z > zoom) : [...ZOOM_STEPS].reverse().find((z) => z < zoom);
      if (next) onZoom(next);
      return;
    }
    const next = ZOOM_STEPS[Math.min(ZOOM_STEPS.length - 1, Math.max(0, index + direction))];
    if (next !== zoom) onZoom(next);
  };

  return (
    <div className="toolbar">
      <div className="brand">
        <Logo />
        <span className="brand-name">
          Rocktier PDF Editor<span className="dot">.</span>
        </span>
      </div>

      <div className="toolbar-sep" />

      <button className="btn" onClick={onOpen} disabled={busy}>
        <IconOpen />
        {t('toolbar.open')}
      </button>

      <button className="btn" onClick={onSave} disabled={!doc || busy}>
        <IconSave />
        {t('toolbar.save')}
      </button>
      <button className="btn" onClick={onSaveAs} disabled={!doc || busy} title={t('toolbar.saveAs')}>
        {t('toolbar.saveAs')}
      </button>

      <div className="toolbar-sep" />

      <button className="btn btn-icon" onClick={onUndo} disabled={!doc || busy} title={t('toolbar.undo')}>
        <IconUndo />
      </button>
      <button className="btn btn-icon" onClick={onRedo} disabled={!doc || busy} title={t('toolbar.redo')}>
        <IconRedo />
      </button>

      <div className="toolbar-sep" />

      <button className="btn" onClick={onMerge} disabled={busy}>
        <IconMerge />
        {t('toolbar.merge')}
      </button>
      <button className="btn" onClick={onSplit} disabled={!doc || busy}>
        <IconSplit />
        {t('toolbar.split')}
      </button>

      <div className="toolbar-sep" />

      <button className="btn" onClick={() => onRotate(-90)} disabled={!doc || busy}>
        <IconRotateLeft />
      </button>
      <button className="btn" onClick={() => onRotate(90)} disabled={!doc || busy}>
        <IconRotateRight />
      </button>
      <button className="btn btn-danger" onClick={onDelete} disabled={!doc || busy || !hasSelection}>
        <IconTrash />
        {t('toolbar.delete')}
      </button>
      <button className="btn" onClick={onExtract} disabled={!doc || busy || !hasSelection}>
        <IconExtract />
        {t('toolbar.extract')}
      </button>
      <button className="btn btn-icon" onClick={onCopyText} disabled={!doc || busy || !hasSelection} title={t('toolbar.copyText')}>
        <IconCopy />
      </button>
      <button className="btn" onClick={onStamp} disabled={!doc || busy}>
        <IconStamp />
        {t('stamp.pageNumbers')}
      </button>
      <button className="btn" onClick={onExportImages} disabled={!doc || busy}>
        <IconExportImages />
        {t('toolbar.exportImages')}
      </button>
      <button className="btn" onClick={onImagesToPdf} disabled={busy}>
        <IconImagesToPdf />
        {t('toolbar.imagesToPdf')}
      </button>

      <div className="toolbar-sep" />

      <button
        className={`btn btn-icon${markupTool === 'highlight' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'highlight' ? null : 'highlight')}
        disabled={!doc || busy}
        title={t('markup.highlight')}
      >
        <IconHighlight />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'underline' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'underline' ? null : 'underline')}
        disabled={!doc || busy}
        title={t('markup.underline')}
      >
        <IconUnderline />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'strikeout' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'strikeout' ? null : 'strikeout')}
        disabled={!doc || busy}
        title={t('markup.strikeout')}
      >
        <IconStrikeout />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'note' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'note' ? null : 'note')}
        disabled={!doc || busy}
        title={t('markup.note')}
      >
        <IconNote />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'sign' ? ' active' : ''}`}
        onClick={onSign}
        disabled={!doc || busy}
        title={t('toolbar.sign')}
      >
        <IconSign />
      </button>
      <button className="btn btn-icon" onClick={onSecurity} disabled={!doc || busy} title={t('security.title')}>
        <IconLock />
      </button>
      <button className="btn btn-icon" onClick={onCompress} disabled={!doc || busy} title={t('security.title')}>
        <IconSave />
      </button>
      <button className="btn btn-icon" onClick={onForm} disabled={!doc || busy} title={t('form.title')}>
        <IconForm />
      </button>

      <div className="toolbar-spacer" />

      <button className="btn btn-icon" onClick={() => stepZoom(-1)} disabled={!doc} title={t('zoom.out')}>
        <IconMinus />
      </button>
      <span className="mono muted" style={{ minWidth: 44, textAlign: 'center', fontSize: 11 }}>
        {Math.round(zoom * 100)}%
      </span>
      <button className="btn btn-icon" onClick={() => stepZoom(1)} disabled={!doc} title={t('zoom.in')}>
        <IconPlus />
      </button>

      <div className="toolbar-sep" />

      <button
        className="btn btn-icon lang-toggle"
        onClick={() => setLang(lang === 'en' ? 'zh' : 'en')}
        title={t('toolbar.language')}
      >
        {lang === 'en' ? 'EN' : '中'}
      </button>

      <button className="btn btn-icon" onClick={onToggleTheme} title={t('toolbar.theme')}>
        {theme === 'dark' ? <IconSun /> : <IconMoon />}
      </button>
    </div>
  );
}

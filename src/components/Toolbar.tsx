import { useI18n, useT } from '../i18n';
import type { AnnotTool, DocumentInfo } from '../types';
import {
  IconCompress,
  IconCopy,
  IconExportImages,
  IconExtract,
  IconForm,
  IconHighlight,
  IconImagesToPdf,
  IconLock,
  IconMarkdown,
  IconMerge,
  IconMinus,
  IconMoon,
  IconNote,
  IconOpen,
  IconPlus,
  IconRedact,
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
  theme: 'auto' | 'light' | 'dark';
  markupTool: AnnotTool | null;
  onMarkupTool: (tool: AnnotTool | null) => void;
  redactActive: boolean;
  onRedactTool: () => void;
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
  onMarkdown: () => void;
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
  redactActive,
  onRedactTool,
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
  onMarkdown,
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
          Rocktier PDF<span className="dot-live" aria-hidden="true" />
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

      <button className="btn btn-icon" onClick={onUndo} disabled={!doc || busy} title={t('toolbar.undo')}
        aria-label={t('toolbar.undo')}>
        <IconUndo />
      </button>
      <button className="btn btn-icon" onClick={onRedo} disabled={!doc || busy} title={t('toolbar.redo')}
        aria-label={t('toolbar.redo')}>
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

      <button
        className="btn btn-icon"
        onClick={() => onRotate(-90)}
        disabled={!doc || busy}
        title={t('toolbar.rotateLeft')}
        aria-label={t('toolbar.rotateLeft')}
      >
        <IconRotateLeft />
      </button>
      <button
        className="btn btn-icon"
        onClick={() => onRotate(90)}
        disabled={!doc || busy}
        title={t('toolbar.rotateRight')}
        aria-label={t('toolbar.rotateRight')}
      >
        <IconRotateRight />
      </button>
      <button className="btn btn-danger" onClick={onDelete} disabled={!doc || busy || !hasSelection}>
        <IconTrash />
        {t('toolbar.delete')}
      </button>
      <button className="btn" onClick={onExtract} disabled={!doc || busy || !hasSelection}>
        <IconExtract />
        {t('toolbar.extractPages')}
      </button>
      <button className="btn" onClick={onCopyText} disabled={!doc || busy || !hasSelection}>
        <IconCopy />
        {t('toolbar.copyText')}
      </button>

      {/*路线图 R4：家族唯一的差异化能力（iLovePDF 纯云端、PDF24 在
          macOS 上只能上传），所以给足权重—— 图标 + 文字 + 强调色。
          它也是这一组里唯一「输出到另一种格式」而非「编辑当前页」的动作，
          故单独分组。 */}
      <div className="toolbar-sep" />

      <button
        className="btn btn-primary"
        onClick={onMarkdown}
        disabled={!doc || busy}
        title={t('toolbar.markdown')}
      >
        <IconMarkdown />
        {t('toolbar.markdown')}
      </button>

      <div className="toolbar-sep" />

      {/* 以下都是低频动作 —— 按家族规则「低频用纯图标」收窄，
          每个都必须带 title + aria-label，否则图标成了黑盒。
          纯图标比带字省约 50px，这一组省出的宽度正是上面能塞进
          Markdown 按钮的原因（.toolbar 曾因带字按钮过宽而换行）。 */}
      <button
        className="btn btn-icon"
        onClick={onStamp}
        disabled={!doc || busy}
        title={t('toolbar.stamp')}
        aria-label={t('toolbar.stamp')}
      >
        <IconStamp />
      </button>
      <button
        className="btn btn-icon"
        onClick={onExportImages}
        disabled={!doc || busy}
        title={t('toolbar.exportImages')}
        aria-label={t('toolbar.exportImages')}
      >
        <IconExportImages />
      </button>
      <button
        className="btn btn-icon"
        onClick={onImagesToPdf}
        disabled={busy}
        title={t('toolbar.imagesToPdf')}
        aria-label={t('toolbar.imagesToPdf')}
      >
        <IconImagesToPdf />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'sign' ? ' active' : ''}`}
        onClick={onSign}
        disabled={!doc || busy}
        // P0-15：功能入口处如实声明这是视觉图章，不是数字签名。
        title={t('sign.tooltip')}
        aria-label={t('toolbar.sign')}
      >
        <IconSign />
      </button>
      <button
        className="btn btn-icon"
        onClick={onSecurity}
        disabled={!doc || busy}
        title={t('toolbar.security')}
        aria-label={t('toolbar.security')}
      >
        <IconLock />
      </button>
      <button
        className="btn btn-icon"
        onClick={onCompress}
        disabled={!doc || busy}
        title={t('toolbar.compress')}
        aria-label={t('toolbar.compress')}
      >
        <IconCompress />
      </button>
      <button
        className="btn btn-icon"
        onClick={onForm}
        disabled={!doc || busy}
        title={t('toolbar.fillForm')}
        aria-label={t('toolbar.fillForm')}
      >
        <IconForm />
      </button>

      <div className="toolbar-sep" />

      <button
        className={`btn btn-icon${markupTool === 'highlight' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'highlight' ? null : 'highlight')}
        disabled={!doc || busy}
        title={t('markup.highlight')}
        aria-label={t('markup.highlight')}
      >
        <IconHighlight />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'underline' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'underline' ? null : 'underline')}
        disabled={!doc || busy}
        title={t('markup.underline')}
        aria-label={t('markup.underline')}
      >
        <IconUnderline />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'strikeout' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'strikeout' ? null : 'strikeout')}
        disabled={!doc || busy}
        title={t('markup.strikeout')}
        aria-label={t('markup.strikeout')}
      >
        <IconStrikeout />
      </button>
      <button
        className={`btn btn-icon${markupTool === 'note' ? ' active' : ''}`}
        onClick={() => onMarkupTool(markupTool === 'note' ? null : 'note')}
        disabled={!doc || busy}
        title={t('markup.note')}
        aria-label={t('markup.note')}
      >
        <IconNote />
      </button>
      <button
        className={`btn btn-icon${redactActive ? ' active btn-danger' : ''}`}
        onClick={onRedactTool}
        disabled={!doc || busy}
        title={t('redact.toolbar')}
        aria-label={t('redact.toolbar')}
      >
        <IconRedact />
      </button>

      <div className="toolbar-spacer" />

      <button className="btn btn-icon" onClick={() => stepZoom(-1)} disabled={!doc} title={t('zoom.out')}
        aria-label={t('zoom.out')}>
        <IconMinus />
      </button>
      <span className="mono muted" style={{ minWidth: 44, textAlign: 'center', fontSize: 11 }}>
        {Math.round(zoom * 100)}%
      </span>
      <button className="btn btn-icon" onClick={() => stepZoom(1)} disabled={!doc} title={t('zoom.in')}
        aria-label={t('zoom.in')}>
        <IconPlus />
      </button>

      <div className="toolbar-sep" />

      <button
        className="btn btn-icon lang-toggle"
        onClick={() => setLang(lang === 'en' ? 'zh' : 'en')}
        title={t('toolbar.language')}
        aria-label={t('toolbar.language')}
      >
        {lang === 'zh' ? 'EN' : '中文'}
      </button>

      {/* 家族唯一主题按钮：.icon-btn（28×28 + 40px 命中区），三态靠 data-mode。
          auto 态显示「屏幕」图标（跟系统走），否则显示将要去往的那一档。 */}
      <button
        className="icon-btn"
        data-mode={theme}
        onClick={onToggleTheme}
        title={`${t('toolbar.theme')} · ${t(`theme.mode.${theme}`)}`}
        aria-label={`${t('toolbar.theme')}: ${t(`theme.mode.${theme}`)}`}
      >
        {theme === 'auto' ? (
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <rect x="2.5" y="4" width="19" height="13" rx="2" />
            <path d="M8 20.5h8M12 17v3.5" />
          </svg>
        ) : theme === 'dark' ? (
          <IconSun />
        ) : (
          <IconMoon />
        )}
      </button>
    </div>
  );
}

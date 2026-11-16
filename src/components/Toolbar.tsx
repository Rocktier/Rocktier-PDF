import { useEffect, useRef, useState } from 'react';
import { useI18n, useT } from '../i18n';
import type { AnnotTool, DocumentInfo } from '../types';
import {
  IconCompress,
  IconExportImages,
  IconForm,
  IconHighlight,
  IconImagesToPdf,
  IconLock,
  IconLongImage,
  IconMarkdown,
  IconMerge,
  IconMinus,
  IconMore,
  IconNote,
  IconOpen,
  IconPlus,
  IconRedact,
  IconRedo,
  IconRotateLeft,
  IconRotateRight,
  IconSave,
  IconSearch,
  IconSign,
  IconSplit,
  IconStamp,
  IconStrikeout,
  IconSun,
  IconUnderline,
  IconUndo,
  Logo,
} from './Logo';

interface ToolbarProps {
  doc: DocumentInfo | null;
  busy: boolean;
  zoom: number;
  theme: 'light' | 'dark';
  markupTool: AnnotTool | null;
  onMarkupTool: (tool: AnnotTool | null) => void;
  redactActive: boolean;
  onRedactTool: () => void;
  onOpen: () => void;
  onSave: () => void;
  onSaveAs: () => void;
  onMerge: () => void;
  onSplit: () => void;
  onStamp: () => void;
  onExportImages: () => void;
  onExportLongImage: () => void;
  onImageify: () => void;
  onImagesToPdf: () => void;
  onSign: () => void;
  /** 数字签名（密码学）—— 与上面那个图片印章是两回事。 */
  onDigitalSign: () => void;
  /** 签名引擎在不在包里：不在就禁用入口，并在 title 里说明原因。 */
  signEngineAvailable: boolean;
  onSecurity: () => void;
  onCompress: () => void;
  onMarkdown: () => void;
  onForm: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onRotate: (degrees: number) => void;
  onFind: () => void;
  onZoom: (zoom: number) => void;
  onToggleTheme: () => void;
}

const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3];

/**
 * 外壳顶部：一行自述（产品名 + 一句话 + 当前文件），一行页面/标注工具。
 *
 * 页面优先：带文字的按钮只留「打开 / 保存 / 另存为 / 数字签名」，
 * 其余低频能力收进「更多」菜单与右侧检查器。这里只决定入口的位置，
 * 所有 action 仍由 App 传入，未接管任何业务逻辑。
 */
export function Toolbar({
  doc,
  busy,
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
  onStamp,
  onExportImages,
  onExportLongImage,
  onImageify,
  onImagesToPdf,
  onSign,
  onDigitalSign,
  signEngineAvailable,
  onSecurity,
  onCompress,
  onMarkdown,
  onForm,
  onUndo,
  onRedo,
  onRotate,
  onFind,
  onZoom,
  onToggleTheme,
}: ToolbarProps) {
  const t = useT();
  const { lang, setLang } = useI18n();
  const [moreOpen, setMoreOpen] = useState(false);
  const moreRef = useRef<HTMLDivElement>(null);

  // 菜单：点外面或按 Esc 收起。没有焦点陷阱 —— 菜单里全是普通按钮，
  // Tab 顺序天然接回工具条。
  useEffect(() => {
    if (!moreOpen) return;
    const onPointer = (e: MouseEvent) => {
      if (!moreRef.current?.contains(e.target as Node)) setMoreOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMoreOpen(false);
    };
    document.addEventListener('mousedown', onPointer);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onPointer);
      document.removeEventListener('keydown', onKey);
    };
  }, [moreOpen]);

  /** 菜单项触发后先收起菜单，动作本身仍走 App 的回调。 */
  const pick = (run: () => void) => () => {
    setMoreOpen(false);
    run();
  };

  const stepZoom = (direction: 1 | -1) => {
    const index = ZOOM_STEPS.findIndex((z) => Math.abs(z - zoom) < 0.001);
    if (index === -1) {
      // Current zoom is off-preset: snap to the nearest step in that direction.
      const next =
        direction === 1
          ? ZOOM_STEPS.find((z) => z > zoom)
          : [...ZOOM_STEPS].reverse().find((z) => z < zoom);
      if (next) onZoom(next);
      return;
    }
    const next = ZOOM_STEPS[Math.min(ZOOM_STEPS.length - 1, Math.max(0, index + direction))];
    if (next !== zoom) onZoom(next);
  };

  const toggleMarkup = (tool: AnnotTool) => () =>
    onMarkupTool(markupTool === tool ? null : tool);

  return (
    <>
      <header className="topbar">
        <div className="brand">
          <Logo size={22} />
          <span className="brand-name">Rocktier PDF</span>
        </div>
        <span className="brand-tagline">{t('tagline')}</span>

        {doc ? (
          <span className="topbar-file" title={doc.path}>
            {doc.name}
          </span>
        ) : null}

        <div className="topbar-spacer" />

        {doc ? (
          <>
            <button className="btn" onClick={onOpen} disabled={busy}>
              <IconOpen />
              {t('toolbar.open')}
            </button>
            {/* 只读（已签名）文档原地保存必然被后端 SIGNED_READ_ONLY 拒掉，
                所以这里禁用；「另存为」保持可用 —— 那正是取可编辑副本的合法路径。 */}
            <button
              className="btn"
              onClick={onSave}
              disabled={busy || doc.readOnly}
              title={doc.readOnly ? t('error.signedReadOnly') : undefined}
            >
              <IconSave />
              {t('toolbar.save')}
            </button>
            <button className="btn btn-quiet" onClick={onSaveAs} disabled={busy}>
              {t('toolbar.saveAs')}
            </button>
            <button
              className="btn btn-primary"
              onClick={onDigitalSign}
              disabled={busy || !signEngineAvailable}
              title={signEngineAvailable ? t('toolbar.digitalSign') : t('dsign.engineMissing')}
            >
              <IconSign />
              {t('toolbar.digitalSign')}
            </button>
          </>
        ) : (
          <button className="btn btn-primary" onClick={onOpen} disabled={busy}>
            <IconOpen />
            {t('toolbar.open')}
          </button>
        )}

        {/* 家族唯一主题按钮：.icon-btn（28×28 + 40px 命中区），两态 light ⇄ dark。
            2026-11-15 二次简化：固定一枚太阳，颜色随主题走（currentColor），
            当前档位由角标 + title/aria 说明。 */}
        <button
          className="icon-btn"
          data-mode={theme}
          onClick={onToggleTheme}
          title={`${t('toolbar.theme')} · ${t(`theme.mode.${theme}`)}`}
          aria-label={`${t('toolbar.theme')}: ${t(`theme.mode.${theme}`)}`}
        >
          <IconSun />
        </button>

        {/* 语言切换：固定一枚地球（2026-11-15 用户令：去掉 A，只留地球），
            en/zh 对切，与主题按钮同款交互。 */}
        <button
          className="icon-btn"
          onClick={() => setLang(lang === 'en' ? 'zh' : 'en')}
          title={t('toolbar.language')}
          aria-label={t('toolbar.language')}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="9" />
            <ellipse cx="12" cy="12" rx="3.8" ry="9" />
            <path d="M3 12h18" />
          </svg>
        </button>
      </header>

      <div className="toolstrip" role="toolbar" aria-label={t('toolbar.toolsLabel')}>
        <button
          className="tool"
          onClick={onUndo}
          disabled={!doc || busy}
          title={t('toolbar.undo')}
          aria-label={t('toolbar.undo')}
        >
          <IconUndo />
        </button>
        <button
          className="tool"
          onClick={onRedo}
          disabled={!doc || busy}
          title={t('toolbar.redo')}
          aria-label={t('toolbar.redo')}
        >
          <IconRedo />
        </button>

        <span className="tool-sep" />

        <button
          className="tool"
          aria-pressed={markupTool === 'highlight'}
          onClick={toggleMarkup('highlight')}
          disabled={!doc || busy}
          title={t('markup.highlight')}
          aria-label={t('markup.highlight')}
        >
          <IconHighlight />
          <span className="tool-label">{t('markup.highlight')}</span>
        </button>
        <button
          className="tool"
          aria-pressed={markupTool === 'underline'}
          onClick={toggleMarkup('underline')}
          disabled={!doc || busy}
          title={t('markup.underline')}
          aria-label={t('markup.underline')}
        >
          <IconUnderline />
          <span className="tool-label">{t('markup.underline')}</span>
        </button>
        <button
          className="tool"
          aria-pressed={markupTool === 'strikeout'}
          onClick={toggleMarkup('strikeout')}
          disabled={!doc || busy}
          title={t('markup.strikeout')}
          aria-label={t('markup.strikeout')}
        >
          <IconStrikeout />
          <span className="tool-label">{t('markup.strikeout')}</span>
        </button>
        <button
          className="tool"
          aria-pressed={markupTool === 'note'}
          onClick={toggleMarkup('note')}
          disabled={!doc || busy}
          title={t('markup.note')}
          aria-label={t('markup.note')}
        >
          <IconNote />
          <span className="tool-label">{t('markup.note')}</span>
        </button>
        {/* 图片印章（视觉图章，不是密码学签名）。数字签名在顶栏与检查器。 */}
        <button
          className="tool"
          aria-pressed={markupTool === 'sign'}
          onClick={onSign}
          disabled={!doc || busy}
          title={t('sign.tooltip')}
          aria-label={t('toolbar.sign')}
        >
          <IconSign />
          <span className="tool-label">{t('toolbar.sign')}</span>
        </button>
        <button
          className="tool tool-danger"
          aria-pressed={redactActive}
          onClick={onRedactTool}
          disabled={!doc || busy}
          title={t('redact.toolbar')}
          aria-label={t('redact.toolbar')}
        >
          <IconRedact />
          <span className="tool-label">{t('redact.toolbar')}</span>
        </button>

        <span className="tool-sep" />

        <button
          className="tool"
          onClick={onFind}
          disabled={!doc || busy}
          title={t('toolbar.find')}
          aria-label={t('toolbar.find')}
        >
          <IconSearch />
        </button>

        <span className="tool-sep" />

        <button
          className="tool"
          onClick={onMerge}
          disabled={busy}
          title={t('toolbar.merge')}
          aria-label={t('toolbar.merge')}
        >
          <IconMerge />
          <span className="tool-label">{t('toolbar.merge')}</span>
        </button>
        <button
          className="tool"
          onClick={onSplit}
          disabled={!doc || busy}
          title={t('toolbar.split')}
          aria-label={t('toolbar.split')}
        >
          <IconSplit />
          <span className="tool-label">{t('toolbar.split')}</span>
        </button>
        <button
          className="tool"
          onClick={() => onRotate(-90)}
          disabled={!doc || busy}
          title={t('toolbar.rotateLeft')}
          aria-label={t('toolbar.rotateLeft')}
        >
          <IconRotateLeft />
        </button>
        <button
          className="tool"
          onClick={() => onRotate(90)}
          disabled={!doc || busy}
          title={t('toolbar.rotateRight')}
          aria-label={t('toolbar.rotateRight')}
        >
          <IconRotateRight />
        </button>

        <span className="tool-sep" />

        {/* 低频工具收进菜单：页码/水印、压缩、密码、表单、Markdown、导出图片、图片转 PDF。 */}
        <div className="menu-wrap" ref={moreRef}>
          <button
            className="tool"
            aria-haspopup="menu"
            aria-expanded={moreOpen}
            onClick={() => setMoreOpen((v) => !v)}
            title={t('toolbar.more')}
            aria-label={t('toolbar.more')}
          >
            <IconMore />
          </button>
          {moreOpen ? (
            <div className="menu" role="menu">
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onStamp)}
                disabled={!doc || busy}
              >
                <IconStamp />
                {t('toolbar.stamp')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onCompress)}
                disabled={!doc || busy}
              >
                <IconCompress />
                {t('toolbar.compress')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onSecurity)}
                disabled={!doc || busy}
              >
                <IconLock />
                {t('toolbar.security')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onForm)}
                disabled={!doc || busy}
              >
                <IconForm />
                {t('toolbar.fillForm')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onMarkdown)}
                disabled={!doc || busy}
              >
                <IconMarkdown />
                {t('toolbar.markdown')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onExportImages)}
                disabled={!doc || busy}
              >
                <IconExportImages />
                {t('toolbar.exportImages')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onExportLongImage)}
                disabled={!doc || busy}
              >
                <IconLongImage />
                {t('toolbar.exportLongImage')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onImageify)}
                disabled={!doc || busy}
              >
                <IconImagesToPdf />
                {t('toolbar.imageify')}
              </button>
              <button
                className="menu-item"
                role="menuitem"
                onClick={pick(onImagesToPdf)}
                disabled={busy}
              >
                <IconImagesToPdf />
                {t('toolbar.imagesToPdf')}
              </button>
            </div>
          ) : null}
        </div>

        <div className="zoom">
          <button
            className="tool"
            onClick={() => stepZoom(-1)}
            disabled={!doc}
            title={t('zoom.out')}
            aria-label={t('zoom.out')}
          >
            <IconMinus />
          </button>
          <span className="zoom-value">{Math.round(zoom * 100)}%</span>
          <button
            className="tool"
            onClick={() => stepZoom(1)}
            disabled={!doc}
            title={t('zoom.in')}
            aria-label={t('zoom.in')}
          >
            <IconPlus />
          </button>
        </div>
      </div>
    </>
  );
}

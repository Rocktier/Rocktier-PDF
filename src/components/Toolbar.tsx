import { useI18n, useT } from '../i18n';
import type { DocumentInfo } from '../types';
import {
  IconExtract,
  IconMerge,
  IconMinus,
  IconMoon,
  IconOpen,
  IconPlus,
  IconRotateLeft,
  IconRotateRight,
  IconSave,
  IconSplit,
  IconSun,
  IconTrash,
  Logo,
} from './Logo';

interface ToolbarProps {
  doc: DocumentInfo | null;
  busy: boolean;
  selectedCount: number;
  zoom: number;
  theme: 'dark' | 'light';
  onOpen: () => void;
  onSave: () => void;
  onSaveAs: () => void;
  onMerge: () => void;
  onSplit: () => void;
  onExtract: () => void;
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
  onOpen,
  onSave,
  onSaveAs,
  onMerge,
  onSplit,
  onExtract,
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

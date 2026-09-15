import { useCallback, useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DropZone } from './components/DropZone';
import { MergeDialog } from './components/MergeDialog';
import { PageViewer } from './components/PageViewer';
import { SplitDialog } from './components/SplitDialog';
import { StatusBar } from './components/StatusBar';
import { ThumbnailRail } from './components/ThumbnailRail';
import { Toolbar } from './components/Toolbar';
import { usePdf } from './hooks/usePdf';
import { useT } from './i18n';
import { fileStem, pickPdf, pickSavePath, revealInFinder } from './services/engine';

type Theme = 'dark' | 'light';
type Dialog = 'merge' | 'split' | null;

const THEME_KEY = 'rocktier-pdf-editor.theme';

export function App() {
  const t = useT();
  const pdf = usePdf();

  const [theme, setTheme] = useState<Theme>(() => readTheme());
  const [zoom, setZoom] = useState(1);
  const [current, setCurrent] = useState(0);
  const [jump, setJump] = useState<{ index: number; token: number } | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [dragActive, setDragActive] = useState(false);
  const [toast, setToast] = useState<{ text: string; error?: boolean } | null>(null);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {
      /* ignore */
    }
  }, [theme]);

  const notify = useCallback((text: string, error = false) => {
    setToast({ text, error });
    window.setTimeout(() => setToast(null), 2600);
  }, []);

  /* ── Drag & drop from the OS ────────────────────────────────── */
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void getCurrentWindow()
      .onDragDropEvent((event) => {
        if (disposed) return;
        if (event.payload.type === 'enter' || event.payload.type === 'over') {
          setDragActive(true);
          return;
        }
        if (event.payload.type === 'leave') {
          setDragActive(false);
          return;
        }
        setDragActive(false);
        const path = event.payload.paths.find((p) => p.toLowerCase().endsWith('.pdf'));
        if (path) void pdf.openPath(path);
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [pdf.openPath]);

  /* ── Keyboard shortcuts ─────────────────────────────────────── */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === 'o') {
        e.preventDefault();
        void openFile();
      } else if (mod && e.key.toLowerCase() === 's') {
        e.preventDefault();
        void save();
      } else if ((e.key === 'Delete' || e.key === 'Backspace') && pdf.selected.length > 0) {
        e.preventDefault();
        void removeSelected();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pdf.selected]);

  /* ── Actions ────────────────────────────────────────────────── */

  const openFile = useCallback(async () => {
    const path = await pickPdf();
    if (!path) return;
    const info = await pdf.openPath(path);
    if (info) {
      setCurrent(0);
      setZoom(1);
      notify(t('toast.opened', { name: info.name }));
    }
  }, [notify, pdf, t]);

  const save = useCallback(async () => {
    const path = await pdf.save();
    if (path) notify(t('toast.saved', { name: fileStem(path) + '.pdf' }));
  }, [notify, pdf, t]);

  const saveAs = useCallback(async () => {
    if (!pdf.doc) return;
    const target = await pickSavePath(pdf.doc.name);
    if (!target) return;
    const path = await pdf.saveAs(target);
    if (path) notify(t('toast.saved', { name: fileStem(path) + '.pdf' }));
  }, [notify, pdf, t]);

  const removeSelected = useCallback(async () => {
    const count = pdf.selected.length;
    await pdf.removePages();
    if (count > 0) notify(t('toast.deleted', { count }));
  }, [notify, pdf, t]);

  const rotateSelected = useCallback(
    async (degrees: number) => {
      const count = pdf.selected.length || 1;
      await pdf.rotate(degrees);
      notify(t('toast.rotated', { count }));
    },
    [notify, pdf, t]
  );

  const extractSelected = useCallback(async () => {
    if (pdf.selected.length === 0 || !pdf.doc) return;
    const target = await pickSavePath(`${fileStem(pdf.doc.name)}-extract.pdf`);
    if (!target) return;
    const path = await pdf.extract(pdf.selected, target);
    if (path) {
      notify(t('toast.saved', { name: fileStem(path) + '.pdf' }));
      void revealInFinder(path);
    }
  }, [notify, pdf, t]);

  const selectPage = useCallback((index: number) => {
    setCurrent(index);
    setJump({ index, token: Date.now() });
  }, []);

  const toggleSelected = useCallback(
    (index: number) => {
      const next = pdf.selected.includes(index)
        ? pdf.selected.filter((i) => i !== index)
        : [...pdf.selected, index].sort((a, b) => a - b);
      pdf.setSelected(next);
    },
    [pdf]
  );

  /* ── Render ─────────────────────────────────────────────────── */

  const doc = pdf.doc;

  return (
    <div className="app">
      <Toolbar
        doc={doc}
        busy={pdf.busy}
        selectedCount={pdf.selected.length}
        zoom={zoom}
        theme={theme}
        onOpen={openFile}
        onSave={save}
        onSaveAs={saveAs}
        onMerge={() => setDialog('merge')}
        onSplit={() => setDialog('split')}
        onExtract={extractSelected}
        onRotate={rotateSelected}
        onDelete={removeSelected}
        onZoom={setZoom}
        onToggleTheme={() => setTheme((prev) => (prev === 'dark' ? 'light' : 'dark'))}
      />

      {doc ? (
        <div className="main">
          <ThumbnailRail
            pages={doc.pages}
            revision={pdf.revision}
            current={current}
            selected={pdf.selected}
            onSelect={selectPage}
            onMove={pdf.move}
          />
          <PageViewer
            pages={doc.pages}
            revision={pdf.revision}
            zoom={zoom}
            selected={pdf.selected}
            jump={jump}
            onSelect={toggleSelected}
            onVisible={setCurrent}
          />
        </div>
      ) : (
        <DropZone onOpen={openFile} dragActive={dragActive} />
      )}

      <StatusBar
        doc={doc}
        busy={pdf.busy}
        error={pdf.error}
        selectedCount={pdf.selected.length}
      />

      {dialog === 'merge' ? (
        <MergeDialog onClose={() => setDialog(null)} onRun={pdf.merge} />
      ) : null}
      {dialog === 'split' && doc ? (
        <SplitDialog
          pageCount={doc.pageCount}
          onClose={() => setDialog(null)}
          onRun={pdf.split}
        />
      ) : null}

      {toast ? <div className={`toast${toast.error ? ' error' : ''}`}>{toast.text}</div> : null}
    </div>
  );
}

function readTheme(): Theme {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === 'dark' || saved === 'light') return saved;
  } catch {
    /* ignore */
  }
  return 'dark';
}

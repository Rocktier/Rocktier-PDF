import { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DropZone } from './components/DropZone';
import { FindBar } from './components/FindBar';
import { FormDialog } from './components/FormDialog';
import { MergeDialog } from './components/MergeDialog';
import { NoteDialog } from './components/NoteDialog';
import { PageViewer } from './components/PageViewer';
import { PasswordDialog } from './components/PasswordDialog';
import { SecurityDialog } from './components/SecurityDialog';
import { SplitDialog } from './components/SplitDialog';
import { StampDialog } from './components/StampDialog';
import { StatusBar } from './components/StatusBar';
import { ThumbnailRail } from './components/ThumbnailRail';
import { Toolbar } from './components/Toolbar';
import { usePdf } from './hooks/usePdf';
import { useI18n, useT } from './i18n';
import {
  buildMenu,
  copyText,
  exportPageImages,
  fileStem,
  imagesToPdf,
  onMenuAction,
  openUrl,
  pageText,
  pickDirectory,
  pickImages,
  pickPdf,
  pickSavePath,
  removePassword,
  revealInFinder,
  searchDocument,
  setPassword,
} from './services/engine';
import type { AnnotTool, MarkupRect, SearchHit, StampKind } from './types';

type Theme = 'dark' | 'light';
type Dialog = 'merge' | 'split' | 'stamp' | 'security' | 'form' | null;

const THEME_KEY = 'rocktier-pdf-editor.theme';

export function App() {
  const t = useT();
  const { lang } = useI18n();
  const pdf = usePdf();

  const [theme, setTheme] = useState<Theme>(() => readTheme());
  const [zoom, setZoom] = useState(1);
  const [current, setCurrent] = useState(0);
  const [jump, setJump] = useState<{ index: number; token: number } | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [dragActive, setDragActive] = useState(false);
  const [toast, setToast] = useState<{ text: string; error?: boolean } | null>(null);

  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState('');
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [activeHit, setActiveHit] = useState(0);
  const searchToken = useRef(0);
  const [markupTool, setMarkupTool] = useState<AnnotTool | null>(null);
  const [noteAt, setNoteAt] = useState<{ page: number; x: number; y: number } | null>(null);
  const [sigPath, setSigPath] = useState<string | null>(null);
  const [pwPrompt, setPwPrompt] = useState<{ path: string; incorrect: boolean } | null>(null);

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

  /* ── Native menu ────────────────────────────────────────────── */
  useEffect(() => {
    void buildMenu(lang);
  }, [lang]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void onMenuAction((id) => {
      switch (id) {
        case 'open':
          void openFile();
          break;
        case 'save':
          void save();
          break;
        case 'save-as':
          void saveAs();
          break;
        case 'undo':
          void pdf.stepHistory('undo');
          break;
        case 'redo':
          void pdf.stepHistory('redo');
          break;
        case 'find':
          if (pdf.doc) setFindOpen(true);
          break;
        case 'actual-size':
          setZoom(1);
          break;
        case 'toggle-theme':
          setTheme((prev) => (prev === 'dark' ? 'light' : 'dark'));
          break;
        case 'website':
          void openUrl('https://rocktier.com/');
          break;
        case 'feedback':
          void openUrl('mailto:hello@rocktier.com');
          break;
        default:
          break;
      }
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pdf.doc, pdf.stepHistory, lang]);

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
      } else if (mod && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        if (pdf.doc) setFindOpen(true);
      } else if (mod && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        void pdf.stepHistory(e.shiftKey ? 'redo' : 'undo');
      } else if ((e.key === 'Delete' || e.key === 'Backspace') && pdf.selected.length > 0) {
        // 焦点在可编辑元素里时，退格/删除是"改字"，不是"删页"。
        // 少了这道守卫，用户在查找框、表单值或批注文字里按退格会静默删掉一整页 ——
        // 只有撤销能救回来，而用户多半不知道发生了什么。
        const el = e.target as HTMLElement | null;
        const editing =
          !!el &&
          (el.isContentEditable ||
            el.tagName === 'INPUT' ||
            el.tagName === 'TEXTAREA' ||
            el.tagName === 'SELECT');
        if (editing) return;
        e.preventDefault();
        void removeSelected();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pdf.selected, pdf.doc, pdf.stepHistory]);

  /* ── Find ───────────────────────────────────────────────────── */
  useEffect(() => {
    if (!findOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        setFindOpen(false);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [findOpen]);

  // Debounced full-document search; a token guards against out-of-order replies.
  useEffect(() => {
    if (!findOpen || !findQuery.trim() || !pdf.doc) {
      setHits([]);
      setActiveHit(0);
      return;
    }
    const token = ++searchToken.current;
    const handle = window.setTimeout(() => {
      void searchDocument(findQuery)
        .then((result) => {
          if (token !== searchToken.current) return;
          setHits(result);
          setActiveHit(0);
        })
        .catch(() => {
          if (token === searchToken.current) setHits([]);
        });
    }, 180);
    return () => window.clearTimeout(handle);
  }, [findQuery, findOpen, pdf.doc, pdf.revision]);

  // Follow the active hit with the viewport.
  useEffect(() => {
    const hit = hits[activeHit];
    if (hit) setJump({ index: hit.page, token: Date.now() });
  }, [activeHit, hits]);

  const stepHit = useCallback(
    (delta: number) => {
      setHits((prevHits) => {
        if (prevHits.length === 0) return prevHits;
        setActiveHit((prev) => (prev + delta + prevHits.length) % prevHits.length);
        return prevHits;
      });
    },
    []
  );

  /* ── Actions ────────────────────────────────────────────────── */

  const openFile = useCallback(async () => {
    const path = await pickPdf();
    if (!path) return;
    const { info, error } = await pdf.openPathWithResult(path);
    if (info) {
      setCurrent(0);
      setZoom(1);
      setFindQuery('');
      setHits([]);
      setActiveHit(0);
      notify(t('toast.opened', { name: info.name }));
    } else if (error === 'PASSWORD_REQUIRED' || error === 'PASSWORD_INCORRECT') {
      setPwPrompt({ path, incorrect: error === 'PASSWORD_INCORRECT' });
    }
  }, [notify, pdf, t]);

  const submitPassword = useCallback(
    async (password: string) => {
      if (!pwPrompt) return false;
      const { info, error } = await pdf.openPathWithResult(pwPrompt.path, password);
      if (info) {
        setPwPrompt(null);
        setCurrent(0);
        setZoom(1);
        notify(t('toast.opened', { name: info.name }));
        return true;
      }
      return !(error === 'PASSWORD_REQUIRED' || error === 'PASSWORD_INCORRECT');
    },
    [notify, pdf, pwPrompt, t]
  );

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
      await pdf.rotate(degrees, current);
      notify(t('toast.rotated', { count }));
    },
    [current, notify, pdf, t]
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

  const copySelectedText = useCallback(async () => {
    if (pdf.selected.length === 0) return;
    try {
      const text = await pageText(pdf.selected);
      if (!text.trim()) {
        notify(t('toast.noText'), true);
        return;
      }
      await copyText(text);
      notify(t('toast.copied'));
    } catch (e) {
      notify(e instanceof Error ? e.message : String(e), true);
    }
  }, [notify, pdf.selected, t]);

  const applyStamp = useCallback(
    async (kind: StampKind, text: string, fontSize: number, margin: number, opacity: number) => {
      const info = await pdf.stamp(kind, text, fontSize, margin, opacity);
      if (info) notify(t('stamp.done'));
    },
    [notify, pdf, t]
  );

  const exportImages = useCallback(async () => {
    if (!pdf.doc) return;
    const dir = await pickDirectory();
    if (!dir) return;
    const pages = pdf.selected.length > 0 ? pdf.selected : pdf.doc.pages.map((p) => p.index);
    try {
      const results = await exportPageImages(pages, dir, 1600);
      notify(t('toast.exported', { count: results.length }));
      if (results[0]) void revealInFinder(results[0].path);
    } catch (e) {
      notify(e instanceof Error ? e.message : String(e), true);
    }
  }, [notify, pdf, t]);

  const runImagesToPdf = useCallback(async () => {
    const imagePaths = await pickImages();
    if (imagePaths.length === 0) return;
    const target = await pickSavePath('images.pdf');
    if (!target) return;
    try {
      const result = await imagesToPdf(imagePaths, target);
      notify(t('toast.imagesPdf', { name: fileStem(result.path) + '.pdf' }));
      void revealInFinder(result.path);
    } catch (e) {
      notify(e instanceof Error ? e.message : String(e), true);
    }
  }, [notify, t]);

  const applyMarkup = useCallback(
    async (rect: MarkupRect) => {
      if (!markupTool || markupTool === 'note' || markupTool === 'sign') return;
      const color: [number, number, number] =
        markupTool === 'highlight' ? [255, 235, 59] : [229, 57, 53];
      const opacity = markupTool === 'highlight' ? 0.35 : 0.9;
      const info = await pdf.markup(markupTool, rect, color, opacity);
      if (info) notify(t('markup.done'));
    },
    [markupTool, notify, pdf, t]
  );

  const applyNote = useCallback(
    async (text: string) => {
      if (!noteAt) return;
      const info = await pdf.note(noteAt.page, noteAt.x, noteAt.y, text, [255, 200, 0]);
      if (info) notify(t('note.done'));
    },
    [noteAt, notify, pdf, t]
  );

  const chooseSignature = useCallback(async () => {
    const images = await pickImages();
    if (images.length === 0) return;
    setSigPath(images[0]);
    setMarkupTool('sign');
    notify(t('sign.hint'));
  }, [notify, t]);

  const handlePageClick = useCallback(
    async (page: number, x: number, y: number) => {
      if (markupTool === 'sign') {
        if (!sigPath) return;
        const info = await pdf.signature(page, x, y, 160, sigPath);
        if (info) notify(t('sign.done'));
        setMarkupTool(null);
        return;
      }
      if (markupTool === 'note') setNoteAt({ page, x, y });
    },
    [markupTool, notify, pdf, sigPath, t]
  );

  const runSecurity = useCallback(
    async (mode: 'set' | 'remove', password: string, ownerPassword: string) => {
      if (!pdf.doc || !pdf.doc.path) {
        throw new Error(t('security.needDoc'));
      }
      const target = await pickSavePath(
        `${fileStem(pdf.doc.name)}-${mode === 'set' ? 'protected' : 'unlocked'}.pdf`
      );
      if (!target) return;

      const result =
        mode === 'set'
          ? await setPassword(pdf.doc.path, target, password, ownerPassword)
          : await removePassword(pdf.doc.path, target, password);

      notify(t('security.done', { name: fileStem(result.path) + '.pdf' }));
      void revealInFinder(result.path);
    },
    [notify, pdf.doc, t]
  );

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
        markupTool={markupTool}
        onMarkupTool={setMarkupTool}
        onOpen={openFile}
        onSave={save}
        onSaveAs={saveAs}
        onMerge={() => setDialog('merge')}
        onSplit={() => setDialog('split')}
        onExtract={extractSelected}
        onCopyText={copySelectedText}
        onStamp={() => setDialog('stamp')}
        onExportImages={exportImages}
        onImagesToPdf={runImagesToPdf}
        onSign={chooseSignature}
        onSecurity={() => setDialog('security')}
        onForm={() => setDialog('form')}
        onUndo={() => void pdf.stepHistory('undo')}
        onRedo={() => void pdf.stepHistory('redo')}
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
          <div className="viewer-wrap">
            <PageViewer
              pages={doc.pages}
              revision={pdf.revision}
              zoom={zoom}
              selected={pdf.selected}
              jump={jump}
              hits={hits}
              activeHit={activeHit}
              markupTool={markupTool}
              onMarkup={(rect) => void applyMarkup(rect)}
              onNoteAt={handlePageClick}
              onSelect={toggleSelected}
              onVisible={setCurrent}
            />
            {findOpen ? (
              <FindBar
                query={findQuery}
                hitCount={hits.length}
                active={activeHit}
                onQuery={setFindQuery}
                onPrev={() => stepHit(-1)}
                onNext={() => stepHit(1)}
                onClose={() => setFindOpen(false)}
              />
            ) : null}
          </div>
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
      {dialog === 'stamp' && doc ? (
        <StampDialog onClose={() => setDialog(null)} onRun={applyStamp} />
      ) : null}
      {dialog === 'security' && doc ? (
        <SecurityDialog onClose={() => setDialog(null)} onRun={runSecurity} />
      ) : null}
      {dialog === 'form' && doc ? (
        <FormDialog
          onClose={() => setDialog(null)}
          onApplied={() => {
            pdf.refresh();
            notify(t('form.done'));
          }}
        />
      ) : null}
      {noteAt ? <NoteDialog onClose={() => setNoteAt(null)} onRun={applyNote} /> : null}
      {pwPrompt ? (
        <PasswordDialog
          incorrect={pwPrompt.incorrect}
          onClose={() => setPwPrompt(null)}
          onSubmit={submitPassword}
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

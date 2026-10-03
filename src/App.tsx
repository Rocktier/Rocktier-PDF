import { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DropZone } from './components/DropZone';
import { FindBar } from './components/FindBar';
import { FormDialog } from './components/FormDialog';
import { MergeDialog } from './components/MergeDialog';
import { NoteDialog } from './components/NoteDialog';
import { PageViewer } from './components/PageViewer';
import { PasswordDialog } from './components/PasswordDialog';
import { Modal } from './components/Modal';
import { RedactBar } from './components/RedactBar';
import { RedactDialog } from './components/RedactDialog';
import { SecurityDialog } from './components/SecurityDialog';
import { CompressDialog } from './components/CompressDialog';
import { LicenseDialog } from './components/LicenseDialog';
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
    compressDocument,
  licenseStatus,
  onLicenseExpired,
  type LicenseInfo,
  } from './services/engine';
import type { AnnotTool, MarkupRect, RedactRect, SearchHit, StampKind } from './types';

type Theme = 'dark' | 'light';
type Dialog = 'merge' | 'split' | 'stamp' | 'security' | 'form' | 'compress' | 'license' | null;

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
  const [license, setLicense] = useState<LicenseInfo | null>(null);

  const refreshLicense = useCallback(() => {
    void licenseStatus()
      .then(setLicense)
      // 读不到授权状态不该打断使用：按"没有状态"处理，界面就不显示胶囊。
      .catch(() => setLicense(null));
  }, []);
  const [dragActive, setDragActive] = useState(false);
  const [toast, setToast] = useState<{ text: string; error?: boolean } | null>(null);

  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState('');
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [activeHit, setActiveHit] = useState(0);
  const searchToken = useRef(0);
  const [markupTool, setMarkupTool] = useState<AnnotTool | null>(null);
  // 脱敏是独立于 markupTool 的选区模式（同一互斥组），但拖出来的矩形先
  // 累积在画布上，等用户在确认弹层里点头才真正应用——永久删除必须有一道闸。
  const [redactMode, setRedactMode] = useState(false);
  const [redactRects, setRedactRects] = useState<RedactRect[]>([]);
  const [redactConfirm, setRedactConfirm] = useState(false);
  const [noteAt, setNoteAt] = useState<{ page: number; x: number; y: number } | null>(null);
  const [sigPath, setSigPath] = useState<string | null>(null);
  const [pwPrompt, setPwPrompt] = useState<{ path: string; incorrect: boolean } | null>(null);
  // 关窗 / 打开另一个文件 / 拖拽替换，三条替换路径共用一个守卫。
  const [guard, setGuard] = useState<
    { kind: 'close' } | { kind: 'open'; path: string; password?: string } | null
  >(null);

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

  // 菜单打开与拖拽共用：报告结果、需要时弹密码框。定义在拖拽监听之前 ——
  // useEffect 的依赖数组在渲染期求值，晚于 const 声明会踩 TDZ。
  const runOpen = useCallback(
    async (path: string, password?: string) => {
      const { info, error } = await pdf.openPathWithResult(path, password);
      if (info) {
        setCurrent(0);
        setZoom(1);
        setFindQuery('');
        setHits([]);
        setActiveHit(0);
        // 换文档后旧选区指向的是别的页面，必须清干净。
        setRedactRects([]);
        setRedactMode(false);
        notify(t('toast.opened', { name: info.name }));
      } else if (error === 'PASSWORD_REQUIRED' || error === 'PASSWORD_INCORRECT') {
        setPwPrompt({ path, incorrect: error === 'PASSWORD_INCORRECT' });
      }
    },
    [notify, pdf, t]
  );

  // 丢掉一份带未保存修改的内存文档是最贵的错误：任何替换先过守卫。
  const requestOpen = useCallback(
    (path: string, password?: string) => {
      if (pdf.doc?.dirty) setGuard({ kind: 'open', path, password });
      else void runOpen(path, password);
    },
    [pdf.doc?.dirty, runOpen]
  );

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
        if (path) requestOpen(path);
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [requestOpen]);

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
        case 'license':
          setDialog('license');
          refreshLicense();
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

  /* ── License: read the trial state once, and open the dialog whenever a write
     was refused for lack of one. Listening here rather than checking the error
     string in every action's catch: the gate is in Rust and covers seventeen
     commands, so a per-action check would be seventeen chances to miss one. ── */
  useEffect(() => {
    refreshLicense();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void onLicenseExpired(() => {
      setDialog('license');
      refreshLicense();
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [refreshLicense]);

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
        // 守卫对话框开着时，退格归对话框（决定保不保存），不许穿透删页。
        if (guard) return;
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
      } else if (e.key === 'Escape' && redactMode) {
        // Esc 只退出选区模式（规格如此），已画的矩形与浮出条原地保留，
        // 应用/取消交给浮出条——按 Esc 绝不能悄悄丢掉用户的选区。
        if (redactConfirm) return; // 确认弹层开着时，Esc 归弹层。
        e.preventDefault();
        setRedactMode(false);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pdf.selected, pdf.doc, pdf.stepHistory, guard, redactMode, redactConfirm]);

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

  /* ── Window close & replacement guard ────────────────────────── */
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void getCurrentWindow()
      .onCloseRequested((event) => {
        if (pdf.doc?.dirty) {
          event.preventDefault();
          setGuard({ kind: 'close' });
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [pdf.doc]);

  const resolveGuard = useCallback(
    async (choice: 'save' | 'discard') => {
      const pending = guard;
      if (!pending) return;
      if (choice === 'save') {
        const saved = await pdf.save();
        // 保存失败就留在守卫上，绝不能静默丢掉未保存的修改。
        if (!saved) return;
      }
      setGuard(null);
      if (pending.kind === 'close') {
        // 必须是 destroy() 而不是 close()：close() 会再发一次 close-requested
        // 事件，而"不保存"并不重置后端 dirty 标志，守卫会把窗口再次拦下 ——
        // 用户点了"不保存"也永远关不掉。destroy() 由守卫自己兜底收尾：
        // 走到这里代表"保存/不保存"已裁决，强制关窗、不再走第二遍守卫。
        void getCurrentWindow().destroy();
      } else {
        void runOpen(pending.path, pending.password);
      }
    },
    [guard, pdf, runOpen]
  );

  /* ── Actions ────────────────────────────────────────────────── */

  const openFile = useCallback(async () => {
    const path = await pickPdf();
    if (!path) return;
    requestOpen(path);
  }, [requestOpen]);

  const submitPassword = useCallback(
    async (password: string) => {
      if (!pwPrompt) return false;
      const { info, error } = await pdf.openPathWithResult(pwPrompt.path, password);
      if (info) {
        setPwPrompt(null);
        setCurrent(0);
        setZoom(1);
        setRedactRects([]);
        setRedactMode(false);
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
    if (count === 0) return;
    const { ok, error } = await pdf.removePages();
    // 成功才报"已删除"；失败把根因弹成错误 toast，绝不谎报成功。
    if (ok) notify(t('toast.deleted', { count }));
    else if (error) notify(error, true);
  }, [notify, pdf, t]);

  const rotateSelected = useCallback(
    async (degrees: number) => {
      const count = pdf.selected.length || 1;
      const { ok, error } = await pdf.rotate(degrees, current);
      if (ok) notify(t('toast.rotated', { count }));
      else if (error) notify(error, true);
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
      const { info, error } = await pdf.stamp(kind, text, fontSize, margin, opacity);
      if (info) notify(t('stamp.done'));
      // 失败要抛回给 StampDialog：它的错误 UI 会显示原因，而不是静默 onClose。
      if (error) throw new Error(error);
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

  /* ── Redaction ───────────────────────────────────────────────── */

  /** Markup 与 Redact 同一互斥组：拿起一头，先放下另一头。 */
  const chooseMarkupTool = useCallback((tool: AnnotTool | null) => {
    setRedactMode(false);
    setMarkupTool(tool);
  }, []);

  const toggleRedact = useCallback(() => {
    setMarkupTool(null);
    if (!redactMode) notify(t('redact.hint'));
    setRedactMode((prev) => !prev);
  }, [notify, redactMode, t]);

  const addRedactRect = useCallback((rect: RedactRect) => {
    setRedactRects((prev) => [...prev, rect]);
  }, []);

  /** 取消 = 丢掉全部待应用矩形并退出选区模式。 */
  const cancelRedact = useCallback(() => {
    setRedactRects([]);
    setRedactMode(false);
  }, []);

  const confirmRedact = useCallback(async () => {
    if (redactRects.length === 0) return;
    const regions = redactRects.map((r) => ({
      pageIndex: r.page,
      x: r.x,
      y: r.y,
      width: r.width,
      height: r.height,
    }));
    // 失败时把错误抛回给 RedactDialog 显示；矩形原地保留，用户可以重试。
    const { outcome, error } = await pdf.redact(regions);
    if (error) throw new Error(error);
    if (!outcome) return;
    setRedactRects([]);
    setRedactMode(false);

    const parts = [t('redact.done', { count: outcome.removed })];
    if (outcome.crossed > 0) {
      parts.push(t('redact.crossed', { count: outcome.crossed }));
    }
    notify(parts.join(' '), false);
    // 校验告警（区域里疑似还有字）延迟一拍，否则会被前一条 toast 覆盖。
    if (outcome.residualRegions > 0) {
      window.setTimeout(
        () => notify(t('redact.residual', { count: outcome.residualRegions }), true),
        2700
      );
    }
  }, [notify, pdf, redactRects, t]);

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
    notify(t('sign.disclaimer'));
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

  // 阅读区方向键翻页：直接算出目标页并跳转。
  const stepPage = useCallback(
    (delta: number) => {
      if (!pdf.doc) return;
      const next = Math.min(Math.max(0, current + delta), pdf.doc.pages.length - 1);
      setCurrent(next);
      setJump({ index: next, token: Date.now() });
    },
    [current, pdf.doc]
  );

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

  /**
   * 压缩到用户选定的**新文件**。压缩是有损操作，没理由让它有权覆盖原件 ——
   * 与加密/解密一致。
   */
  const runCompress = useCallback(
    async (profile: 'web' | 'balanced' | 'archive', output: string): Promise<boolean> => {
      if (!doc) return false;
      try {
        const res = await compressDocument(doc.path, output, profile);
        notify(t('toast.compressed', { name: fileStem(res.path) + '.pdf' }));
        return true;
      } catch (e) {
        notify(String(e));
        return false;
      }
    },
    [doc, notify, t],
  );

  return (
    <div className="app">
      <Toolbar
        doc={doc}
        busy={pdf.busy}
        selectedCount={pdf.selected.length}
        zoom={zoom}
        theme={theme}
        markupTool={markupTool}
        onMarkupTool={chooseMarkupTool}
        redactActive={redactMode}
        onRedactTool={toggleRedact}
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
        onCompress={() => setDialog('compress')}
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
              redactMode={redactMode}
              redactRects={redactRects}
              onRedactRect={addRedactRect}
              onNoteAt={handlePageClick}
              onSelect={toggleSelected}
              onVisible={setCurrent}
              onStep={stepPage}
            />
            {redactRects.length > 0 ? (
              <RedactBar
                count={redactRects.length}
                onApply={() => setRedactConfirm(true)}
                onCancel={cancelRedact}
              />
            ) : null}
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
        license={license}
        onLicenseClick={() => {
          setDialog('license');
          refreshLicense();
        }}
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
      {dialog === 'license' ? (
        <LicenseDialog
          info={license}
          onRefresh={refreshLicense}
          onClose={() => setDialog(null)}
        />
      ) : null}
      {dialog === 'compress' && doc ? (
        <CompressDialog
          defaultName={fileStem(doc.path) + '-compressed.pdf'}
          onClose={() => setDialog(null)}
          onRun={runCompress}
        />
      ) : null}
      {dialog === 'form' && doc ? (
        <FormDialog
          onClose={() => setDialog(null)}
          onApplied={(info) => {
            pdf.applyDoc(info);
            notify(t('form.done'));
          }}
        />
      ) : null}
      {noteAt ? <NoteDialog onClose={() => setNoteAt(null)} onRun={applyNote} /> : null}
      {redactConfirm ? (
        <RedactDialog
          count={redactRects.length}
          onClose={() => setRedactConfirm(false)}
          onConfirm={confirmRedact}
        />
      ) : null}
      {pwPrompt ? (
        <PasswordDialog
          incorrect={pwPrompt.incorrect}
          onClose={() => setPwPrompt(null)}
          onSubmit={submitPassword}
        />
      ) : null}
      {guard ? (
        <Modal
          title={t('dialog.unsaved.title')}
          onClose={() => setGuard(null)}
          footer={
            <>
              <button type="button" className="btn" onClick={() => setGuard(null)}>
                {t('dialog.unsaved.cancel')}
              </button>
              <button
                type="button"
                className="btn btn-danger"
                onClick={() => void resolveGuard('discard')}
              >
                {t('dialog.unsaved.discard')}
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void resolveGuard('save')}
                disabled={pdf.busy}
              >
                {t('dialog.unsaved.save')}
              </button>
            </>
          }
        >
          <p>{t('dialog.unsaved.body', { name: pdf.doc?.name ?? '' })}</p>
        </Modal>
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

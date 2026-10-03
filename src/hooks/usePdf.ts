import { useCallback, useState } from 'react';
import {
  closeDocument,
  deletePages,
  extractPages,
  mergeDocuments,
  movePage,
  openDocument,
  rotatePages,
  saveDocument,
  splitDocument,
  stampDocument,
  addMarkup,
  addNote,
  addSignature,
  redactRegions,
  redo as redoDocument,
  undo as undoDocument,
} from '../services/engine';
import { clearRenderCache } from '../services/renderCache';
import { fileStem } from '../services/engine';
import type {
  DocumentInfo,
  MarkupKind,
  MarkupRect,
  RedactOutcome,
  RedactRegion,
  SplitMode,
  StampKind,
} from '../types';

/**
 * Owns the single open document and every mutation that touches it.
 *
 * Any operation that changes page content or order must clear the render
 * cache — otherwise the UI happily shows stale bitmaps for pages that have
 * moved or been deleted.
 */
export function usePdf() {
  const [doc, setDoc] = useState<DocumentInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  // Bumped whenever the document changes so render caches and lazy pages know
  // to re-fetch, even when a page keeps the same index.
  const [revision, setRevision] = useState(0);

  /**
   * Wraps an async command with busy + error handling.
   *
   * The error message travels with the result instead of being swallowed:
   * callers that talk to the user (toasts, dialogs) need the *reason*, not
   * just a null — reporting "deleted N pages" after a failed delete is a lie
   * (B-7 修复). `error` is also mirrored into the status bar via `setError`.
   */
  const run = useCallback(async <T,>(fn: () => Promise<T>): Promise<{ value: T | null; error: string | null }> => {
    setBusy(true);
    setError(null);
    try {
      return { value: await fn(), error: null };
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      setError(message);
      return { value: null, error: message };
    } finally {
      setBusy(false);
    }
  }, []);

  /** Adopt a document returned by a backend mutation. */
  const applyDoc = useCallback((next: DocumentInfo) => {
    clearRenderCache();
    setDoc(next);
    setSelected([]);
    setRevision((r) => r + 1);
  }, []);

  /** Keep the header in sync after the file was written under a new path. */
  const adoptSaved = useCallback((result: { path: string; size: number }) => {
    setDoc((prev) =>
      prev
        ? {
            ...prev,
            path: result.path,
            name: `${fileStem(result.path)}.pdf`,
            fileSize: result.size,
            dirty: false,
          }
        : prev
    );
  }, []);

  const openPath = useCallback(
    async (path: string, password?: string) => {
      const { value: info } = await run(() => openDocument(path, password));
      if (info) applyDoc(info);
      return info;
    },
    [applyDoc, run]
  );

  /**
   * Like [openPath], but returns the error string instead of swallowing it, so
   * the caller can distinguish a password prompt from a real failure.
   */
  const openPathWithResult = useCallback(
    async (path: string, password?: string) => {
      setBusy(true);
      try {
        const info = await openDocument(path, password);
        applyDoc(info);
        return { info, error: null as string | null };
      } catch (e) {
        const message = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
        setError(message);
        return { info: null, error: message };
      } finally {
        setBusy(false);
      }
    },
    [applyDoc]
  );

  const close = useCallback(async () => {
    await run(() => closeDocument());
    clearRenderCache();
    setDoc(null);
    setSelected([]);
    setRevision((r) => r + 1);
  }, [run]);

  const save = useCallback(async () => {
    const { value: result } = await run(() => saveDocument(null));
    if (result) adoptSaved(result);
    return result?.path ?? null;
  }, [adoptSaved, run]);

  const saveAs = useCallback(
    async (path: string) => {
      const { value: result } = await run(() => saveDocument(path));
      if (result) adoptSaved(result);
      return result?.path ?? null;
    },
    [adoptSaved, run]
  );

  /**
   * Deletes the selected pages and reports the outcome, so the UI can tell
   * "deleted N pages" apart from "the delete failed".
   */
  const removePages = useCallback(async (): Promise<{ ok: boolean; error: string | null }> => {
    if (selected.length === 0) return { ok: false, error: null };
    const { value: info, error } = await run(() => deletePages(selected));
    if (info) applyDoc(info);
    return { ok: info !== null, error };
  }, [applyDoc, run, selected]);

  /** With nothing selected, rotate the page the user is actually looking at. */
  const rotate = useCallback(
    async (degrees: number, fallbackIndex = 0): Promise<{ ok: boolean; error: string | null }> => {
      const target = selected.length > 0 ? selected : doc ? [fallbackIndex] : [];
      if (target.length === 0) return { ok: false, error: null };
      const { value: info, error } = await run(() => rotatePages(target, degrees));
      if (info) applyDoc(info);
      return { ok: info !== null, error };
    },
    [applyDoc, doc, run, selected]
  );

  const move = useCallback(
    async (from: number, to: number) => {
      const { value: info } = await run(() => movePage(from, to));
      if (info) applyDoc(info);
    },
    [applyDoc, run]
  );

  const extract = useCallback(
    async (indices: number[], outputPath: string) => {
      const { value: result } = await run(() => extractPages(indices, outputPath));
      return result?.path ?? null;
    },
    [run]
  );

  const merge = useCallback(
    async (paths: string[], outputPath: string) => {
      const { value: result } = await run(() => mergeDocuments(paths, outputPath));
      return result?.path ?? null;
    },
    [run]
  );

  const split = useCallback(
    async (outputDir: string, mode: SplitMode) => {
      const { value: result } = await run(() => splitDocument(outputDir, mode));
      return result?.map((r) => r.path) ?? [];
    },
    [run]
  );

  /**
   * Applies the stamp and reports the outcome. The dialog that invoked it
   * surfaces `error` in its own error UI instead of closing silently.
   */
  const stamp = useCallback(
    async (
      kind: StampKind,
      text: string,
      fontSize: number,
      margin: number,
      opacity: number
    ): Promise<{ info: DocumentInfo | null; error: string | null }> => {
      const { value: info, error } = await run(() => stampDocument(kind, text, fontSize, margin, opacity));
      if (info) applyDoc(info);
      return { info, error };
    },
    [applyDoc, run]
  );

  const markup = useCallback(
    async (kind: MarkupKind, rect: MarkupRect, color: [number, number, number], opacity: number) => {
      const { value: info } = await run(() => addMarkup(kind, rect, color, opacity));
      if (info) applyDoc(info);
      return info;
    },
    [applyDoc, run]
  );

  const note = useCallback(
    async (page: number, x: number, y: number, text: string, color: [number, number, number]) => {
      const { value: info } = await run(() => addNote(page, x, y, text, color));
      if (info) applyDoc(info);
      return info;
    },
    [applyDoc, run]
  );

  const signature = useCallback(
    async (page: number, x: number, y: number, width: number, imagePath: string) => {
      const { value: info } = await run(() => addSignature(page, x, y, width, imagePath));
      if (info) applyDoc(info);
      return info;
    },
    [applyDoc, run]
  );

  /**
   * Applies the redaction and reports the outcome (counts + warnings travel
   * with the result so the caller can surface them via i18n). Like every
   * mutation, the returned document is adopted and the render cache cleared.
   */
  const redact = useCallback(
    async (regions: RedactRegion[]): Promise<{ outcome: RedactOutcome | null; error: string | null }> => {
      const { value: outcome, error } = await run(() => redactRegions(regions));
      if (outcome) applyDoc(outcome.info);
      return { outcome, error };
    },
    [applyDoc, run]
  );

  /**
   * Steps the document history. An empty history is not an error worth
   * surfacing, so it simply reports `false`.
   */
  const stepHistory = useCallback(
    async (direction: 'undo' | 'redo') => {
      setBusy(true);
      setError(null);
      try {
        const info = direction === 'undo' ? await undoDocument() : await redoDocument();
        applyDoc(info);
        return true;
      } catch {
        return false;
      } finally {
        setBusy(false);
      }
    },
    [applyDoc]
  );

  /** Force every lazily rendered page to redraw (e.g. after a form change). */
  const refresh = useCallback(() => {
    clearRenderCache();
    setRevision((r) => r + 1);
  }, []);

  const clearError = useCallback(() => setError(null), []);

  return {
    doc,
    revision,
    busy,
    error,
    selected,
    setSelected,
    openPath,
    openPathWithResult,
    close,
    save,
    saveAs,
    removePages,
    rotate,
    move,
    extract,
    merge,
    split,
    stamp,
    markup,
    note,
    signature,
    redact,
    stepHistory,
    refresh,
    clearError,
    applyDoc,
  };
}

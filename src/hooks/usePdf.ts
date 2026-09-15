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
} from '../services/engine';
import { clearRenderCache } from '../services/renderCache';
import type { DocumentInfo, SplitMode } from '../types';

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

  /** Wraps an async command with busy + error handling. */
  const run = useCallback(async <T,>(fn: () => Promise<T>): Promise<T | null> => {
    setBusy(true);
    setError(null);
    try {
      return await fn();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return null;
    } finally {
      setBusy(false);
    }
  }, []);

  const applyDoc = useCallback((next: DocumentInfo) => {
    clearRenderCache();
    setDoc(next);
    setSelected([]);
    setRevision((r) => r + 1);
  }, []);

  const openPath = useCallback(
    async (path: string) => {
      const info = await run(() => openDocument(path));
      if (info) {
        clearRenderCache();
        setDoc(info);
        setSelected([]);
        setRevision((r) => r + 1);
      }
      return info;
    },
    [run]
  );

  const close = useCallback(async () => {
    await run(() => closeDocument());
    clearRenderCache();
    setDoc(null);
    setSelected([]);
    setRevision((r) => r + 1);
  }, [run]);

  const save = useCallback(async () => {
    const result = await run(() => saveDocument(null));
    if (result && doc) setDoc({ ...doc, path: result.path, dirty: false });
    return result?.path ?? null;
  }, [doc, run]);

  const saveAs = useCallback(
    async (path: string) => {
      const result = await run(() => saveDocument(path));
      if (result && doc) setDoc({ ...doc, path: result.path, dirty: false });
      return result?.path ?? null;
    },
    [doc, run]
  );

  const removePages = useCallback(async () => {
    if (selected.length === 0) return;
    const info = await run(() => deletePages(selected));
    if (info) applyDoc(info);
  }, [applyDoc, run, selected]);

  const rotate = useCallback(
    async (degrees: number) => {
      const target = selected.length > 0 ? selected : doc ? [0] : [];
      if (target.length === 0) return;
      const info = await run(() => rotatePages(target, degrees));
      if (info) applyDoc(info);
    },
    [applyDoc, doc, run, selected]
  );

  const move = useCallback(
    async (from: number, to: number) => {
      const info = await run(() => movePage(from, to));
      if (info) applyDoc(info);
    },
    [applyDoc, run]
  );

  const extract = useCallback(
    async (indices: number[], outputPath: string) => {
      const result = await run(() => extractPages(indices, outputPath));
      return result?.path ?? null;
    },
    [run]
  );

  const merge = useCallback(
    async (paths: string[], outputPath: string) => {
      const result = await run(() => mergeDocuments(paths, outputPath));
      return result?.path ?? null;
    },
    [run]
  );

  const split = useCallback(
    async (outputDir: string, mode: SplitMode) => {
      const result = await run(() => splitDocument(outputDir, mode));
      return result?.map((r) => r.path) ?? [];
    },
    [run]
  );

  const clearError = useCallback(() => setError(null), []);

  return {
    doc,
    revision,
    busy,
    error,
    selected,
    setSelected,
    openPath,
    close,
    save,
    saveAs,
    removePages,
    rotate,
    move,
    extract,
    merge,
    split,
    clearError,
  };
}

import { invoke } from '@tauri-apps/api/core';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import type { DocumentInfo, PageInfo, PathResult, RenderedPage, SplitMode } from '../types';

/**
 * Every backend call goes through this module. Components never import
 * `@tauri-apps/api` directly, so the whole IPC surface stays in one place.
 *
 * Tauri v2 converts these camelCase JS arguments to snake_case Rust parameters.
 */

export async function openDocument(path: string): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('open_document', { path });
}

export async function getDocument(): Promise<DocumentInfo | null> {
  return invoke<DocumentInfo | null>('get_document');
}

export async function closeDocument(): Promise<void> {
  return invoke<void>('close_document');
}

/** Render a page (or thumbnail) at the requested pixel width. */
export async function renderPage(index: number, targetWidth: number): Promise<RenderedPage> {
  return invoke<RenderedPage>('render_page', { index, targetWidth });
}

export async function deletePages(indices: number[]): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('delete_pages', { indices });
}

export async function rotatePages(indices: number[], degrees: number): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('rotate_pages', { indices, degrees });
}

/** Move the page at `from` so that it lands at index `to`. */
export async function movePage(from: number, to: number): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('move_page', { from, to });
}

export async function saveDocument(path: string | null): Promise<PathResult> {
  return invoke<PathResult>('save_document', { path });
}

/** Write the selected pages out as a brand new PDF; leaves the current doc untouched. */
export async function extractPages(indices: number[], outputPath: string): Promise<PathResult> {
  return invoke<PathResult>('extract_pages', { indices, outputPath });
}

export async function mergeDocuments(paths: string[], outputPath: string): Promise<PathResult> {
  return invoke<PathResult>('merge_documents', { paths, outputPath });
}

export async function splitDocument(outputDir: string, mode: SplitMode): Promise<PathResult[]> {
  return invoke<PathResult[]>('split_document', { outputDir, mode });
}

export async function revealInFinder(path: string): Promise<void> {
  return invoke<void>('reveal_in_finder', { path });
}

/* ── Native dialogs ─────────────────────────────────────────────── */

export async function pickPdf(): Promise<string | null> {
  const result = await openDialog({
    multiple: false,
    directory: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  });
  return typeof result === 'string' ? result : null;
}

export async function pickPdfs(): Promise<string[]> {
  const result = await openDialog({
    multiple: true,
    directory: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  });
  if (!result) return [];
  return Array.isArray(result) ? result : [result];
}

export async function pickSavePath(defaultName: string): Promise<string | null> {
  return saveDialog({
    defaultPath: defaultName,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  });
}

export async function pickDirectory(): Promise<string | null> {
  const result = await openDialog({ multiple: false, directory: true });
  return typeof result === 'string' ? result : null;
}

/* ── Helpers ────────────────────────────────────────────────────── */

export function formatBytes(bytes: number): string {
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(value >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

/** Alias kept because the UI reads more naturally with "size". */
export function formatSize(bytes: number): string {
  return formatBytes(bytes);
}

/** `C:\docs\Report.pdf` → `Report`. Uses the last separator, so it works on
 *  both Windows and POSIX paths as they arrive from the native dialogs. */
export function fileStem(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? path;
  return name.replace(/\.pdf$/i, '');
}

/** Human-readable page label, 1-based. */
export function pageLabel(page: PageInfo): string {
  return String(page.index + 1);
}

import { invoke } from '@tauri-apps/api/core';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import type {
  DocumentInfo,
  MarkupKind,
  MarkupRect,
  PathResult,
  RenderedPage,
  SearchHit,
  SplitMode,
  StampKind,
} from '../types';

/**
 * Every backend call goes through this module. Components never import
 * `@tauri-apps/api` directly, so the whole IPC surface stays in one place.
 *
 * Tauri v2 converts these camelCase JS arguments to snake_case Rust parameters.
 */

export async function openDocument(path: string): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('open_document', { path });
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

/* ── Text & search ──────────────────────────────────────────────── */

/** Find every occurrence of `query`, across all pages. */
export async function searchDocument(
  query: string,
  matchCase = false,
  wholeWord = false
): Promise<SearchHit[]> {
  return invoke<SearchHit[]>('search_document', { query, matchCase, wholeWord });
}

/** Concatenated plain text of the given pages. */
export async function pageText(indices: number[]): Promise<string> {
  return invoke<string>('page_text', { indices });
}

/** Copy text to the OS clipboard via the webview Clipboard API. */
export async function copyText(text: string): Promise<void> {
  if (!text) return;
  await navigator.clipboard.writeText(text);
}

/* ── Stamping ───────────────────────────────────────────────────── */

/** Add page numbers or a text watermark to every page. */
export async function stampDocument(
  kind: StampKind,
  text: string,
  fontSize: number,
  margin: number,
  opacity: number
): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('stamp_document', { kind, text, fontSize, margin, opacity });
}

/* ── Import / export images ─────────────────────────────────────── */

/** Draw a highlight / underline / strikeout over a dragged rectangle. */
export async function addMarkup(
  kind: MarkupKind,
  rect: MarkupRect,
  color: [number, number, number],
  opacity: number
): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('add_markup', { kind, rect, color, opacity });
}

/** Add a sticky note anchored at a point (PDF points, bottom-left origin). */
export async function addNote(
  page: number,
  x: number,
  y: number,
  text: string,
  color: [number, number, number]
): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('add_note', { page, x, y, text, color });
}

/** Place a signature image at a point (PDF points, bottom-left origin). */
export async function addSignature(
  page: number,
  x: number,
  y: number,
  width: number,
  imagePath: string
): Promise<DocumentInfo> {
  return invoke<DocumentInfo>('add_signature', { page, x, y, width, imagePath });
}

/** Export the given pages as PNG files into `outputDir`. */
export async function exportPageImages(
  indices: number[],
  outputDir: string,
  width: number
): Promise<PathResult[]> {
  return invoke<PathResult[]>('export_page_images', { indices, outputDir, width });
}

/** Build a PDF (one page per image) from image files. */
export async function imagesToPdf(paths: string[], outputPath: string): Promise<PathResult> {
  return invoke<PathResult>('images_to_pdf', { paths, outputPath });
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

export async function pickImages(): Promise<string[]> {
  const result = await openDialog({
    multiple: true,
    directory: false,
    filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg'] }],
  });
  if (!result) return [];
  return Array.isArray(result) ? result : [result];
}

/* ── Helpers ────────────────────────────────────────────────────── */

export function formatBytes(bytes: number): string {
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(value >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

/** `C:\docs\Report.pdf` → `Report`. Uses the last separator, so it works on
 *  both Windows and POSIX paths as they arrive from the native dialogs. */
export function fileStem(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? path;
  return name.replace(/\.pdf$/i, '');
}

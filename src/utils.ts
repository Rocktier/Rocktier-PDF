import type { PageInfo } from './types';

/** PDF points (1/72") → CSS pixels at 100% zoom (96 dpi screen baseline). */
export const ZOOM_BASE = 96 / 72;

/** Cap device pixel ratio: 3x displays would quadruple render cost for no gain. */
export function devicePixelRatioCapped(): number {
  const dpr = typeof window === 'undefined' ? 1 : window.devicePixelRatio || 1;
  return Math.min(Math.max(dpr, 1), 2);
}

/** Page dimensions in CSS pixels, accounting for intrinsic rotation. */
export function displaySize(page: PageInfo, zoom: number): { width: number; height: number } {
  const swapped = page.rotation === 90 || page.rotation === 270;
  const w = swapped ? page.height : page.width;
  const h = swapped ? page.width : page.height;
  return {
    width: Math.max(1, Math.round(w * zoom * ZOOM_BASE)),
    height: Math.max(1, Math.round(h * zoom * ZOOM_BASE)),
  };
}

/** Bitmap width to request from the backend (device pixels). */
export function renderWidth(page: PageInfo, zoom: number): number {
  return Math.round(displaySize(page, zoom).width * devicePixelRatioCapped());
}

/**
 * The backend reports page geometry and search hits in the *unrotated* PDF
 * coordinate space (origin bottom-left), while the rendered bitmap is already
 * rotated by `page.rotation`. These helpers bridge the two so that highlights
 * and annotation placement line up on rotated pages.
 */

function normRotation(r: number): number {
  return ((r % 360) + 360) % 360;
}

/** Unrotated PDF rect (x, y, w, h in points) → fraction of the rotated display box. */
export function pdfRectToDisplayFrac(
  x: number, y: number, w: number, h: number,
  rotation: number, W: number, H: number,
): { left: number; top: number; width: number; height: number } {
  const an = x / W;
  const bn = (H - y - h) / H; // unrotated top-left corner
  const wn = w / W;
  const hn = h / H;
  switch (normRotation(rotation)) {
    case 90:
      return { left: 1 - bn - hn, top: 1 - an - wn, width: hn, height: wn };
    case 180:
      return { left: 1 - an - wn, top: 1 - bn - hn, width: wn, height: hn };
    case 270:
      return { left: bn, top: an, width: hn, height: wn };
    default:
      return { left: an, top: bn, width: wn, height: hn };
  }
}

/**
 * A rect in the rotated display box (CSS pixels, top-left origin) → unrotated
 * PDF rect (points, bottom-left origin). Pass w = h = 0 to map a single point
 * (e.g. a click for note/stamp placement).
 */
export function displayRectToPdf(
  left: number, top: number, w: number, h: number,
  cssWidth: number, cssHeight: number, rotation: number, W: number, H: number,
): { x: number; y: number; width: number; height: number } {
  const L = left / cssWidth;
  const T = top / cssHeight;
  const Wd = w / cssWidth;
  const Hd = h / cssHeight;
  let an = L, bn = T, wn = Wd, hn = Hd;
  switch (normRotation(rotation)) {
    case 90:
      an = 1 - T - Hd; bn = 1 - L - Wd; wn = Hd; hn = Wd; break;
    case 180:
      an = 1 - L - Wd; bn = 1 - T - Hd; wn = Wd; hn = Hd; break;
    case 270:
      an = T; bn = L; wn = Hd; hn = Wd; break;
    default:
      break;
  }
  return { x: an * W, y: H - (bn + hn) * H, width: wn * W, height: hn * H };
}

/* ── 最近打开 ─────────────────────────────────────────────────────
   Acrobat / PDF Expert / macOS Preview 都有这一栏，它解决的是最高频的一个
   问题：「再打开上次那份」。只存路径不存内容 —— 路径本来就是用户自己的
   本地信息，刷新成本是零。
   放在 utils 而不是组件里：这是一段有分支和解析的存储逻辑，值得被独立测；
   埋在 App.tsx 里就只能靠读代码确认。 */

/** 家族命名空间前缀（rocktier.）由 family-check 的 storage-key-namespace 强制。 */
const RECENT_KEY = 'rocktier.pdf.recent';
export const RECENT_MAX = 5;

/** 读最近列表。存量值可能是任何东西（手工编辑过、别的版本写的），逐项过滤。 */
export function readRecent(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const list: unknown = JSON.parse(raw);
    if (!Array.isArray(list)) return [];
    return list.filter((p): p is string => typeof p === 'string').slice(0, RECENT_MAX);
  } catch {
    /* 值损坏或 storage 不可用 —— 当作没有最近记录。打开文件不该因此失败。 */
    return [];
  }
}

/** 写最近列表。storage 不可用时静默降级：本次会话内列表照常工作，只是不持久。 */
export function writeRecent(list: string[]): void {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list));
  } catch {
    /* 同上 */
  }
}

export function clearStoredRecent(): void {
  try {
    localStorage.removeItem(RECENT_KEY);
  } catch {
    /* 同上 */
  }
}

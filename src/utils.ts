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

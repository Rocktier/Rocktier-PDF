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

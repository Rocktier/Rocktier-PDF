import { renderPage } from './engine';
import type { RenderedPage } from '../types';

/**
 * Rendered bitmaps are expensive (encode + IPC + base64), so we cache them.
 *
 * Key is `index@width` because the caller renders the same page at several
 * sizes (thumbnail vs. main view at different zoom levels).
 *
 * The cache is process-wide and must be cleared explicitly whenever the
 * document changes — see `clearRenderCache()`.
 */

const MAX_ENTRIES = 160;

const cache = new Map<string, RenderedPage>();
const inflight = new Map<string, Promise<RenderedPage>>();

function key(index: number, width: number): string {
  return `${index}@${width}`;
}

/** Simple FIFO eviction — good enough and never surprises the user mid-scroll. */
function evictIfNeeded(): void {
  while (cache.size > MAX_ENTRIES) {
    const oldest = cache.keys().next();
    if (oldest.done) break;
    cache.delete(oldest.value);
  }
}

export function getCached(index: number, width: number): RenderedPage | undefined {
  return cache.get(key(index, width));
}

export function getRender(index: number, width: number): Promise<RenderedPage> {
  const k = key(index, width);

  const hit = cache.get(k);
  if (hit) return Promise.resolve(hit);

  const pending = inflight.get(k);
  if (pending) return pending;

  const p = renderPage(index, Math.max(16, Math.round(width)))
    .then((page) => {
      cache.set(k, page);
      inflight.delete(k);
      evictIfNeeded();
      return page;
    })
    .catch((err) => {
      inflight.delete(k);
      throw err;
    });

  inflight.set(k, p);
  return p;
}

/** Call after any mutation that changes page content or order. */
export function clearRenderCache(): void {
  cache.clear();
  inflight.clear();
}

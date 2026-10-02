import { useEffect, useRef, useState } from 'react';

/**
 * Reports whether an element is near the viewport.
 *
 * Pages are only rendered once they scroll close to view — a 500-page document
 * must not trigger 500 Pdfium renders on open.
 *
 * The flag re-arms when the element leaves the margin zone again. That does
 * NOT discard anything already on screen: callers keep the rendered bitmap in
 * their own state and the shared render cache serves re-entry, so scrolling
 * back is cheap. What it fixes is the latch's real leak — without re-arming,
 * every page the user had ever scrolled past re-rendered on each zoom change
 * and document revision, because `inView` stayed `true` forever.
 */
export function useInView<T extends HTMLElement>(rootMargin = '400px') {
  const ref = useRef<T | null>(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === 'undefined') {
      setInView(true);
      return;
    }

    // IntersectionObserver always fires once right after observe(), so pages
    // already inside the margin turn visible without waiting for a scroll.
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          setInView(entry.isIntersecting);
        }
      },
      { rootMargin }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [rootMargin]);

  return { ref, inView };
}

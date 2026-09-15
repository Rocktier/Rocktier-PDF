import { useEffect, useRef, useState } from 'react';

/**
 * Reports whether an element is near the viewport.
 *
 * Pages are only rendered once they scroll close to view — a 500-page document
 * must not trigger 500 Pdfium renders on open.
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

    // If it is already visible there is no need to wait for a callback.
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setInView(true);
            observer.disconnect();
          }
        }
      },
      { rootMargin }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [rootMargin]);

  return { ref, inView };
}

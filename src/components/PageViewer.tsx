import { useEffect, useState } from 'react';
import { getCached, getRender } from '../services/renderCache';
import type { PageInfo, RenderedPage } from '../types';
import { devicePixelRatioCapped, renderWidth } from '../utils';
import { useInView } from '../hooks/useInView';

interface PageViewerProps {
  pages: PageInfo[];
  revision: number;
  zoom: number;
  selected: number[];
  jump: { index: number; token: number } | null;
  onSelect: (index: number) => void;
  onVisible: (index: number) => void;
}

export function PageViewer({
  pages,
  revision,
  zoom,
  selected,
  jump,
  onSelect,
  onVisible,
}: PageViewerProps) {
  return (
    <div className="viewer">
      <div className="viewer-inner">
        {pages.map((page, position) => (
          <Page
            key={`${revision}:${page.index}`}
            page={page}
            position={position}
            revision={revision}
            zoom={zoom}
            selected={selected.includes(position)}
            jump={jump}
            onSelect={onSelect}
            onVisible={onVisible}
          />
        ))}
      </div>
    </div>
  );
}

interface PageProps {
  page: PageInfo;
  position: number;
  revision: number;
  zoom: number;
  selected: boolean;
  jump: { index: number; token: number } | null;
  onSelect: (index: number) => void;
  onVisible: (index: number) => void;
}

function Page({
  page,
  position,
  revision,
  zoom,
  selected,
  jump,
  onSelect,
  onVisible,
}: PageProps) {
  const { ref, inView } = useInView<HTMLDivElement>('600px');
  const [rendered, setRendered] = useState<RenderedPage | null>(null);

  const width = renderWidth(page, zoom);

  useEffect(() => {
    if (!inView) return;

    const cached = getCached(page.index, width);
    if (cached) {
      setRendered(cached);
      return;
    }

    let alive = true;
    getRender(page.index, width)
      .then((p) => {
        if (alive) setRendered(p);
      })
      .catch(() => {
        /* leave the placeholder in place */
      });

    return () => {
      alive = false;
    };
  }, [inView, page.index, width, revision]);

  // Report the page closest to the top of the viewport as "current".
  useEffect(() => {
    if (inView) onVisible(position);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inView]);

  // Explicit jump from a thumbnail click.
  useEffect(() => {
    if (jump && jump.index === position) {
      ref.current?.scrollIntoView({ block: 'start', behavior: 'smooth' });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [jump?.token]);

  const cssWidth = rendered
    ? Math.round(rendered.width / devicePixelRatioCapped())
    : Math.round(page.width * zoom * (96 / 72));
  const cssHeight = rendered
    ? Math.round(rendered.height / devicePixelRatioCapped())
    : Math.round(page.height * zoom * (96 / 72));

  return (
    <div
      ref={ref}
      className={`page${selected ? ' selected' : ''}`}
      style={{ width: cssWidth, height: rendered ? cssHeight : cssHeight }}
      onClick={() => onSelect(position)}
    >
      {rendered ? (
        <img
          className="page-canvas"
          src={rendered.dataUrl}
          alt=""
          style={{ width: cssWidth, height: cssHeight }}
          draggable={false}
        />
      ) : (
        <div className="page-placeholder" style={{ width: cssWidth, height: cssHeight }}>
          {position + 1}
        </div>
      )}
      <span className="page-index">{position + 1}</span>
    </div>
  );
}

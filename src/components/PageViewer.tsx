import { useEffect, useRef, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { getCached, getRender } from '../services/renderCache';
import type { AnnotTool, MarkupRect, PageInfo, RenderedPage, SearchHit } from '../types';
import { devicePixelRatioCapped, renderWidth } from '../utils';
import { useInView } from '../hooks/useInView';

interface PageViewerProps {
  pages: PageInfo[];
  revision: number;
  zoom: number;
  selected: number[];
  jump: { index: number; token: number } | null;
  hits: SearchHit[];
  activeHit: number;
  markupTool: AnnotTool | null;
  onMarkup: (rect: MarkupRect) => void;
  onNoteAt: (page: number, x: number, y: number) => void;
  onSelect: (index: number) => void;
  onVisible: (index: number) => void;
}

export function PageViewer({
  pages,
  revision,
  zoom,
  selected,
  jump,
  hits,
  activeHit,
  markupTool,
  onMarkup,
  onNoteAt,
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
            hits={hits}
            activeHit={activeHit}
            markupTool={markupTool}
            onMarkup={onMarkup}
            onNoteAt={onNoteAt}
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
  hits: SearchHit[];
  activeHit: number;
  markupTool: AnnotTool | null;
  onMarkup: (rect: MarkupRect) => void;
  onNoteAt: (page: number, x: number, y: number) => void;
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
  hits,
  activeHit,
  markupTool,
  onMarkup,
  onNoteAt,
  onSelect,
  onVisible,
}: PageProps) {
  const { ref, inView } = useInView<HTMLDivElement>('600px');
  const [rendered, setRendered] = useState<RenderedPage | null>(null);
  const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const layerRef = useRef<HTMLDivElement>(null);

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

  // Explicit jump from a thumbnail click or a search hit.
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

  // Search hits that belong to this page, keeping their document-wide index so
  // the "active" one can be emphasised across pages.
  const pageHits = hits
    .map((hit, index) => ({ ...hit, index }))
    .filter((hit) => hit.page === position);

  const localPoint = (e: ReactMouseEvent) => {
    const el = layerRef.current;
    if (!el) return { x: 0, y: 0 };
    const r = el.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  };

  const endDrag = () => {
    if (!drag) return;
    const left = Math.min(drag.x0, drag.x1);
    const top = Math.min(drag.y0, drag.y1);
    const w = Math.abs(drag.x1 - drag.x0);
    const h = Math.abs(drag.y1 - drag.y0);
    setDrag(null);

    if (markupTool === 'note' || markupTool === 'sign') {
      // Notes and signatures are placed with a click, not a drag.
      if (w > 6 || h > 6) return;
      onNoteAt(
        position,
        (left / cssWidth) * page.width,
        page.height - (top / cssHeight) * page.height
      );
      return;
    }

    if (w < 4 || h < 4) return;
    // DOM (top-left origin) → PDF points (bottom-left origin).
    onMarkup({
      page: position,
      x: (left / cssWidth) * page.width,
      y: page.height - ((top + h) / cssHeight) * page.height,
      width: (w / cssWidth) * page.width,
      height: (h / cssHeight) * page.height,
    });
  };

  return (
    <div
      ref={ref}
      className={`page${selected ? ' selected' : ''}`}
      style={{ width: cssWidth, height: cssHeight }}
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

      {pageHits.map((hit) => (
        <span
          key={hit.index}
          className={`hit${hit.index === activeHit ? ' active' : ''}`}
          style={{
            left: `${(hit.x / page.width) * 100}%`,
            top: `${((page.height - hit.y - hit.height) / page.height) * 100}%`,
            width: `${(hit.width / page.width) * 100}%`,
            height: `${(hit.height / page.height) * 100}%`,
          }}
        />
      ))}

      {markupTool ? (
        <div
          ref={layerRef}
          className="annot-layer"
          onClick={(e) => e.stopPropagation()}
          onMouseDown={(e) => {
            const p = localPoint(e);
            setDrag({ x0: p.x, y0: p.y, x1: p.x, y1: p.y });
          }}
          onMouseMove={(e) => {
            if (!drag) return;
            const p = localPoint(e);
            setDrag({ ...drag, x1: p.x, y1: p.y });
          }}
          onMouseUp={endDrag}
          onMouseLeave={() => setDrag(null)}
        >
          {drag ? (
            <div
              className={`annot-preview ${markupTool}`}
              style={{
                left: Math.min(drag.x0, drag.x1),
                top: Math.min(drag.y0, drag.y1),
                width: Math.abs(drag.x1 - drag.x0),
                height: Math.abs(drag.y1 - drag.y0),
              }}
            />
          ) : null}
        </div>
      ) : null}

      <span className="page-index">{position + 1}</span>
    </div>
  );
}

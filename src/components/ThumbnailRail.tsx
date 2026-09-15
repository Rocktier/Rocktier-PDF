import { useEffect, useState } from 'react';
import type { DragEvent } from 'react';
import { useT } from '../i18n';
import { getCached, getRender } from '../services/renderCache';
import type { PageInfo, RenderedPage } from '../types';
import { devicePixelRatioCapped, displaySize } from '../utils';
import { useInView } from '../hooks/useInView';

const THUMB_CSS_WIDTH = 104;

interface ThumbnailRailProps {
  pages: PageInfo[];
  revision: number;
  current: number;
  selected: number[];
  onSelect: (index: number) => void;
  onMove: (from: number, to: number) => void;
}

export function ThumbnailRail({
  pages,
  revision,
  current,
  selected,
  onSelect,
  onMove,
}: ThumbnailRailProps) {
  const t = useT();
  const [dragIndex, setDragIndex] = useState<number | null>(null);

  return (
    <div className="rail">
      <div className="rail-header">
        {t('rail.pages')} · {pages.length}
      </div>
      {pages.map((page, position) => (
        <Thumb
          key={`${revision}:${page.index}`}
          page={page}
          position={position}
          revision={revision}
          active={position === current}
          selected={selected.includes(position)}
          dragging={dragIndex === position}
          onSelect={onSelect}
          onDragStart={() => setDragIndex(position)}
          onDragEnd={() => setDragIndex(null)}
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault();
            if (dragIndex !== null && dragIndex !== position) onMove(dragIndex, position);
            setDragIndex(null);
          }}
        />
      ))}
    </div>
  );
}

interface ThumbProps {
  page: PageInfo;
  position: number;
  revision: number;
  active: boolean;
  selected: boolean;
  dragging: boolean;
  onSelect: (index: number) => void;
  onDragStart: () => void;
  onDragEnd: () => void;
  onDragOver: (e: DragEvent) => void;
  onDrop: (e: DragEvent) => void;
}

function Thumb({
  page,
  position,
  revision,
  active,
  selected,
  dragging,
  onSelect,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDrop,
}: ThumbProps) {
  const { ref, inView } = useInView<HTMLDivElement>('250px');
  const [rendered, setRendered] = useState<RenderedPage | null>(null);

  const width = Math.round(THUMB_CSS_WIDTH * devicePixelRatioCapped());
  const size = displaySize(page, THUMB_CSS_WIDTH / Math.max(page.width, 1));

  useEffect(() => {
    if (!inView) return;

    const cached = getCached(page.index, width);
    if (cached) {
      setRendered(cached);
      return;
    }

    let alive = true;
    setRendered(null);
    getRender(page.index, width)
      .then((p) => {
        if (alive) setRendered(p);
      })
      .catch(() => {
        /* the placeholder stays; surfacing one bad page is not worth a toast */
      });

    return () => {
      alive = false;
    };
  }, [inView, page.index, width, revision]);

  return (
    <div
      ref={ref}
      className={`thumb${active ? ' active' : ''}${selected ? ' selected' : ''}`}
      style={{ opacity: dragging ? 0.4 : 1 }}
      draggable
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onClick={() => onSelect(position)}
    >
      {rendered ? (
        <img
          className="thumb-canvas"
          src={rendered.dataUrl}
          alt=""
          style={{ width: THUMB_CSS_WIDTH, height: 'auto' }}
          draggable={false}
        />
      ) : (
        <div
          className="thumb-canvas loading"
          style={{ width: THUMB_CSS_WIDTH, height: Math.min(160, Math.max(40, size.height)) }}
        />
      )}
      {selected ? <span className="thumb-badge">✓</span> : null}
      <span className="thumb-label">{position + 1}</span>
    </div>
  );
}

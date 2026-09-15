import { useEffect, useRef } from 'react';
import { useT } from '../i18n';

interface FindBarProps {
  query: string;
  /** Total number of matches across the document. */
  hitCount: number;
  /** Zero-based index of the currently focused match. */
  active: number;
  onQuery: (value: string) => void;
  onPrev: () => void;
  onNext: () => void;
  onClose: () => void;
}

const iconBase = {
  width: 16,
  height: 16,
  viewBox: '0 0 16 16',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.4,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
  'aria-hidden': true,
};

/** Floating find bar, anchored to the top-right of the viewer. */
export function FindBar({ query, hitCount, active, onQuery, onPrev, onNext, onClose }: FindBarProps) {
  const t = useT();
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const hasQuery = query.trim().length > 0;

  return (
    <div className="findbar" role="search">
      <input
        ref={inputRef}
        className="findbar-input"
        type="text"
        value={query}
        placeholder={t('find.placeholder')}
        onChange={(e) => onQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            if (e.shiftKey) onPrev();
            else onNext();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            onClose();
          }
        }}
      />

      <span className="findbar-count mono">
        {hasQuery ? (hitCount > 0 ? `${active + 1} / ${hitCount}` : t('find.none')) : ''}
      </span>

      <button className="btn btn-icon" onClick={onPrev} disabled={hitCount === 0} title={t('find.prev')}>
        <svg {...iconBase}>
          <path d="M8 12.5v-9M8 3.5 4 7.5M8 3.5l4 4" />
        </svg>
      </button>
      <button className="btn btn-icon" onClick={onNext} disabled={hitCount === 0} title={t('find.next')}>
        <svg {...iconBase}>
          <path d="M8 3.5v9M8 12.5 4 8.5M8 12.5l4-4" />
        </svg>
      </button>
      <button className="btn btn-icon" onClick={onClose} title={t('find.close')}>
        <svg {...iconBase}>
          <path d="M4 4l8 8M12 4l-8 8" />
        </svg>
      </button>
    </div>
  );
}

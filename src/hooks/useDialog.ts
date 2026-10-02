import { useEffect, useRef } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';

/**
 * Shared behaviour for the hand-rolled dialogs (they predate `Modal.tsx`,
 * whose pattern this mirrors):
 *
 * - `dialogProps` marks the panel as `role="dialog" aria-modal="true"` so
 *   assistive tech announces a modal context.
 * - Escape closes, same as `Modal`.
 * - The overlay dismisses only when BOTH the press and the click land on the
 *   overlay itself. A plain `onClick={onClose}` fires with the overlay as the
 *   click target whenever it is merely the common ancestor of the press and
 *   release points — so selecting text that starts inside the dialog and ends
 *   on the backdrop silently closed the dialog and threw the input away.
 *
 * Usage:
 *   const { overlayProps, dialogProps } = useDialogA11y(onClose);
 *   <div className="overlay" {...overlayProps}>
 *     <div className="modal" {...dialogProps} aria-label={t('…')}>…</div>
 *   </div>
 */
export function useDialogA11y(onClose: () => void) {
  const pressedOverlay = useRef(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const onMouseDown = (e: ReactMouseEvent<HTMLDivElement>) => {
    pressedOverlay.current = e.target === e.currentTarget;
  };
  const onClick = (e: ReactMouseEvent<HTMLDivElement>) => {
    if (pressedOverlay.current && e.target === e.currentTarget) onClose();
  };

  return {
    dialogProps: { role: 'dialog' as const, 'aria-modal': true as const },
    overlayProps: { onMouseDown, onClick },
  };
}

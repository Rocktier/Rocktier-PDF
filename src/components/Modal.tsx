import { useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import { IconClose } from './Logo';

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

export function Modal({ title, onClose, children, footer }: ModalProps) {
  const boxRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    // 打开时把焦点交给对话框本体：否则 activeElement 停在 BODY，
    // 键盘用户要盲 Tab 从文档头摸进来（2026-10-25 审计实测）。
    boxRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  return (
    <div className="overlay" onMouseDown={onClose} role="presentation">
      <div
        ref={boxRef}
        className="modal"
        tabIndex={-1}
        onMouseDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="modal-header">
          <h3>{title}</h3>
          <button className="btn btn-icon" onClick={onClose} aria-label="Close">
            <IconClose />
          </button>
        </div>
        <div className="modal-body">{children}</div>
        {footer ? <div className="modal-footer">{footer}</div> : null}
      </div>
    </div>
  );
}

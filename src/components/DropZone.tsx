import { useT } from '../i18n';
import { DocumentGlyph } from './Logo';

interface DropZoneProps {
  onOpen: () => void;
  dragActive: boolean;
}

export function DropZone({ onOpen, dragActive }: DropZoneProps) {
  const t = useT();

  return (
    <div className={`empty${dragActive ? ' dragover' : ''}`}>
      <div className="dropzone">
        <DocumentGlyph />
        <h2>{t('empty.title')}</h2>
        <p>{t('empty.subtitle')}</p>
        <button className="btn btn-primary" onClick={onOpen} style={{ marginTop: 4 }}>
          {t('empty.browse')}
        </button>
      </div>
      <div className="empty-hint">{t('empty.hint')}</div>
    </div>
  );
}

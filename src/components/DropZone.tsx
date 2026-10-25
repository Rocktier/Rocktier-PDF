import { useT } from '../i18n';
import { DocumentGlyph } from './Logo';

interface DropZoneProps {
  onOpen: () => void;
  dragActive: boolean;
  /** 最近打开过的文件路径，最新的在前。空数组时不渲染这一栏。 */
  recent: string[];
  onOpenRecent: (path: string) => void;
  onClearRecent: () => void;
}

/** 路径 → 文件名。栏里只显示文件名，完整路径放 title：
 *  长路径会把这一栏撑爆，而文件名才是用户认得出的那一个。 */
function baseName(p: string): string {
  return p.split(/[\\/]/).pop() || p;
}

export function DropZone({
  onOpen,
  dragActive,
  recent,
  onOpenRecent,
  onClearRecent,
}: DropZoneProps) {
  const t = useT();

  return (
    <div className={`empty${dragActive ? ' dragover' : ''}`}>
      <div className="dropzone">
        <DocumentGlyph />
        {/* 空态页的主标题就是这一行 —— 用 h1 而非 h2：外层已有 <main>，
            标题序列应从 h1 开始，不能跳级（2026-10-25 审计 D-78） */}
        <h1>{t('empty.title')}</h1>
        <p>{t('empty.subtitle')}</p>
        <button className="btn btn-primary" onClick={onOpen} style={{ marginTop: 4 }}>
          {t('empty.browse')}
        </button>
      </div>

      {/* 最近打开：空态下的第二条主路径。没有它时，用户每次开新文件都得重走一遍
          文件对话框 —— 而「再打开上次那份」是 PDF 最高频的重复动作。 */}
      {recent.length > 0 ? (
        <div className="recent">
          <div className="recent-head">
            <span>{t('empty.recent')}</span>
            <button type="button" className="recent-clear" onClick={onClearRecent}>
              {t('empty.clear')}
            </button>
          </div>
          <ul>
            {recent.map((p) => (
              <li key={p}>
                <button type="button" onClick={() => onOpenRecent(p)} title={p}>
                  {baseName(p)}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      <div className="empty-hint">{t('empty.hint')}</div>
    </div>
  );
}

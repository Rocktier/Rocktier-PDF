import { useEffect, useState } from 'react';
import { useT } from '../i18n';
import { pdfToMarkdown, pickMarkdownPath, saveMarkdown } from '../services/engine';
import type { MarkdownConversion, MarkdownWarning } from '../types';
import { Modal } from './Modal';

interface MarkdownDialogProps {
  defaultName: string;
  onClose: () => void;
}

/**
 * PDF → Markdown（路线图 R4）的预览与导出对话框。
 *
 * ## 为什么先预览再导出
 *
 * 结构推断是**启发式**的 —— 标题层级靠字号聚类、段落靠行距、列表靠前缀。
 * 在一篇真实文档上，它可能把加粗正文判成标题、把表格拍平成一行。
 * 用户必须**先看见结果**再决定要不要存，而不是点了导出才发现全错。
 *
 * ## 告警必须展示，不能只给统计
 *
 * 「表格未转换」「第N 页疑似扫描件」直接决定结果可不可用。只显示一段
 * 看起来很干净的 Markdown 而不提示这些，用户会当成完整转换 —— 这是
 * 误导，不是简洁。所以 warnings 一律列出来，且用可辨识的样式（不用
 * 只靠颜色）。
 */
export function MarkdownDialog({ defaultName, onClose }: MarkdownDialogProps) {
  const t = useT();
  const [result, setResult] = useState<MarkdownConversion | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void pdfToMarkdown()
      .then((r) => {
        if (!cancelled) setResult(r);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const exportMd = async () => {
    const path = await pickMarkdownPath(defaultName);
    if (!path) return;
    setBusy(true);
    setError(null);
    try {
      await saveMarkdown(path);
      onClose();
    } catch (e) {
      // 后端用 NO_TEXT_EXTRACTED 表示「全文取不到文字」——
      // 这多半是扫描件，要给专门的提示而不是笼统的失败。
      const msg = String(e);
      setError(
        msg.includes('NO_TEXT_EXTRACTED') ? t('markdown.emptyDocument') : t('markdown.saveFailed'),
      );
    } finally {
      setBusy(false);
    }
  };

  const scannedPages =
    result?.warnings
      .filter((w): w is Extract<MarkdownWarning, { kind: 'looksScanned' }> => w.kind === 'looksScanned')
      .map((w) => w.page) ?? [];
  const tableRegions =
    result?.warnings.find(
      (w): w is Extract<MarkdownWarning, { kind: 'tablesNotConverted' }> =>
        w.kind === 'tablesNotConverted',
    )?.regions ?? 0;
  const isEmpty = result?.warnings.some((w) => w.kind === 'emptyDocument') ?? false;

  return (
    <Modal
      title={t('markdown.title')}
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose} disabled={busy}>
            {t('markdown.close')}
          </button>
          <button
            className="btn btn-primary"
            onClick={() => void exportMd()}
            disabled={busy || loading || !result || result.markdown.trim() === ''}
          >
            {busy ? t('markdown.saving') : t('markdown.save')}
          </button>
        </>
      }
    >
      {loading && <p className="hint">{t('markdown.converting')}</p>}
      {error && (
        <p className="md-warn md-warn-error" role="alert">
          <span className="md-warn-tag">{t('markdown.tag.error')}</span>
          {error}
        </p>
      )}

      {result && (
        <>
          <p className="hint">
            {t('markdown.stats', {
              pages: result.stats.pages,
              headings: result.stats.headings,
              paragraphs: result.stats.paragraphs,
              lists: result.stats.listItems,
              bodySize: result.stats.bodySize.toFixed(1),
            })}
          </p>

          {/* 告警区。三条通道，不依赖颜色单通道：
              标签（形状 + 文字，可辨识类型）→ 淡底 + 1px 边框（分组）
              → 正文文案（问题 + 怎么办）。
              刻意**不用** 2px 彩色左边框 —— 那是设计准则明令禁止的默认做法，
              而且它只给「形状」不给人「类型」，两种告警长得一样。 */}
          {isEmpty && (
            <p className="md-warn" role="status">
              <span className="md-warn-tag">{t('markdown.tag.scanned')}</span>
              {t('markdown.emptyDocumentWarning')}
            </p>
          )}
          {scannedPages.length > 0 && (
            <p className="md-warn" role="status">
              <span className="md-warn-tag">{t('markdown.tag.scanned')}</span>
              {t('markdown.scannedPages', { pages: scannedPages.join(', ') })}
            </p>
          )}
          {tableRegions > 0 && (
            <p className="md-warn" role="status">
              <span className="md-warn-tag">{t('markdown.tag.tables')}</span>
              {t('markdown.tablesKept', { regions: tableRegions })}
            </p>
          )}

          <textarea
            className="md-preview"
            readOnly
            value={result.markdown}
            spellCheck={false}
            aria-label={t('markdown.preview')}
          />
        </>
      )}
    </Modal>
  );
}

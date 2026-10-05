export interface PageInfo {
  /** Zero-based page index. */
  index: number;
  /** Page width in PDF points (72/inch), rotation excluded. */
  width: number;
  /** Page height in PDF points, rotation excluded. */
  height: number;
  /** Page rotation in degrees: 0 | 90 | 180 | 270. */
  rotation: number;
}

export interface DocumentInfo {
  /** Absolute path on disk, or "" for an in-memory document. */
  path: string;
  /** File name shown in the UI. */
  name: string;
  pageCount: number;
  fileSize: number;
  /** True when there are unsaved edits. */
  dirty: boolean;
  pages: PageInfo[];
  /**
   * Always `false` today: signatures this app places are visual image stamps,
   * not cryptographic digital signatures (P0-15). The UI must keep saying so
   * until a real digital-signature feature exists.
   */
  hasCryptographicSignature: boolean;
}

export interface RenderedPage {
  index: number;
  /** `data:image/png;base64,...` — never touches the network. */
  dataUrl: string;
  /** Rendered bitmap width in device-independent pixels. */
  width: number;
  /** Rendered bitmap height in device-independent pixels. */
  height: number;
}

export interface PathResult {
  path: string;
  size: number;
}

export interface SearchHit {
  /** Zero-based page index. */
  page: number;
  /** Bounds in PDF points, origin at the bottom-left, rotation excluded. */
  x: number;
  y: number;
  width: number;
  height: number;
}

export type StampKind = 'pageNumbers' | 'watermark';

export interface FormFieldInfo {
  /** Zero-based page index the widget sits on. */
  page: number;
  name: string;
  /** "text" | "checkbox" | "radio" | "combo" | "list" | "button" | "signature" | "unknown" */
  kind: string;
  value: string;
}

export type MarkupKind = 'highlight' | 'underline' | 'strikeout';

/** Any annotation tool the user can arm: rect marks, notes, and signatures. */
export type AnnotTool = MarkupKind | 'note' | 'sign';

/** A rectangle the user marked for redaction, in PDF points (origin bottom-left). */
export interface RedactRect {
  /** Zero-based page index. */
  page: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Mirrors `pdf::RedactRegion` — the wire shape `redact_regions` expects. */
export interface RedactRegion {
  pageIndex: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Mirrors `commands::RedactOutcome`. Warnings are counts, rendered via i18n. */
export interface RedactOutcome {
  /** Text objects removed. */
  removed: number;
  /** Text objects that extended beyond a region and were removed entirely. */
  crossed: number;
  /** Regions where a rescan still found text after the pass. */
  residualRegions: number;
  info: DocumentInfo;
}

export interface MarkupRect {
  /** Zero-based page index. */
  page: number;
  /** Bounds in PDF points, origin bottom-left. */
  x: number;
  y: number;
  width: number;
  height: number;
}

export type SplitMode =
  /** One file per page. */
  | { kind: 'everyPage' }
  /** Split after every N pages. */
  | { kind: 'everyN'; n: number }
  /** Explicit page ranges, e.g. "1-3,5,8-". */
  | { kind: 'ranges'; ranges: string };

/* ── PDF → Markdown（路线图 R4）───────────────────────────────────────── */

/** 转换统计 —— 前端要如实展示，不能只给 markdown 让用户以为转换完美。 */
export interface MarkdownStats {
  pages: number;
  lines: number;
  headings: number;
  paragraphs: number;
  listItems: number;
  tableishRegions: number;
  /** 识别到的正文字号，供用户核对推断是否合理 */
  bodySize: number;
}

/**
 * 非致命问题。**每一条都必须让用户看到**：
 * 表格未转换、疑似扫描件会直接影响结果可用性。
 */
export type MarkdownWarning =
  /** 该页几乎取不到文字 —— 很可能是扫描件，需要 OCR */
  | { kind: 'looksScanned'; page: number }
  /** 检出疑似表格；v1 只输出纯文本，不猜表格结构 */
  | { kind: 'tablesNotConverted'; regions: number }
  /** 全文没有可提取文字 */
  | { kind: 'emptyDocument' };

export interface MarkdownConversion {
  markdown: string;
  stats: MarkdownStats;
  warnings: MarkdownWarning[];
}

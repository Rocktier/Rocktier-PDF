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
  /** 带密码学签名：只读打开，除追加签名外的一切编辑都会被后端拒绝。 */
  readOnly: boolean;
  /** 由后端签名探测决定，不再是恒 false。 */
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

/** `save_document` 的返回值。`readOnly` 表示写出之后是否仍处于只读状态。 */
export interface SaveResult extends PathResult {
  readOnly: boolean;
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

/**
 * Any annotation tool the user can arm: rect marks, notes, and signatures.
 * `sign` 是图片印章（视觉），`dsign` 是数字签名的**放置模式**：点页面取坐标，
 * 不直接落任何注释，所以它不需要 onMarkup 那条拖拽分支。
 */
export type AnnotTool = MarkupKind | 'note' | 'sign' | 'dsign';

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
  /** 双栏切分生效且检出表格区域 —— 切栏可能把表格行切碎，需人工复核 */
  | { kind: 'columnSplitWithTables'; regions: number }
  /** 全文没有可提取文字 */
  | { kind: 'emptyDocument' };

export interface MarkdownConversion {
  markdown: string;
  stats: MarkdownStats;
  warnings: MarkdownWarning[];
}

/* ── 数字签名（T7–T11）───────────────────────────────────────────────── */

/** 证书元数据，镜像 Rust `sign::CertMeta`。 */
export interface CertMeta {
  subject: string;
  cn: string;
  issuer: string;
  notBefore: string;
  notAfter: string;
  keyType: string;
  chainLen: number;
  expired: boolean;
  allowExpired: boolean;
}

export interface CertInfo {
  name: string;
  created: string;
  path: string;
  meta?: CertMeta;
}

/** 单个签名者的验签结论。 */
export interface SignerInfo {
  name: string;
  validSignature: boolean;
  trustedIssuer: boolean;
  extKeyUsageValid: boolean;
  revoked: boolean;
  signatureTime: string;
  timestampStatus: string;
  status: 'valid' | 'invalid' | 'untrusted' | string;
}

export type VerifyVerdict = 'VALID' | 'UNTRUSTED' | 'INVALID' | 'NO_SIGNATURES';

/** 引擎验签结果。`NO_SIGNATURES` 与 `INVALID` 是两件事，界面必须分开说。 */
export interface VerifyReport {
  verdict: VerifyVerdict;
  exitCode: number;
  summary: string;
  signers: SignerInfo[];
}

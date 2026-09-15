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

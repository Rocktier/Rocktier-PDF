# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Save As now updates the document name and size in the status bar instead of
  leaving the previous file's name on screen.
- "Reveal in File Explorer" selects the file instead of opening the default
  folder (Explorer needs `/select,<path>` as a single argument).
- Rotating with no selection now rotates the page you are looking at, not
  page 1.
- Merge/Split dialog errors are localised; they were hardcoded English.

### Added

- Language toggle (EN / 中) in the toolbar — the locale was previously
  auto-detected with no way to change it.

### Removed

- Unused `get_document` command, `formatSize`/`pageLabel` helpers, `clamp`,
  two unused icons and the `Modal` width prop.

## [0.1.0] — 2026-09-15

First MVP release.

### Added

- **View** — continuous scroll with lazy page rendering, thumbnail rail,
  50–300% zoom, dark/light themes.
- **Organise** — reorder pages by dragging thumbnails, rotate 90° left/right,
  delete selected pages.
- **Merge** — append any number of PDFs into a single document.
- **Split** — one file per page, every *N* pages, or custom ranges
  (`1-3, 5, 8-`).
- **Extract** — write selected pages out as a new PDF, leaving the original
  untouched.
- **Save / Save As** — fully local.
- English (default) and 简体中文 interface.
- OS drag & drop, `Ctrl/Cmd+O`, `Ctrl/Cmd+S`, `Delete` shortcuts.
- Bundled Pdfium engine fetched at build time via `scripts/fetch-pdfium.mjs`.

### Notes

- Encrypted PDFs are not yet supported.
- No undo/redo yet.
- Text editing, annotations, and OCR are deliberately out of scope for v0.1.

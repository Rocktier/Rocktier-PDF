# Rocktier PDF Editor — Architecture

> Version 0.1.0
> Status: MVP

## 1. Design philosophy

Inherited unchanged from the Rocktier family:

- **Fast is a feature.** Native code where it matters; nothing rendered that
  isn't on screen.
- **Private by design.** No network client ships in the binary. At all.
- **Every abstraction must pay for itself.** If the simple version works, ship
  the simple version.
- **Built to last.** One bundled native dependency, no framework churn.

## 2. Stack

| Layer | Choice | Why |
|---|---|---|
| Shell | Tauri 2 | ~10 MB installers, OS webview, Rust core |
| Frontend | React 18 + TypeScript + Vite | Same as Rocktier PDF Squeeze |
| PDF engine | Pdfium via `pdfium-render` 0.9 | Chromium's engine; handles the long tail of real-world PDFs |
| Images | `image` 0.25 | PNG encoding for the page bitmap |

Pdfium was chosen over pure-Rust parsers (`lopdf`, `pdf-rs`) because an editor
lives or dies on **rendering fidelity**. If a page renders wrong, nothing else
matters. Pdfium also gives us page import/export (`FPDF_ImportPagesByIndex`)
for free, which is exactly what merge/split/reorder need.

## 3. System shape

```
┌──────────────────────────────────────────────┐
│  Frontend (React)                            │
│  Toolbar · ThumbnailRail · PageViewer        │
│  usePdf (document state) · renderCache       │
└────────────────┬─────────────────────────────┘
                 │ Tauri commands (JSON over IPC)
                 ▼
┌──────────────────────────────────────────────┐
│  Rust backend                                │
│  commands.rs  ── Tauri command surface       │
│  state.rs     ── the single open document    │
│  pdf.rs       ── Pdfium, render, ranges      │
└────────────────┬─────────────────────────────┘
                 │ FFI
                 ▼
┌──────────────────────────────────────────────┐
│  Pdfium (bundled, ~7 MB)                     │
└──────────────────────────────────────────────┘
```

### One process-wide Pdfium

`pdfium-render` stores its bindings in a global slot and **refuses a second
bind**. So there is exactly one `Pdfium`, created lazily on first use and held
in a `static OnceLock<Pdfium>`.

Because `Pdfium::load_pdf_from_*` returns a `PdfDocument<'a>` that borrows the
engine, and the engine is `&'static`, documents are `PdfDocument<'static>` and
can be stored in application state.

> `OnceLock::get_or_try_init` is still unstable, so `init_pdfium` uses
> `get` → `bind` → `get_or_init` by hand. If two threads race, the loser's
> instance is dropped.

### Binary discovery

Three tiers, mirroring how Rocktier PDF Squeeze finds Ghostscript:

1. `<resource dir>/pdfium-runtime/` — bundled in the installer
2. `src-tauri/resources/pdfium-runtime/` — staged by `build.rs` during dev
3. System library

`build.rs` copies **only** the library matching the current target triple into
`resources/pdfium-runtime/`. All four platform builds live in
`resources/pdfium/` (fetched at build time, gitignored), but bundling all of
them would add ~28 MB to every installer. Staging happens *before*
`tauri_build::build()`, which validates resource paths and hard-fails on a
missing directory.

## 4. Rendering model

Pages are expensive; the UI must never render more than it needs.

1. `PageViewer` renders one placeholder per page, sized from the page's point
   dimensions × zoom × device pixel ratio.
2. A `useInView` hook (IntersectionObserver, 600 px root margin) flips each
   placeholder to "wanted" as it approaches the viewport.
3. `renderCache` dedupes in-flight requests and memoises results by
   `index@width`. FIFO eviction above 160 entries.
4. The backend rasterises, PNG-encodes, base64s, and returns a data URL.

The cache is **cleared on every document mutation**. Without this, deleting or
rotating a page leaves stale bitmaps at the surviving indices. `usePdf` also
bumps a `revision` counter so lazy components re-run their effects even when a
page keeps the same index.

### Sizing

- Baseline: 1 PDF point = 1/72", 100% zoom = 96 dpi, so `scale = 96/72`.
- Device pixel ratio is capped at 2 — a 3× display quadruples render cost for
  no perceptible gain.
- Backend clamps requested width to 16–6000 px.

## 5. Command surface

| Command | Purpose |
|---|---|
| `open_document` | Read bytes, load, replace the open document |
| `get_document` / `close_document` | Inspect / release |
| `render_page` | `{ index, targetWidth }` → data URL |
| `delete_pages` | Descending deletion so indices don't shift |
| `rotate_pages` | Reads current `/Rotate`, adds ±90, writes back |
| `move_page` | Rebuilds the document in a new order |
| `save_document` | `path: null` overwrites the original |
| `extract_pages` | Selected pages → new PDF (original untouched) |
| `merge_documents` | First PDF as destination, rest appended |
| `split_document` | Every page / every N / custom ranges |
| `reveal_in_finder` | `open -R` · `explorer /select,` · `xdg-open` |

All commands are `async` so Pdfium work never blocks the UI thread.

### Why documents load from bytes, not a file handle

`load_pdf_from_byte_vec` costs memory on very large files, but it leaves the
source file **unlocked**. With `load_pdf_from_file`, Pdfium keeps the handle
open and "Save" fails on Windows when it tries to overwrite the original.
Correctness beat memory here; the trade-off is documented and can be revisited
with a save-to-temp-then-replace flow.

### Rotation

`PdfPage::set_rotation` needs `&mut PdfPage`, but `PdfPages::get()` returns an
*owned* `PdfPage`. Binding it as `let mut page = …` is therefore enough — the
rotation flag lives on the underlying page object in the document, so the
change survives after the handle drops.

## 6. Frontend structure

```
src/
├── App.tsx               Shell, keyboard shortcuts, OS drag & drop
├── utils.ts              Point→pixel maths, DPR clamping
├── hooks/
│   ├── usePdf.ts         Document state + every mutation
│   └── useInView.ts      IntersectionObserver wrapper
├── services/
│   ├── engine.ts         The only module that calls `invoke`
│   └── renderCache.ts    Memoised, deduped page bitmaps
├── components/
│   ├── Toolbar.tsx
│   ├── ThumbnailRail.tsx  Lazy thumbnails, drag to reorder
│   ├── PageViewer.tsx     Continuous scroll, lazy pages
│   ├── MergeDialog.tsx / SplitDialog.tsx / Modal.tsx
│   ├── DropZone.tsx       Empty state
│   └── Logo.tsx           Rocktier diamond mark + icon set
└── styles/
    ├── tokens.css         Shared family design tokens
    ├── global.css
    └── app.css
```

**Rule:** components never import `@tauri-apps/api` directly. Everything goes
through `services/engine.ts` so the IPC surface stays auditable in one file.

## 7. Design system

`tokens.css` is copied verbatim from Rocktier PDF Squeeze. Dark is the default;
light inverts the monochrome scale. Accent is a single red dot (`#FF4A3D`).

| Token | Dark | Light |
|---|---|---|
| `--bg-primary` | `#000` | `#fff` |
| `--bg-secondary` | `#0a0a0a` | `#f6f6f6` |
| `--accent` | `#fff` | `#000` |
| `--red` | `#FF4A3D` | `#E64537` |

## 8. Security & privacy

- No HTTP client is compiled in.
- CSP: `default-src 'self'`; images restricted to `data:` (page bitmaps) and
  the Tauri asset protocol.
- Capabilities are limited to `core:default`, `dialog:default`, and the window
  controls the shell needs.
- Page bitmaps travel as base64 data URLs through IPC — never written to disk.

## 9. Known limitations (v0.1)

- Encrypted PDFs fail to open; there is no password prompt yet.
- Thumbnail drag-reorder moves one page at a time; multi-select reorder is a
  v0.2 feature.
- Documents are held fully in memory (see §5).
- No undo/redo.
- Text editing, annotations, and OCR are out of scope for now.

## 10. Testing

`cargo test --bin rocktier-pdf-editor` binds the real bundled Pdfium and
covers: rendering to PNG, save→reload round-trips, rotation persistence and
accumulation, page deletion, page copying between documents, merging, and page
range parsing.

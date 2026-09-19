# Rocktier PDF Editor

**Edit PDFs. Fast, private, no internet needed.**

Part of the [Rocktier](https://rocktier.com/) family of small, offline tools.

---

## What it does

Open a PDF, rearrange it, and write it back. That is the whole job for v0.1 —
and it happens entirely on your machine.

| | |
|---|---|
| **View** | Continuous scroll, lazy-rendered pages, thumbnail rail, 50–300% zoom |
| **Organise** | Reorder by dragging thumbnails, rotate, delete |
| **Merge** | Append any number of PDFs into one |
| **Split** | Every page, every *N* pages, or custom ranges (`1-3, 5, 8-`) |
| **Extract** | Pull selected pages out into a new PDF |
| **Save** | Save or Save As — no cloud, no account, no upload |

Deliberately **not** in v0.1: text editing, annotations, form filling, OCR.
Those come later, if they earn their place.

## Why it is fast

- **Pdfium**, the engine inside Chromium, does the heavy lifting in native code.
- Pages render lazily — open a 900-page book and only what you scroll past is
  ever rasterised.
- Rendered bitmaps are cached and reused across zoom levels.
- One bundled ~7 MB library, no sidecar processes, no Electron.

## Why it is private

- Zero network requests. There is no HTTP client in the app.
- No telemetry, no analytics, no crash reporting.
- Files are read from disk, edited in memory, written back to disk.
- The engine is bundled locally, so nothing is ever fetched at runtime.

See [PRIVACY.md](PRIVACY.md).

---

## Building from source

```bash
git clone https://github.com/Rocktier/Rocktier-PDF.git
cd Rocktier-PDF

npm install          # also downloads the Pdfium build for your platform
npm run tauri:dev    # development
npm run tauri:build  # packaged installer
```

`npm install` runs `scripts/fetch-pdfium.mjs`, which pulls the matching
prebuilt Pdfium from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries)
into `src-tauri/resources/pdfium/`. This is the **only** step that touches the
network, and it happens at build time — never when the app runs.

To fetch every supported platform (for cross-compiling or CI):

```bash
node scripts/fetch-pdfium.mjs --all
```

### Requirements

- Node.js 20+
- Rust 1.77+ (via [rustup](https://rustup.rs/))
- Platform build tools:
  - **Windows**: MSVC build tools + WebView2
  - **macOS**: Xcode command line tools

## Platform support

| Platform | Architecture | Status |
|---|---|---|
| Windows | x64 | Supported |
| macOS | Apple Silicon (arm64) | Supported |
| macOS | Intel (x64) | Supported |
| Linux | x64 | Builds, untested |

---

## Project layout

```
src/                    React 18 + TypeScript frontend
  components/           Toolbar, thumbnail rail, page viewer, dialogs
  hooks/                usePdf (document state), useInView (lazy render)
  i18n/                 English (default) + 简体中文
  services/             engine.ts — the single IPC boundary
  styles/               tokens.css — shared Rocktier design tokens
src-tauri/
  src/pdf.rs            Pdfium binding, rendering, page-range parsing
  src/commands.rs       Tauri command surface
  src/state.rs          The one open document
scripts/fetch-pdfium.mjs
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full picture.

## Testing

```bash
cd src-tauri
cargo test --bin rocktier-pdf-editor
```

The tests bind the real bundled Pdfium and verify rendering, save/reload
round-trips, rotation persistence, page copy/delete, and merging.

---

## Design language

Rocktier uses one design system across the family: monochrome, restrained,
dot-matrix texture, and a single red dot as the accent. `src/styles/tokens.css`
is shared verbatim with Rocktier PDF Squeeze and pic2webp — change it in one
place, change it everywhere.

## License

MIT — see [LICENSE](LICENSE).

---

Rock = bedrock. Tier = the next layer up. Tools built to last a decade.

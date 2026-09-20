# Rocktier PDF

**Edit PDFs. Fast, private, no internet needed.**

Part of the [Rocktier](https://rocktier.com/) family of small, offline tools.

---

## What it does

Open a PDF, change it, and write it back — entirely on your machine.

| | |
|---|---|
| **View** | Continuous scroll, lazy-rendered pages, thumbnail rail, 50–300% zoom, dark/light |
| **Read** | Search the whole document, copy text out of it |
| **Organise** | Reorder by dragging thumbnails, rotate, delete, extract selected pages |
| **Merge** | Append any number of PDFs into one |
| **Split** | Every page, every *N* pages, or custom ranges (`1-3, 5, 8-`) |
| **Annotate** | Highlight, underline, strike through, and notes anchored to the page |
| **Sign** | Place your signature image anywhere on a page |
| **Stamp** | Page numbers, text or an image onto the pages |
| **Forms** | Fill text fields, checkboxes and radio buttons — and clear a radio choice again |
| **Security** | Put a password (AES-128) on a document, or remove one you know |
| **Compress** | Three quality levels, powered by qpdf — always writes a **new** file |
| **Images** | Export pages as PNG, or build a PDF out of a folder of images |
| **Undo** | Every destructive step is undoable, 40 steps deep |

Two things are deliberately absent. **Editing existing text runs**: the PDF
content-stream model does not support it honestly, and pretending otherwise
produces documents that reflow into a mess. **OCR**: reading text out of scans is
a separate product in this family, still in development, and it is not going to be
bolted onto this one.

The interface ships in English (default) and 简体中文.

## Why it is fast

- **Pdfium**, the engine inside Chromium, does the heavy lifting in native code.
- Pages render lazily — open a 900-page book and only what you scroll past is
  ever rasterised.
- Rendered bitmaps are cached and reused across zoom levels.
- **qpdf** runs as a bundled sidecar, and only when you ask for compression.
- Around 9 MB packaged. No Electron, no bundled browser, no services.

## Why it is private

- Zero network requests. There is no HTTP client in the app.
- No telemetry, no analytics, no crash reporting.
- Files are read from disk, edited in memory, written back to disk.
- The engine is bundled locally, so nothing is ever fetched at runtime.
- The Windows package declares exactly one capability, `runFullTrust`. There is
  no `internetClient` in the manifest — you can check that yourself in the store
  package.

See [PRIVACY.md](PRIVACY.md).

---

## Building from source

```bash
git clone https://github.com/Rocktier/Rocktier-PDF.git
cd Rocktier-PDF

npm install                                    # also fetches the pinned Pdfium build
bash scripts/provision-qpdf-macos.sh           # macOS: qpdf as a self-contained sidecar
# powershell scripts/provision-qpdf-windows.ps1   # Windows equivalent

npm run tauri:dev                              # development
npm run tauri:build                            # packaged installer
```

**The qpdf step is not optional for a packaged build.** `tauri.conf.json` lists
`resources/qpdf` in `bundle.resources`, and Tauri's build script fails hard when a
listed resource path is missing — so a `cargo check` without the sidecar in place
stops with an error that says very little about the real cause.

`npm install` runs `scripts/fetch-pdfium.mjs`, which pulls prebuilt Pdfium from
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) into
`src-tauri/resources/pdfium/`. This is the **only** step that touches the network,
and it happens at build time — never when the app runs.

The upstream build is **pinned** (`PDFIUM_BUILD` in that script) rather than read
from `releases/latest`: the release download URLs are not the rate-limited API,
and pinning stops upstream's weekly rebuilds from changing what a given commit
produces. To fetch every platform, or to try a different upstream build:

```bash
node scripts/fetch-pdfium.mjs --all
node scripts/fetch-pdfium.mjs --build=8057 --force
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
| Windows | x64 | Supported — shipped as MSIX / MSI / NSIS |
| macOS | Apple Silicon (arm64) | Supported — shipped as DMG (ad-hoc signed, not notarised) |
| macOS | Intel (x64) | Builds from source (`--target x86_64-apple-darwin`); not shipped |
| Linux | x64 | Builds, untested |

---

## Project layout

```
src/                    React 18 + TypeScript frontend
  components/           Toolbar, thumbnail rail, page viewer, dialogs
  hooks/                usePdf (document state), useInView (lazy render)
  i18n/                 en.ts (default) + zh.ts
  services/             engine.ts — the single IPC boundary
  styles/               tokens.css — shared Rocktier design tokens
src-tauri/src/
  pdf.rs                Pdfium binding, rendering, page ranges, annotations, forms
  security.rs           AES-128 encryption and decryption (lopdf)
  compress.rs           The three qpdf profiles
  imagepass.rs          Downsampling, PNG export, images → PDF
  formclear.rs          Clearing a radio choice, which pdfium cannot do — applied at save time
  commands.rs           Tauri command surface
  state.rs              The one open document
scripts/fetch-pdfium.mjs
scripts/provision-qpdf-{macos.sh,windows.ps1}
```

Architecture notes live with the family records (private) rather than in this
repository, which carries source only.

## Testing

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

The tests bind the real bundled Pdfium and cover rendering, save/reload
round-trips, rotation persistence, page copy/delete, merging, the encryption
round-trip, radio-group clearing, and a byte-level comparison of the text layer
against the input on real documents.

---

## Design language

Rocktier uses one design system across the family: monochrome, restrained,
dot-matrix texture, and a single red dot as the accent. `src/styles/tokens.css`
is shared verbatim with Rocktier Markdown and pic2webp — change it in one place,
change it everywhere.

## License

MIT — see [LICENSE](LICENSE). Bundled third-party components are Pdfium
(BSD-3-Clause) and qpdf (Apache-2.0); their notices ship with the app —
see [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) and [`LICENSES/`](LICENSES).

---

Rock = bedrock. Tier = the next layer up. Tools built to last a decade.

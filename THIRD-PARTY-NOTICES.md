# Third-party notices — Rocktier PDF

Rocktier PDF is proprietary software. It **redistributes** the third-party
components listed below, unmodified. This file and the full licence texts in
`LICENSES/` ship inside every installer and Microsoft Store package (look for
`resources/legal/` next to the application), so the notices travel with the
program.

---

## qpdf — the PDF engine

| | |
|---|---|
| **Component** | qpdf — `qpdf.exe` plus the DLLs beside it (Windows), or `qpdf` plus its dylibs (macOS) |
| **Licence** | **Apache License 2.0** — full text in [`LICENSES/qpdf-Apache-2.0.txt`](LICENSES/qpdf-Apache-2.0.txt) |
| **Copyright** | © Jay Berkenbilt and the qpdf contributors |
| **Upstream** | <https://qpdf.readthedocs.io/> · source: <https://github.com/qpdf/qpdf> |
| **Version shipped** | pinned by `scripts/provision-qpdf-macos.sh` / `scripts/provision-qpdf-windows.ps1` (currently 12.4.1) |
| **Modified?** | No. The files are redistributed exactly as published upstream. |

Rocktier PDF Squeeze runs qpdf as a **separate process** to compress a PDF; no
qpdf code is linked into the application binary. Apache-2.0 is permissive:
there is no copyleft obligation on this application and no source-disclosure
requirement beyond keeping these notices with the program.

The Windows package uses qpdf's official MSVC build, downloaded and checked
against the SHA-256 that qpdf publishes alongside it. The macOS package collects
qpdf and its dylibs with `@loader_path` rewrites so the sidecar is fully
self-contained.

### Why the engine changed (Ghostscript, used until 2026-09-19)

Earlier releases shipped **Ghostscript 10.07 / 10.08** as the engine, licensed
**AGPL-3.0-or-later**. Redistributing AGPL software inside a product that is not
itself AGPL is a contradiction, and it was that contradiction — not a defect —
that forced this product off the Microsoft Store. Those binaries are no longer
part of this project's source tree or packages, and their licence texts are no
longer shipped.

Two further reasons made the switch worth more than compliance:

- **Fidelity.** qpdf rewrites image streams and object structure only; it never
  touches the content stream, so the text layer survives byte for byte.
  Ghostscript re-rasterises whole pages and substitutes fonts, which puts "text
  stays selectable / searchable" — a criterion this product is judged on — at
  risk.
- **No conflict.** Apache-2.0 imposes nothing that conflicts with how this
  product is distributed.

The trade-off is honest and known: neither qpdf nor our own image pass changes
pixel dimensions, so PDFs whose images are already JPEG (scans, photo
catalogues) compress less than they did under Ghostscript, which reached 39% on
such a file purely by reducing 200 ppi to 150 dpi. What our image pass does
instead is re-encode the images qpdf leaves alone at the profile's quality — and
that turned out to matter more than the dimensions did. Measured on three real
documents: **51%** on a scanned catalogue, **82%** and **90%** on two papers
whose figures were stored uncompressed. In every case the page count, the
bookmarks and the text layer came through byte for byte — the text layer is
asserted as such in the test suite, because "it still opens" is not the same
claim as "nothing inside changed".

### Libraries that arrive inside the qpdf distribution

The official builds bundle third-party libraries — notably OpenSSL
(`libcrypto`), libjpeg-turbo and zlib — redistributed unchanged as part of the
qpdf distribution. Their notices are published in the upstream qpdf release for
the exact version named above. On macOS the provisioning script collects them
next to the binary; on Windows they ship as DLLs beside `qpdf.exe`.

If you need any of those texts separately, we will supply them on request —
write to **hello@rocktier.com**.

---

## Pdfium — the rendering engine

| | |
|---|---|
| **Component** | Pdfium — `libpdfium.dylib` (macOS) and `pdfium.dll` (Windows), shipped in `resources/pdfium-runtime/` |
| **Licence** | **BSD-3-Clause** for Pdfium itself, plus **Apache License 2.0** and other permissive licences for the components compiled into the same binary — full text in [`LICENSES/pdfium.txt`](LICENSES/pdfium.txt) |
| **Copyright** | © The PDFium Authors |
| **Upstream** | <https://pdfium.googlesource.com/pdfium> · prebuilt binaries: <https://github.com/bblanchon/pdfium-binaries> (MIT — [`LICENSES/pdfium-binaries-MIT.txt`](LICENSES/pdfium-binaries-MIT.txt)) |
| **Version shipped** | pinned by `PDFIUM_BUILD` in `scripts/fetch-pdfium.mjs` |
| **Modified?** | No. The library is redistributed exactly as published; we neither rebuild nor patch it. |

Pdfium renders PDF pages for the viewer. It is loaded as a **dynamic library at
runtime** — no Pdfium code is linked into the application binary.

The prebuilt libraries come from the [`pdfium-binaries`](https://github.com/bblanchon/pdfium-binaries)
project (MIT licence, © Benoit Blanchon), which builds upstream Pdfium and
applies packaging patches to its **build files only**, not to Pdfium's sources.
That build compiles in further components — V8, libjpeg-turbo, zlib, FreeType,
LCMS, OpenJPEG and others — whose notices are aggregated in the upstream
`LICENSE` file reproduced verbatim in `LICENSES/pdfium.txt`.

As with the libraries inside the qpdf distribution, if you need any of those
texts separately, we will supply them on request — write to
**hello@rocktier.com**.

---

## Trademarks

Rocktier PDF Squeeze is an independent product. It is not affiliated with,
endorsed by, or sponsored by Artifex Software, Inc., or by any other vendor
mentioned here. All trademarks are the property of their respective owners.

PDF is an ISO standard (ISO 32000); no Adobe trademark is claimed by this
project.

---

## Your files

PDFs are processed entirely on your machine. Nothing is uploaded, and the
application makes no network requests of its own — with one exception, described
in `PRIVACY.md`: entering an activation code in the website build sends that code
to `rocktier.com` once. The Microsoft Store build has no such dialog.

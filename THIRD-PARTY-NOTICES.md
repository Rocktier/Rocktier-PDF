# Third-party notices — Rocktier PDF Squeeze

Rocktier PDF Squeeze is proprietary software. It **redistributes** the
third-party components listed below, unmodified. This file and the full licence
texts in `LICENSES/` ship inside every installer and Microsoft Store package
(look for `resources/legal/` next to the application), so the notices travel
with the program.

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

The trade-off is honest and known: qpdf has **no downsampling option**, so PDFs
whose images are already JPEG (scans, photo catalogues) compress less than they
did under Ghostscript. Measured on four real documents: **79–89% smaller** for
exports carrying uncompressed image data, **4%** for an already-JPEG catalogue
where Ghostscript reached 39% purely by reducing 200 ppi to 150 dpi. Closing
that gap requires our own downsampler; it is tracked as the next step.

### Libraries that arrive inside the qpdf distribution

The official builds bundle third-party libraries — notably OpenSSL
(`libcrypto`), libjpeg-turbo and zlib — redistributed unchanged as part of the
qpdf distribution. Their notices are published in the upstream qpdf release for
the exact version named above. On macOS the provisioning script collects them
next to the binary; on Windows they ship as DLLs beside `qpdf.exe`.

If you need any of those texts separately, we will supply them on request —
write to **hello@rocktier.com**.

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
application makes no network requests of its own.

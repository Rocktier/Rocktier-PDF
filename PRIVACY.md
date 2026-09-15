# Privacy

**Rocktier PDF Editor does not have a network stack.**

Not "we don't collect data" — there is no code in the application capable of
sending anything anywhere.

## What is true

| | |
|---|---|
| Network requests | **None.** No HTTP client is compiled into the binary. |
| Analytics / telemetry | **None.** |
| Crash reporting | **None.** |
| Accounts / sign-in | **None.** There is no such screen. |
| File uploads | **None.** Files never leave your disk. |
| Tracking cookies | Not a web app. |
| Ads / third-party SDKs | **None.** |

## What the app actually does

1. You pick a PDF (or drop one on the window).
2. The bundled Pdfium engine reads it from disk into memory.
3. Edits — reorder, rotate, delete, merge, split — happen in memory.
4. You choose where to save; the file is written back to your disk.

Page bitmaps are handed to the interface as in-memory data URLs over a local
IPC channel. They are never written to a temp file and never transmitted.

## The one network call

`scripts/fetch-pdfium.mjs` downloads the Pdfium library **when you build the
app from source**. It runs on your build machine, at build time, from a public
GitHub repository. The released application contains that library and makes no
further requests.

## Permissions

The app asks the operating system for nothing beyond:

- Read/write access to the files you explicitly open or save.
- The native file picker.
- Window controls.

## Verification

The binary contains no networking code and the Tauri capability set grants no
network permission. You can audit it yourself — the source is public at
<https://github.com/Rocktier/Rocktier-PDF-editor>.

If you find a request leaving this app, it is a bug. Please report it.

---

*Last updated: 2026-09-15*

# Privacy

**Rocktier PDF never sends anything on its own.**

Not "we don't collect data" — nothing leaves the machine unless you ask for it.
There is exactly one thing you can ask for, and it is described below.

## What is true

| | |
|---|---|
| Network requests | **None, unless you activate a license.** That one request is described under "The two network calls". |
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

## The two network calls

**1. Fetching Pdfium — build time, not in the app you run.**

`scripts/fetch-pdfium.mjs` downloads the Pdfium library **when you build the
app from source**. It runs on your build machine, at build time, from a public
GitHub repository. The released application contains that library already.

**2. Activation — only when you enter a code, only in the website build.**

If you bought from `rocktier.com`, the app has a **License** dialog where you can
paste an activation code. Pressing *Activate* sends that code to
`rocktier.com/api/activate` once; the reply is a signed receipt that is stored
locally and checked offline from then on. Nothing else is sent — not the code
again, not the document, not any identifier. The app works offline forever after.

Two things follow from that, and both are deliberate:

- **The code is checked on the server, not in the app.** Checking it locally
  would mean shipping the signing secret inside the application, which would let
  anyone mint their own codes. The app only ever holds a **public** key.
- **The Microsoft Store build has none of this.** It shows no license dialog and
  makes no such request: the Store sold it and the Store knows it. If you are
  running the Store version, this section does not apply to you at all.

## Permissions

The app asks the operating system for nothing beyond:

- Read/write access to the files you explicitly open or save.
- The native file picker.
- Window controls.

## Verification

The Rust binary contains no networking code and the Tauri capability set grants no
network permission; the only way out of the application is the activation request
made by the webview, which the content-security policy restricts to
`rocktier.com`. You can audit both yourself — the source is public at
<https://github.com/Rocktier/Rocktier-PDF>.

If you find a request leaving this app, it is a bug. Please report it.

---

*Last updated: 2026-09-15*

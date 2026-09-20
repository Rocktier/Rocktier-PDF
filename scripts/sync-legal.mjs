#!/usr/bin/env node
/**
 * Stages the third-party notices and licence texts as bundle resources, so they
 * actually ship inside the installers.
 *
 * Why a copy rather than a `"../THIRD-PARTY-NOTICES.md"` entry in
 * `bundle.resources`: Tauri maps a `..` segment to a `_up_/` folder. The macOS
 * bundler keeps that, but `tauri-windows-bundle` collects only `resources/**`
 * and drops everything else **without a word** — which is how the Microsoft
 * Store package ended up shipping Ghostscript (AGPL-3.0) with no licence and no
 * notices at all, the exact thing AGPL section 4 forbids.
 *
 * This mechanism was written for the Squeeze-era build and was lost when the two
 * PDF products merged: `bundle.resources` kept only the binaries and
 * `beforeBuildCommand` stopped calling this script — so qpdf (Apache-2.0) and
 * Pdfium (arriving as libpdfium.dylib / pdfium.dll / libpdfium.so) were being
 * redistributed with their notices sitting in the repository instead of inside
 * the package, while `THIRD-PARTY-NOTICES.md` still claimed they travelled with
 * the program. Restored from the archived Squeeze source.
 *
 * Files under `resources/` land at `resources/legal/…` on every platform, so one
 * mechanism covers the DMG, the MSI and the MSIX.
 *
 * Runs from `beforeBuildCommand`, so the copies cannot drift from the files at
 * the repository root — those stay the single source of truth.
 */
import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dest = join(root, "src-tauri", "resources", "legal");
const licDest = join(dest, "LICENSES");

mkdirSync(licDest, { recursive: true });

copyFileSync(join(root, "THIRD-PARTY-NOTICES.md"), join(dest, "THIRD-PARTY-NOTICES.md"));
console.log("legal -> resources/legal/THIRD-PARTY-NOTICES.md");

const licSrc = join(root, "LICENSES");
for (const name of readdirSync(licSrc)) {
  if (!name.endsWith(".txt")) continue;
  copyFileSync(join(licSrc, name), join(licDest, name));
  console.log(`legal -> resources/legal/LICENSES/${name}`);
}

#!/usr/bin/env node
/**
 * Rocktier PDF Editor — fetch prebuilt Pdfium binaries.
 *
 * Downloads the platform-appropriate Pdfium dynamic library from
 * https://github.com/bblanchon/pdfium-binaries and places it in
 * src-tauri/resources/pdfium/<triple>/ so it can be bundled with the app.
 *
 * Usage:
 *   node scripts/fetch-pdfium.mjs              # current platform only
 *   node scripts/fetch-pdfium.mjs --all        # every supported platform
 *   node scripts/fetch-pdfium.mjs --force      # re-download even if present
 *
 * Offline note: this is the ONLY step that touches the network, and it runs at
 * build time — never at app runtime. Once bundled, the app is 100% offline.
 */

import { execFileSync } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { copyFile, mkdir, readdir, rm, stat } from 'node:fs/promises';
import { get } from 'node:https';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const OUT_ROOT = join(ROOT, 'src-tauri', 'resources', 'pdfium');

/** Tauri target triples we ship for. */
const TARGETS = {
  'windows-x64': { host: 'win32', arch: 'x64', asset: /^pdfium-win-x64\.tgz$/, file: 'pdfium.dll' },
  'macos-arm64': { host: 'darwin', arch: 'arm64', asset: /^pdfium-mac-arm64\.tgz$/, file: 'libpdfium.dylib' },
  'macos-x64': { host: 'darwin', arch: 'x64', asset: /^pdfium-mac-x64\.tgz$/, file: 'libpdfium.dylib' },
  'linux-x64': { host: 'linux', arch: 'x64', asset: /^pdfium-linux-x64\.tgz$/, file: 'libpdfium.so' },
};

const API = 'https://api.github.com/repos/bblanchon/pdfium-binaries/releases/latest';

function httpsGet(url, redirects = 0) {
  return new Promise((ok, fail) => {
    const req = get(
      url,
      { headers: { 'User-Agent': 'rocktier-pdf-editor', Accept: 'application/vnd.github+json' } },
      (res) => {
        const { statusCode, headers } = res;
        if (statusCode >= 300 && statusCode < 400 && headers.location && redirects < 5) {
          res.resume();
          return httpsGet(headers.location, redirects + 1).then(ok, fail);
        }
        if (statusCode !== 200) {
          res.resume();
          return fail(new Error(`HTTP ${statusCode} for ${url}`));
        }
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => ok(Buffer.concat(chunks)));
      }
    );
    req.on('error', fail);
    req.setTimeout(30000, () => req.destroy(new Error('timeout')));
  });
}

function download(url, dest) {
  return new Promise((ok, fail) => {
    const req = get(url, { headers: { 'User-Agent': 'rocktier-pdf-editor' } }, (res) => {
      const { statusCode, headers } = res;
      if (statusCode >= 300 && statusCode < 400 && headers.location) {
        res.resume();
        return download(headers.location, dest).then(ok, fail);
      }
      if (statusCode !== 200) {
        res.resume();
        return fail(new Error(`HTTP ${statusCode} downloading ${url}`));
      }
      const ws = createWriteStream(dest);
      res.pipe(ws);
      ws.on('finish', () => ws.close(ok));
      ws.on('error', fail);
    });
    req.on('error', fail);
    req.setTimeout(120000, () => req.destroy(new Error('download timeout')));
  });
}

async function findLib(dir, filename) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      const found = await findLib(p, filename);
      if (found) return found;
    } else if (e.name === filename || /^libpdfium\.(dylib|so)(\.\d+)?$/.test(e.name) || /^pdfium\.dll$/.test(e.name)) {
      return p;
    }
  }
  return null;
}

async function fetchTarget(key, release, force) {
  const t = TARGETS[key];
  const destDir = join(OUT_ROOT, key);
  const destFile = join(destDir, t.file);

  if (!force) {
    try {
      const s = await stat(destFile);
      if (s.size > 100_000) return { key, skipped: true, size: s.size };
    } catch {
      /* not present, download */
    }
  }

  const asset = release.assets.find((a) => t.asset.test(a.name));
  if (!asset) throw new Error(`No release asset matching ${t.asset} in ${release.tag_name}`);

  const tmp = join(tmpdir(), `pdfium-${key}-${Date.now()}`);
  const archive = `${tmp}.tgz`;
  await mkdir(tmp, { recursive: true });
  await download(asset.browser_download_url, archive);

  // Windows 10+ and macOS both ship a BSD-compatible tar.
  execFileSync('tar', ['-xzf', archive, '-C', tmp], { stdio: 'pipe' });

  const lib = await findLib(tmp, t.file);
  if (!lib) throw new Error(`Extracted archive for ${key} contains no pdfium library`);

  await mkdir(destDir, { recursive: true });
  // copyFile (not rename): temp dir and project often live on different drives.
  await copyFile(lib, destFile);
  await rm(tmp, { recursive: true, force: true });
  await rm(archive, { force: true });

  const s = await stat(destFile);
  return { key, skipped: false, size: s.size };
}

async function main() {
  const args = process.argv.slice(2);
  const all = args.includes('--all');
  const force = args.includes('--force');

  const currentKey = Object.keys(TARGETS).find(
    (k) => TARGETS[k].host === process.platform && TARGETS[k].arch === process.arch
  );
  const keys = all ? Object.keys(TARGETS) : [currentKey];

  if (!currentKey && !all) {
    console.error(`Unsupported platform ${process.platform}/${process.arch}. Use --all to fetch anyway.`);
    process.exit(1);
  }

  console.log('Fetching release metadata from bblanchon/pdfium-binaries ...');
  const release = JSON.parse((await httpsGet(API)).toString('utf8'));
  console.log(`Latest release: ${release.tag_name}`);

  const results = [];
  for (const key of keys) {
    if (!key) continue;
    results.push(await fetchTarget(key, release, force));
  }

  for (const r of results) {
    const mb = (r.size / 1024 / 1024).toFixed(2);
    console.log(`  ${r.skipped ? '=' : '+'} ${r.key.padEnd(12)} ${r.skipped ? 'up to date' : 'installed'} (${mb} MB)`);
  }
  console.log('Done. Pdfium binaries are in src-tauri/resources/pdfium/');
}

main().catch((e) => {
  console.error('fetch-pdfium failed:', e.message);
  process.exit(1);
});

#!/usr/bin/env node
// Vendoring helper for hls.js — the ~100 KB player shim non-Safari browsers need
// (SPEC §6). It is NOT committed to the repo and NOT fetched silently: this script
// is run explicitly (at setup, M5, or by hand) and pins an exact version + SHA-256,
// so what lands on disk is reproducible and verifiable. Self-hosting the file (vs a
// CDN <script>) keeps playback dependency-free and leaks no viewer IPs to a third
// party — the whole self-host story.
//
// Usage:  node fetch-hls.mjs [--out <path>]   (default: ./hls.min.js next to this)
//
// To bump the version: change PIN below, run once, copy the printed actual hash
// into PIN.sha256, and re-run to confirm it verifies.

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const PIN = {
  version: '1.5.20',
  url: 'https://cdn.jsdelivr.net/npm/hls.js@1.5.20/dist/hls.min.js',
  // SHA-256 of the fetched bytes. Empty ⇒ first run prints the hash to paste here
  // (we don't ship an unverified hash; you pin it the first time you fetch).
  // Pinned 2026-06-27 after cross-checking the hash across jsdelivr + unpkg.
  sha256: 'd016c1230496ee59f3f5b01c16cce4cc01b5a1d3d357adec200c908b131ebe49',
};

const HERE = path.dirname(fileURLToPath(import.meta.url));

function parseOut(argv) {
  const i = argv.indexOf('--out');
  return i >= 0 && argv[i + 1] ? path.resolve(argv[i + 1]) : path.join(HERE, 'hls.min.js');
}

async function main() {
  const out = parseOut(process.argv.slice(2));
  process.stderr.write(`Fetching hls.js ${PIN.version} from ${PIN.url}\n`);

  const res = await fetch(PIN.url);
  if (!res.ok) {
    process.stderr.write(`Fetch failed: HTTP ${res.status}\n`);
    process.exit(1);
  }
  const bytes = Buffer.from(await res.arrayBuffer());
  const got = crypto.createHash('sha256').update(bytes).digest('hex');

  if (!PIN.sha256 || /^(.)\1+$/.test(PIN.sha256)) {
    process.stderr.write(`No pinned hash set. Actual SHA-256:\n  ${got}\nPaste it into PIN.sha256 and re-run.\n`);
    process.exit(2);
  }
  if (got !== PIN.sha256) {
    process.stderr.write(`SHA-256 MISMATCH — refusing to write.\n  expected ${PIN.sha256}\n  got      ${got}\n`);
    process.exit(3);
  }

  fs.writeFileSync(out, bytes);
  process.stderr.write(`Verified ✓ wrote ${bytes.length} bytes → ${out}\n`);
}

main().catch((e) => {
  process.stderr.write(`fetch-hls fatal: ${e?.stack || e}\n`);
  process.exit(1);
});

#!/usr/bin/env node
// build-page — finalize → per-video static page (SPEC §6, milestone M4).
//
// Deterministic: one template → one static HTML page with baked og: tags, plus a
// poster.jpg, written into a per-video output dir ready to deploy to Cloudflare
// Pages. Generation touches NOTHING on the network; the only cloud step (the
// actual `wrangler pages deploy`) lands in M5. Re-runnable / idempotent.
//
// Inputs come from two places (SPEC §3 substrate split):
//   --session <dir> : the recording dir (~/.shroom/recordings/<id>) — preview.mp4
//                     for the poster, events.ndjson for id + durationSec fallback.
//   --meta <file>   : the git-library `<id>.md` — title / tldr / chapters the agent
//                     authored (a skill, M5). Optional; sensible defaults without it.
//
// Usage:
//   node build-page.mjs --session <dir> [--meta <id.md>] [--out <siteDir>]
//                       [--id <id>] [--public-base <url>] [--pages-base <url>]
//                       [--hlsjs-url <path>]
// Output: <out>/<id>/index.html (+ poster.jpg). Prints a JSON summary on stdout.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { renderPage } from './lib/render.mjs';
import { readMetadataFile } from './lib/metadata.mjs';
import { loadPageConfig, urlsFor } from './lib/page-config.mjs';
import { generatePoster } from './lib/poster.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const TEMPLATE = path.resolve(HERE, '../../templates/player.html');

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : 'true';
  }
  return o;
}

// Pull id + durationSec out of a recording's events.ndjson (session_started /
// finalized). The metadata file wins if it also carries them.
function readSessionFacts(sessionDir) {
  const facts = {};
  const p = path.join(sessionDir, 'events.ndjson');
  if (!fs.existsSync(p)) return facts;
  for (const line of fs.readFileSync(p, 'utf8').split('\n')) {
    if (!line.trim()) continue;
    let e;
    try { e = JSON.parse(line); } catch { continue; }
    if (e.event === 'session_started' && e.id) facts.id = e.id;
    if (e.event === 'finalized' && typeof e.durationSec === 'number') facts.durationSec = e.durationSec;
  }
  return facts;
}

function die(msg, code = 2) {
  process.stderr.write(msg + '\n');
  process.exit(code);
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  const sessionDir = opts.session ? path.resolve(opts.session) : null;
  if (sessionDir && !fs.existsSync(sessionDir)) die(`No such session dir: ${sessionDir}`);

  const facts = sessionDir ? readSessionFacts(sessionDir) : {};
  const fileMeta = opts.meta ? readMetadataFile(path.resolve(opts.meta)) : null;
  const meta = { ...(fileMeta?.meta ?? {}) };

  const id = opts.id !== undefined && opts.id !== 'true' ? opts.id : (meta.id ?? facts.id);
  if (!id) die('Could not determine recording id (pass --id, --meta, or --session with events.ndjson).');
  meta.id = id;
  if (meta.durationSec == null && facts.durationSec != null) meta.durationSec = facts.durationSec;

  const cfg = loadPageConfig({
    overrides: {
      publicBaseUrl: opts['public-base'] !== 'true' ? opts['public-base'] : undefined,
      pagesBaseUrl: opts['pages-base'] !== 'true' ? opts['pages-base'] : undefined,
      hlsJsUrl: opts['hlsjs-url'] !== 'true' ? opts['hlsjs-url'] : undefined,
    },
  });
  const urls = urlsFor(cfg, id, meta.mp4);

  const outRoot = path.resolve(opts.out ?? path.join(os.homedir(), '.shroom', 'site'));
  const pageDir = path.join(outRoot, id);
  fs.mkdirSync(pageDir, { recursive: true });

  // Poster (best-effort) from the local preview.mp4.
  let poster = { ok: false, reason: 'no_session' };
  if (sessionDir) {
    poster = await generatePoster({
      previewPath: path.join(sessionDir, 'preview.mp4'),
      outPath: path.join(pageDir, 'poster.jpg'),
      durationSec: meta.durationSec,
    });
  }
  // If no poster could be made, don't advertise one that 404s.
  const pageUrls = poster.ok ? urls : { ...urls, posterUrl: '' };

  const template = fs.readFileSync(TEMPLATE, 'utf8');
  const html = renderPage({ template, meta, urls: pageUrls });
  const indexPath = path.join(pageDir, 'index.html');
  fs.writeFileSync(indexPath, html);

  const summary = {
    event: 'page_built',
    id,
    indexPath,
    pageDir,
    poster: poster.ok ? path.join(pageDir, 'poster.jpg') : null,
    posterSkipped: poster.ok ? undefined : poster.reason,
    playbackUrl: urls.pageUrl || null,
    hlsUrl: urls.hlsUrl,
    configured: Boolean(cfg.publicBaseUrl && cfg.pagesBaseUrl),
  };
  process.stdout.write(JSON.stringify(summary) + '\n');
}

main().catch((e) => {
  process.stderr.write(`build-page fatal: ${e?.stack || e}\n`);
  process.exit(1);
});

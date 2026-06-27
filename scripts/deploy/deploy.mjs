#!/usr/bin/env node
// deploy — put the per-video pages live on Cloudflare Pages (SPEC §6, milestone M5).
//
// Deterministic publish step: place the shared hls.min.js once at the site root,
// `wrangler pages deploy` the whole site bundle build-page produced, then emit a
// `published` event carrying the shareable playback URL. The agent runs this at
// publish time (post-stop), after it has authored title/chapters and build-page
// has generated the page. Idempotent — re-running just re-deploys (no-op pages
// unchanged). Standalone / recovery entry point, mirroring upload.mjs.
//
// Usage:
//   node deploy.mjs --project <pages-project> [--id <id>] [--site <dir>]
//                   [--branch <name>] [--session <dir>] [--pages-base <url>]
//                   [--wrangler <bin>] [--force-hlsjs]
// Defaults: --site ~/.shroom/site, --branch main, --pages-base from page-config.
// Auth: the `wrangler login` OAuth session from setup (no token paste).
// Output: ndjson events on stdout; appended to <session>/events.ndjson if --session.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { runDeploy } from './lib/deploy.mjs';
import { spawnWrangler } from './lib/wrangler.mjs';
import { loadPageConfig } from '../page/lib/page-config.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const VENDOR_HLS = path.resolve(HERE, '../page/vendor/hls.min.js');

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : 'true';
  }
  return o;
}

function readIdFromEvents(dir) {
  const p = path.join(dir, 'events.ndjson');
  if (!fs.existsSync(p)) return null;
  for (const line of fs.readFileSync(p, 'utf8').split('\n')) {
    if (!line.trim()) continue;
    try {
      const e = JSON.parse(line);
      if (e.event === 'session_started' && e.id) return e.id;
    } catch { /* skip malformed line */ }
  }
  return null;
}

function die(msg, code = 2) {
  process.stderr.write(msg + '\n');
  process.exit(code);
}

const opts = parseArgs(process.argv.slice(2));
const flag = (k) => (opts[k] !== undefined && opts[k] !== 'true' ? opts[k] : undefined);

const project = flag('project');
if (!project) die('A Pages project is required: --project <name>.', 2);

const siteDir = path.resolve(opts.site && opts.site !== 'true' ? opts.site : path.join(os.homedir(), '.shroom', 'site'));
if (!fs.existsSync(siteDir)) die(`No such site dir: ${siteDir}. Run build-page first.`, 2);

const sessionDir = flag('session') ? path.resolve(flag('session')) : null;
const id = flag('id') ?? (sessionDir ? readIdFromEvents(sessionDir) : null);

const pageConfig = loadPageConfig({ overrides: { pagesBaseUrl: flag('pages-base') } });

// Emit each event to stdout; if a session dir is given, also append it to the
// session's events.ndjson — the durable pending-publish artifact drained by the
// next /shroom run (SPEC §6).
const eventsFile = sessionDir ? path.join(sessionDir, 'events.ndjson') : null;
const log = (event, fields = {}) => {
  const line = JSON.stringify({ ts: new Date().toISOString(), event, ...fields }) + '\n';
  process.stdout.write(line);
  if (eventsFile) {
    try { fs.appendFileSync(eventsFile, line); } catch { /* best-effort durability */ }
  }
};

const runWrangler = (args) => spawnWrangler(args, { bin: flag('wrangler') ?? 'wrangler' });

const result = await runDeploy({
  siteDir,
  projectName: project,
  id,
  branch: flag('branch') ?? 'main',
  pageConfig,
  vendorPath: VENDOR_HLS,
  runWrangler,
  log,
});

if (!result.ok && result.message) process.stderr.write(result.message + '\n');
process.exit(result.ok ? 0 : 1);

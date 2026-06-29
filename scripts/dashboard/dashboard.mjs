#!/usr/bin/env node
// dashboard — the deterministic backend for the `dashboard` skill (green-set
// item 7): one place that lists every recording with a thumbnail, its public
// link, and its local footprint, so it can be the entry point for management
// actions (open / re-title / clean up).
//
// It MERGES two substrates (SPEC §3): the git library (`<id>.md` records — the
// canonical title/duration/chapters/link list) and the local recordings dir (disk
// footprint + per-session state, via the cleanup scan). Mechanism only — the skill
// decides what to do with what it shows.
//
// Subcommands:
//   data [--json]          The merged list as JSON.
//   build [--out <dir>]    Render a self-contained static HTML dashboard (cards +
//                          thumbnails copied in) and print its path; open it to browse.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { parseMetadata } from '../page/lib/metadata.mjs';
import { formatDuration } from '../page/lib/render.mjs';
import { scanSessions } from '../cleanup/cleanup.mjs';
import { readCreds, credsPath } from '../setup/lib/credentials.mjs';

const HOME = os.homedir();
const DEFAULT_SITE_ROOT = path.join(HOME, '.shroom', 'site');
const DEFAULT_DASHBOARD_DIR = path.join(HOME, '.shroom', 'dashboard');

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : true;
  }
  return o;
}

// Read every `<id>.md` in the library → { id: meta }.
function readLibrary(libraryDir) {
  const out = {};
  let files;
  try { files = fs.readdirSync(libraryDir); } catch { return out; }
  for (const f of files) {
    if (!f.endsWith('.md') || f === 'README.md') continue;
    try {
      const { meta } = parseMetadata(fs.readFileSync(path.join(libraryDir, f), 'utf8'));
      const id = meta.id || f.replace(/\.md$/, '');
      out[id] = { ...meta, id };
    } catch { /* skip an unreadable record */ }
  }
  return out;
}

// Pure merge of library records + local sessions → the dashboard item list.
// Exposed for testing (no filesystem/credentials assumptions baked in).
export function buildDashboardItems({ library = {}, sessions = [], pagesBaseUrl = '', siteRoot = DEFAULT_SITE_ROOT } = {}) {
  const byId = new Map();
  const ensure = (id) => {
    if (!byId.has(id)) byId.set(id, { id, inLibrary: false, local: null });
    return byId.get(id);
  };

  for (const [id, meta] of Object.entries(library)) {
    const it = ensure(id);
    it.inLibrary = true;
    it.title = meta.title || 'Untitled recording';
    it.tldr = meta.tldr || '';
    it.durationSec = Number(meta.durationSec) || 0;
    it.createdAt = meta.createdAt || null;
    it.chapters = Array.isArray(meta.chapters) ? meta.chapters.length : 0;
    it.mp4 = meta.mp4 === true;
  }

  for (const s of sessions) {
    const it = ensure(s.id);
    it.local = {
      dir: s.dir, totalBytes: s.totalBytes, prunableBytes: s.prunableBytes,
      hasPreviewMp4: s.hasPreviewMp4, hasLocalHls: s.hasLocalHls, published: s.published,
    };
    if (it.createdAt == null) it.createdAt = s.createdAt || null;
    if (it.durationSec == null) it.durationSec = 0;
    if (it.title == null && !it.inLibrary) it.title = 'Untitled recording';
    if (!it.inLibrary && s.playbackUrl) it._sessionUrl = s.playbackUrl;
  }

  const items = [...byId.values()].map((it) => {
    const sessionUrl = it._sessionUrl || null;
    delete it._sessionUrl;
    // Only offer a link when there's positive evidence the page is live: a local
    // `published` event, a committed library record on a site-configured account,
    // or a recorded playback URL. An unpublished local-only take has no live page,
    // so we don't hand out a URL that would 404.
    const live = it.local?.published === true || (it.inLibrary && !!pagesBaseUrl) || !!sessionUrl;
    const link = !live ? null : (pagesBaseUrl ? `${pagesBaseUrl.replace(/\/+$/, '')}/${it.id}/` : sessionUrl);
    const posterPath = path.join(siteRoot, it.id, 'poster.jpg');
    return {
      ...it,
      title: it.title || 'Untitled recording',
      durationSec: it.durationSec || 0,
      chapters: it.chapters || 0,
      mp4: it.mp4 || false,
      live,
      link,
      poster: fs.existsSync(posterPath) ? posterPath : null,
    };
  });

  items.sort((a, b) => (Date.parse(b.createdAt || 0) || 0) - (Date.parse(a.createdAt || 0) || 0));
  return items;
}

function gatherData(opts) {
  const creds = readCreds(credsPath());
  const libraryDir = opts.library ? path.resolve(String(opts.library)) : (creds.library || path.join(HOME, 'shroom'));
  const items = buildDashboardItems({
    library: readLibrary(libraryDir),
    sessions: scanSessions(),
    pagesBaseUrl: creds.pagesBaseUrl || '',
    siteRoot: DEFAULT_SITE_ROOT,
  });
  return {
    ok: true,
    library: libraryDir,
    pagesBaseUrl: creds.pagesBaseUrl || null,
    count: items.length,
    totalLocalBytes: items.reduce((n, i) => n + (i.local?.totalBytes || 0), 0),
    prunableBytes: items.reduce((n, i) => n + (i.local?.prunableBytes || 0), 0),
    items,
  };
}

// --- static HTML dashboard -------------------------------------------------

const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ESC[c]);
const fmtBytes = (n) => {
  n = Number(n) || 0;
  if (n < 1024) return n + ' B';
  const u = ['KB', 'MB', 'GB', 'TB'];
  let i = -1;
  do { n /= 1024; i++; } while (n >= 1024 && i < u.length - 1);
  return `${n.toFixed(n < 10 ? 1 : 0)} ${u[i]}`;
};
const fmtDate = (iso) => {
  const d = iso ? new Date(iso) : null;
  return d && !isNaN(d) ? d.toISOString().slice(0, 10) : '';
};

// Render the dashboard HTML. `items` carry a `thumb` (relative path or null).
export function renderDashboard(items, { generatedAt = new Date().toISOString() } = {}) {
  const cards = items.map((it) => {
    const thumb = it.thumb
      ? `<img class="thumb" src="${esc(it.thumb)}" alt="" loading="lazy" />`
      : `<div class="thumb placeholder">${esc((it.title || '?').trim().charAt(0).toUpperCase() || '?')}</div>`;
    const dur = it.durationSec ? `<span class="dur">${esc(formatDuration(it.durationSec))}</span>` : '';
    const date = it.createdAt ? `<span>${esc(fmtDate(it.createdAt))}</span>` : '';
    const ch = it.chapters ? `<span>${it.chapters} chapters</span>` : '';
    const mp4 = it.mp4 ? `<span class="tag">MP4</span>` : '';
    const status = it.link ? '' : `<span class="muted">not published</span>`;
    const local = it.local
      ? `<span title="local footprint">${esc(fmtBytes(it.local.totalBytes))} on disk${it.local.prunableBytes ? ` · ${esc(fmtBytes(it.local.prunableBytes))} prunable` : ''}</span>`
      : `<span class="muted">no local copy</span>`;
    const titleEl = it.link
      ? `<a href="${esc(it.link)}" target="_blank" rel="noopener">${esc(it.title)}</a>`
      : esc(it.title);
    return `      <article class="card">
        <div class="media">${it.link ? `<a href="${esc(it.link)}" target="_blank" rel="noopener">${thumb}</a>` : thumb}${dur}</div>
        <h2>${titleEl}</h2>
        <div class="meta">${[date, ch, mp4, status].filter(Boolean).join('<span class="sep">·</span>')}</div>
        <div class="meta">${local}</div>
      </article>`;
  }).join('\n');

  const total = items.reduce((n, i) => n + (i.local?.totalBytes || 0), 0);
  return `<!doctype html>
<html lang="en"><head>
<meta charset="utf-8" /><meta name="viewport" content="width=device-width, initial-scale=1" />
<title>shroom library</title>
<style>
  :root { color-scheme: dark; --fg:#ECEFF4; --muted:#9aa4b2; --bg:#2E3440; --card:#3B4252; --accent:#88C0D0; }
  * { box-sizing: border-box; }
  body { margin:0; background:var(--bg); color:var(--fg);
    font:15px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif; }
  .wrap { max-width:1100px; margin:0 auto; padding:28px 20px 60px; }
  header { display:flex; align-items:baseline; gap:12px; margin:0 0 20px; }
  h1 { font-size:22px; margin:0; }
  .count { color:var(--muted); }
  .grid { display:grid; grid-template-columns:repeat(auto-fill,minmax(260px,1fr)); gap:18px; }
  .card { background:var(--card); border-radius:12px; overflow:hidden; }
  .media { position:relative; aspect-ratio:16/9; background:#222831; }
  .media a { display:block; height:100%; }
  .thumb { width:100%; height:100%; object-fit:cover; display:block; }
  .thumb.placeholder { display:flex; align-items:center; justify-content:center;
    font-size:40px; font-weight:600; color:var(--accent); background:linear-gradient(135deg,#434c5e,#2e3440); }
  .dur { position:absolute; right:8px; bottom:8px; background:rgba(0,0,0,.7);
    padding:1px 7px; border-radius:6px; font-size:12px; font-variant-numeric:tabular-nums; }
  h2 { font-size:15px; margin:12px 12px 6px; font-weight:600; line-height:1.3; }
  h2 a { color:var(--fg); text-decoration:none; }
  h2 a:hover { color:var(--accent); }
  .meta { color:var(--muted); font-size:12.5px; margin:0 12px 8px;
    display:flex; flex-wrap:wrap; gap:6px; align-items:center; }
  .meta:last-child { margin-bottom:12px; }
  .sep { opacity:.5; }
  .tag { background:rgba(136,192,208,.18); color:var(--accent); border-radius:5px; padding:0 6px; font-size:11px; }
  .empty { color:var(--muted); padding:40px 0; text-align:center; }
  footer { color:var(--muted); font-size:12px; margin-top:28px; }
</style></head><body>
<div class="wrap">
  <header><h1>shroom library</h1><span class="count">${items.length} recording${items.length === 1 ? '' : 's'} · ${esc(fmtBytes(total))} on disk</span></header>
  ${items.length ? `<div class="grid">\n${cards}\n  </div>` : `<div class="empty">No recordings yet — run <code>/shroom:record</code>.</div>`}
  <footer>Generated ${esc(fmtDate(generatedAt))} · <a href="https://github.com/bagrat/shroom" style="color:var(--muted)">shroom</a></footer>
</div></body></html>
`;
}

function cmdBuild(opts) {
  const data = gatherData(opts);
  const outDir = opts.out ? path.resolve(String(opts.out)) : DEFAULT_DASHBOARD_DIR;
  const thumbsDir = path.join(outDir, 'thumbs');
  fs.mkdirSync(thumbsDir, { recursive: true });

  // Copy in each available poster so the page is self-contained + portable.
  const items = data.items.map((it) => {
    let thumb = null;
    if (it.poster) {
      const dest = path.join(thumbsDir, `${it.id}.jpg`);
      try { fs.copyFileSync(it.poster, dest); thumb = `thumbs/${it.id}.jpg`; } catch { /* skip */ }
    }
    return { ...it, thumb };
  });

  const html = renderDashboard(items);
  const indexPath = path.join(outDir, 'index.html');
  fs.writeFileSync(indexPath, html);
  return { ok: true, path: indexPath, count: items.length, library: data.library };
}

async function main() {
  const [sub, ...rest] = process.argv.slice(2);
  const opts = parseArgs(rest);
  let out;
  if (sub === 'data') out = gatherData(opts);
  else if (sub === 'build') out = cmdBuild(opts);
  else {
    process.stderr.write(`unknown subcommand: ${sub || '(none)'}\nexpected: data, build\n`);
    process.exit(2);
  }
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  process.exit(0);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((e) => {
    process.stdout.write(JSON.stringify({ ok: false, reason: 'error', detail: e.message }) + '\n');
    process.exit(1);
  });
}

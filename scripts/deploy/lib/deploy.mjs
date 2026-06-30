// The deploy step (SPEC §6, milestone M5) — the one and only cloud action for the
// *page* side of publish. The bytes (HLS segments) go to the bucket via the
// uploader (M3); this puts the per-video static pages live on Cloudflare Pages
// with `wrangler pages deploy`, then surfaces the shareable playback URL.
//
// Determinism boundary: deploying is pure mechanism → a script. The wrangler call
// is an injected seam (`runWrangler`) so the orchestration is testable offline
// (no wrangler, no Cloudflare account) — the real seam lives in ./wrangler.mjs.
//
// Two facts make a deploy "go live":
//   1. hls.min.js is placed ONCE at the site root (/hls.min.js), shared by every
//      per-video page — so the 20k-file Pages limit counts it once, not per video.
//   2. `wrangler pages deploy <siteDir>` ships the whole site bundle; the playback
//      URL for the just-published id is `<pagesBase>/<id>/`. wrangler is OAuth-
//      authed (the `wrangler login` session from setup), so no token paste.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// The shared favicons ship with the templates; placed at the site root on deploy.
// SVG is the primary (crisp, themeable); the PNG is a fallback for browsers that
// ignore SVG favicons — notably Safari — and doubles as the apple-touch-icon.
const TEMPLATES_DIR = path.join(
  path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..', 'templates',
);
const FAVICON_FILES = ['favicon.svg', 'favicon.png'];

const stripSlash = (s) => (typeof s === 'string' ? s.replace(/\/+$/, '') : s);

// Any *.pages.dev URL printed by wrangler. The unique deployment URL is printed
// last ("Take a peek over at https://<hash>.<project>.pages.dev"), so we keep the
// last match and trim trailing prose punctuation.
const PAGES_URL_RE = /https?:\/\/[^\s'"<>]*pages\.dev[^\s'"<>]*/g;

export function parseDeploymentUrl(text = '') {
  const matches = String(text).match(PAGES_URL_RE);
  if (!matches || !matches.length) return null;
  return matches[matches.length - 1].replace(/[.,)\]]+$/, '');
}

// The per-deploy URL is `https://<hash>.<project>.pages.dev`; the stable,
// shareable production base is `https://<project>.pages.dev` (project names can't
// contain dots, so a 4th label is always the deployment hash). Lets us derive a
// good link even before pagesBaseUrl is persisted at setup.
export function productionBaseFromDeployment(url) {
  if (!url) return null;
  try {
    const u = new URL(url);
    const parts = u.hostname.split('.');
    const host = parts.length >= 4 ? parts.slice(-3).join('.') : parts.join('.');
    return `${u.protocol}//${host}`;
  } catch {
    return null;
  }
}

// Place the vendored hls.min.js at the site root if it isn't already there. We
// copy from the explicitly-vendored, SHA-256-verified file (scripts/page/vendor/)
// rather than fetching here — vendoring is a deliberate step, never a silent
// network side effect (SPEC §6, working agreement: never silently mutate).
export function ensureHlsJs({ siteDir, vendorPath, force = false }) {
  const dest = path.join(siteDir, 'hls.min.js');
  if (!force && fs.existsSync(dest) && fs.statSync(dest).size > 0) {
    return { ok: true, placed: false, path: dest };
  }
  if (!fs.existsSync(vendorPath) || fs.statSync(vendorPath).size === 0) {
    return {
      ok: false,
      reason: 'no_vendor',
      message:
        `hls.min.js is not vendored. Fetch it first (pinned + SHA-256 verified):\n` +
        `  node scripts/page/vendor/fetch-hls.mjs --out ${vendorPath}`,
    };
  }
  fs.copyFileSync(vendorPath, dest);
  return { ok: true, placed: true, path: dest };
}

// Place the favicons at the site root (/favicon.svg + /favicon.png), shared by
// every per-video page (player.html links them absolutely). Best-effort: a
// missing source never blocks a deploy.
export function ensureFavicon({ siteDir, srcDir = TEMPLATES_DIR, files = FAVICON_FILES }) {
  let placed = false;
  for (const file of files) {
    try {
      const src = path.join(srcDir, file);
      if (!fs.existsSync(src)) continue;
      fs.copyFileSync(src, path.join(siteDir, file));
      placed = true;
    } catch {
      // best-effort per file
    }
  }
  return { ok: true, placed };
}

// Run the actual `wrangler pages deploy`. Production deploys target the project's
// production branch (default "main", matching our git default). `--commit-dirty`
// silences the dirty-tree prompt (the site bundle is generated, not a git repo).
export async function deployPages({ siteDir, projectName, branch = 'main', runWrangler, extraArgs = [], retries = 1, log = () => {} }) {
  if (typeof runWrangler !== 'function') throw new Error('deployPages: runWrangler is required');
  const args = [
    'pages', 'deploy', siteDir,
    `--project-name=${projectName}`,
    `--branch=${branch}`,
    '--commit-dirty=true',
    ...extraArgs,
  ];
  // Retry only a *timed-out* deploy — a fresh connection often clears a one-off
  // upload wedge. A real failure (auth, missing project) won't, so we don't retry it.
  let res;
  for (let attempt = 0; attempt <= retries; attempt++) {
    res = await runWrangler(args);
    if (res.code === 0 || !res.timedOut) break;
    if (attempt < retries) log('deploy_retry', { attempt: attempt + 1, reason: 'timeout' });
  }
  const deploymentUrl = res.code === 0 ? parseDeploymentUrl(`${res.stdout ?? ''}\n${res.stderr ?? ''}`) : null;
  return {
    ok: res.code === 0 && Boolean(deploymentUrl),
    code: res.code,
    timedOut: Boolean(res.timedOut),
    deploymentUrl,
    stdout: res.stdout ?? '',
    stderr: res.stderr ?? '',
  };
}

// Orchestrate one publish: guard the bundle, place hls.js, deploy, resolve the
// shareable URL, and emit events. The terminal `published` event carries the
// playback URL — it's the durable SPEC §6 go-live signal drained by the next run.
export async function runDeploy({
  siteDir,
  projectName,
  id,
  branch = 'main',
  pageConfig = {},
  vendorPath,
  runWrangler,
  log = () => {},
}) {
  if (!projectName) return { ok: false, reason: 'no_project', message: 'A Pages --project name is required.' };

  // Guard: don't deploy a bundle that's missing the page we're publishing.
  if (id) {
    const idx = path.join(siteDir, id, 'index.html');
    if (!fs.existsSync(idx)) {
      const message = `No built page at ${idx}. Run build-page first.`;
      log('deploy_failed', { reason: 'page_missing', message });
      return { ok: false, reason: 'page_missing', message };
    }
  }

  const hls = ensureHlsJs({ siteDir, vendorPath });
  if (!hls.ok) {
    log('deploy_failed', { reason: hls.reason, message: hls.message });
    return { ok: false, ...hls };
  }
  if (hls.placed) log('hlsjs_placed', { path: hls.path });

  const fav = ensureFavicon({ siteDir });
  if (fav.placed) log('favicon_placed', {});

  const dep = await deployPages({ siteDir, projectName, branch, runWrangler, log });
  if (!dep.ok) {
    const reason = dep.timedOut ? 'wrangler_timeout' : 'wrangler_failed';
    log('deploy_failed', { reason, code: dep.code, stderr: tail(dep.stderr) });
    return { ok: false, reason, code: dep.code, timedOut: dep.timedOut, stderr: dep.stderr };
  }
  log('deployed', { deploymentUrl: dep.deploymentUrl, projectName, branch, siteDir });

  const base = stripSlash(pageConfig.pagesBaseUrl) || productionBaseFromDeployment(dep.deploymentUrl);
  const playbackUrl = id && base ? `${base}/${id}/` : null;
  if (id) log('published', { id, playbackUrl, deploymentUrl: dep.deploymentUrl });

  return { ok: true, deploymentUrl: dep.deploymentUrl, playbackUrl, hlsjsPlaced: hls.placed };
}

// Keep error events small — surface the tail of wrangler's stderr, not all of it.
function tail(s, n = 600) {
  const str = String(s ?? '').trim();
  return str.length > n ? str.slice(-n) : str;
}

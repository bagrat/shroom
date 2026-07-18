// Public-URL config for the player page (SPEC §6/§9). These are *not* secrets —
// they're the public bases that get baked into each page:
//   publicBaseUrl : where the HLS bytes live (the bucket's public origin, e.g.
//                   an `*.r2.dev` URL). HLS playlist = `<publicBaseUrl>/<id>/stream.m3u8`.
//   pagesBaseUrl  : where the player pages are served (the Pages site, e.g.
//                   `*.pages.dev`). Page URL = `<pagesBaseUrl>/<id>/`.
//   hlsJsUrl      : path to the bundled hls.js on the Pages site (shared, one copy).
//
// They live alongside the S3 creds in ~/.shroom/credentials.json (set at setup,
// M5) but are loaded separately so the secret-bearing storage config stays focused.
// Env vars override the file; flags (build-page CLI) override env.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export const DEFAULT_CREDS_PATH = path.join(os.homedir(), '.shroom', 'credentials.json');

const stripSlash = (s) => (typeof s === 'string' ? s.replace(/\/+$/, '') : s);

export function loadPageConfig({ credsPath = DEFAULT_CREDS_PATH, env = process.env, overrides = {} } = {}) {
  let fromFile = {};
  if (fs.existsSync(credsPath)) {
    try {
      fromFile = JSON.parse(fs.readFileSync(credsPath, 'utf8'));
    } catch (e) {
      throw new Error(`Could not parse ${credsPath}: ${e.message}`);
    }
  }
  const fromEnv = {
    publicBaseUrl: env.SHROOM_PUBLIC_BASE_URL,
    pagesBaseUrl: env.SHROOM_PAGES_BASE_URL,
    hlsJsUrl: env.SHROOM_HLS_JS_URL,
  };
  const merged = {
    hlsJsUrl: '/hls.min.js', // shared, site-root by default
    ...pick(fromFile, ['publicBaseUrl', 'pagesBaseUrl', 'hlsJsUrl']),
    ...defined(fromEnv),
    ...defined(overrides),
  };
  return {
    publicBaseUrl: stripSlash(merged.publicBaseUrl),
    pagesBaseUrl: stripSlash(merged.pagesBaseUrl),
    hlsJsUrl: merged.hlsJsUrl,
  };
}

// Resolve the public links for one video id from a page config. `mp4` is the
// record's downloadable-MP4 field: a `<slug>.mp4` filename (slug-in-key — the object
// is stored under that name so a cross-origin download is named from the URL path),
// or `true` for legacy records (→ `video.mp4`). Falsy → no download URL.
export function urlsFor(cfg, id, mp4) {
  const mp4File = typeof mp4 === 'string' ? mp4 : (mp4 ? 'video.mp4' : null);
  return {
    hlsUrl: cfg.publicBaseUrl ? `${cfg.publicBaseUrl}/${id}/stream.m3u8` : `./stream.m3u8`,
    pageUrl: cfg.pagesBaseUrl ? `${cfg.pagesBaseUrl}/${id}/` : '',
    posterUrl: cfg.pagesBaseUrl ? `${cfg.pagesBaseUrl}/${id}/poster.jpg` : './poster.jpg',
    // The optional downloadable MP4 (uploaded by archive-local / the cleanup skill).
    // Rendered only when the record carries an `mp4` field — see render.mjs.
    downloadUrl: (cfg.publicBaseUrl && mp4File) ? `${cfg.publicBaseUrl}/${id}/${mp4File}` : '',
    hlsJsUrl: cfg.hlsJsUrl ?? '/hls.min.js',
  };
}

export function missingPageFields(cfg) {
  return ['publicBaseUrl', 'pagesBaseUrl'].filter((k) => !cfg?.[k]);
}

function pick(obj, keys) {
  return Object.fromEntries(keys.filter((k) => k in (obj ?? {})).map((k) => [k, obj[k]]));
}
function defined(obj) {
  return Object.fromEntries(Object.entries(obj).filter(([, v]) => v != null));
}

// setup status — the pure computation behind `setup.mjs status` and the record
// preflight's "ready" gate. Extracted here so BOTH the setup CLI (which prints it)
// and the one-shot record preflight (which reads `ready`) share one definition of
// "configured / ready" instead of drifting.
//
// `computeStatus` reads ~/.shroom/credentials.json and reports what's present:
// library, storage, pages — and whether the whole thing is ready to record. It's
// offline by default; `--verify` adds a cheap signed R2 HEAD to confirm the stored
// keys still work. Fail-soft: a null verify (transient/unreachable) never blocks.

import fs from 'node:fs';
import { readCreds, credsPath } from './credentials.mjs';
import { headObject } from '../../uploader/lib/s3.mjs';

// Cheap liveness check for the stored R2 keys: a signed HEAD on a key that won't
// exist. 200/404 → signature accepted (keys + bucket good); 401/403 → bad/expired
// keys (re-ask); anything else or a network error → null (don't cry failure on a
// transient — presence still counts).
export async function verifyR2(creds) {
  if (typeof fetch !== 'function') return { ok: null, reason: 'no_fetch' };  // Node <18
  const client = {
    endpoint: creds.endpoint, region: creds.region || 'auto', bucket: creds.bucket,
    accessKeyId: creds.accessKeyId, secretAccessKey: creds.secretAccessKey,
  };
  try {
    const { status } = await headObject(client, '.shroom-setup-probe');
    if (status === 200 || status === 404) return { ok: true };
    if (status === 401 || status === 403) return { ok: false, reason: 'invalid_keys' };
    return { ok: null, reason: `http_${status}` };
  } catch {
    return { ok: null, reason: 'unreachable' };
  }
}

// Compute the setup status object. `verify` (default false) live-checks the R2 keys.
// Returns { ok, ready, library, storage, pages, credentials }.
export async function computeStatus({ verify = false } = {}) {
  const creds = readCreds(credsPath());

  const libPath = creds.library;
  const library = { configured: Boolean(libPath) && fs.existsSync(libPath), path: libPath ?? null };

  const storageConfigured = Boolean(
    creds.accountId && creds.bucket && creds.endpoint &&
    creds.accessKeyId && creds.secretAccessKey && creds.publicBaseUrl,
  );
  const storage = {
    configured: storageConfigured,
    accountId: creds.accountId ?? null,
    bucket: creds.bucket ?? null,
    publicBaseUrl: creds.publicBaseUrl ?? null,
    verified: null,        // null = not checked, or couldn't tell (transient)
    verifyReason: null,
  };
  if (storageConfigured && verify) {
    const v = await verifyR2(creds);
    storage.verified = v.ok;
    storage.verifyReason = v.reason ?? null;
  }

  const pages = {
    configured: Boolean(creds.pagesProject && creds.pagesBaseUrl),
    project: creds.pagesProject ?? null,
    baseUrl: creds.pagesBaseUrl ?? null,
  };

  // ready = nothing left to ask the user for: library + storage + pages all present,
  // and (when we checked) the keys actually work. A null verify doesn't block.
  const ready =
    library.configured && storage.configured && pages.configured && storage.verified !== false;

  return { ok: true, ready, library, storage, pages, credentials: credsPath() };
}

// Write ~/.shroom/credentials.json (SPEC §9) — the single file the rest of shroom
// reads. It carries two kinds of fields, deliberately in one place so setup writes
// once and the uploader / page / deploy each load the slice they need:
//
//   secrets (S3, uploader):  endpoint, region, bucket, accessKeyId, secretAccessKey
//   public  (page/deploy):   publicBaseUrl, pagesBaseUrl, pagesProject, hlsJsUrl
//   ref     (provisioning):  accountId
//
// Secrets stay out of git (working agreement) — this file lives under ~/.shroom,
// mode 600, dir 700, never in the repo. Writes are MERGE, not clobber: a re-run of
// setup (idempotent by design, SPEC §8) preserves fields it isn't changing, and a
// partially-provisioned account can be topped up without losing what worked.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export function shroomDir({ home = os.homedir() } = {}) {
  return path.join(home, '.shroom');
}
export function credsPath({ home = os.homedir() } = {}) {
  return path.join(shroomDir({ home }), 'credentials.json');
}

// R2's S3 endpoint is derivable from the account id — no need to discover it.
export function r2Endpoint(accountId) {
  return accountId ? `https://${accountId}.r2.cloudflarestorage.com` : undefined;
}

export function readCreds(file) {
  if (!fs.existsSync(file)) return {};
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (e) {
    throw new Error(`Could not parse ${file}: ${e.message}`);
  }
}

// Merge `patch` into the existing creds and write atomically with locked-down
// perms. Only *defined, non-null* patch fields are applied, so passing a
// not-yet-known field as undefined never erases an existing value. Returns the
// merged object.
export function writeCreds(patch, { home = os.homedir() } = {}) {
  const dir = shroomDir({ home });
  const file = credsPath({ home });
  fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
  // mkdir's mode is masked by umask; force it (best-effort on platforms w/o chmod).
  try { fs.chmodSync(dir, 0o700); } catch { /* non-POSIX fs */ }

  const merged = { ...readCreds(file), ...defined(patch) };
  const tmp = `${file}.tmp-${process.pid}`;
  fs.writeFileSync(tmp, JSON.stringify(merged, null, 2) + '\n', { mode: 0o600 });
  try { fs.chmodSync(tmp, 0o600); } catch { /* non-POSIX fs */ }
  fs.renameSync(tmp, file); // atomic replace; preserves the tmp file's 600 mode
  return merged;
}

// Assemble the credential patch from provisioning outputs. Derives the S3 endpoint
// from accountId and defaults region to R2's "auto"; strips trailing slashes from
// the public bases (matching page-config's normalization). Undefined inputs are
// dropped so this composes with the merge-not-clobber write.
export function buildCredentials({
  accountId,
  bucket,
  accessKeyId,
  secretAccessKey,
  publicBaseUrl,
  pagesProject,
  pagesBaseUrl,
  library,
} = {}) {
  return defined({
    accountId,
    endpoint: r2Endpoint(accountId),
    region: accountId || bucket ? 'auto' : undefined,
    bucket,
    accessKeyId,
    secretAccessKey,
    publicBaseUrl: stripSlash(publicBaseUrl),
    pagesProject,
    pagesBaseUrl: stripSlash(pagesBaseUrl),
    library, // where <id>.md records live (the git library); read by write-meta/record
  });
}

const stripSlash = (s) => (typeof s === 'string' ? s.replace(/\/+$/, '') : s);
function defined(obj) {
  return Object.fromEntries(Object.entries(obj ?? {}).filter(([, v]) => v != null));
}

// Cloudflare provisioning — the deterministic mechanism behind the setup command's
// Cloudflare sub-sequence (SPEC §8). Every CF call goes through the injected
// `runWrangler` seam (the same spawn+tee seam deploy uses, scripts/deploy/lib/
// wrangler.mjs), so this orchestration is fully testable offline with a fake.
//
// The driving idea (SPEC §8) is "probe capability, not state": there's no clean
// "account verified / R2 enabled / has a card" flag, so we just *attempt the real
// gated op* and branch on the classified failure. That makes the whole flow
// idempotent and re-runnable — a verified, already-provisioned account sails
// through; a cold one fails with a specific state the command turns into a
// dashboard checklist, then re-runs.

import { classifyWranglerError, isCreateSuccess } from './wrangler-errors.mjs';

const ACCOUNT_ID_RE = /\b([0-9a-f]{32})\b/i;
// Stop the local-part/domain before trailing sentence punctuation ("…email foo@bar.com.").
const EMAIL_RE = /([\w.+-]+@[\w-]+\.[\w-]+(?:\.[\w-]+)*)/;
const R2_DEV_RE = /https?:\/\/[^\s'"<>]*\.r2\.dev[^\s'"<>]*/;

function text(res) {
  return `${res.stdout ?? ''}\n${res.stderr ?? ''}`;
}

// `wrangler whoami` → who we are (and whether we're logged in at all). The OAuth
// session is established by `wrangler login` (interactive, opens a browser — the
// command runs that, then re-probes here). Skippable for a returning user whose
// session is still valid (SPEC §8: "probe → skip or surface").
export async function whoami({ runWrangler }) {
  const res = await runWrangler(['whoami']);
  if (res.code !== 0) {
    const cls = classifyWranglerError(res);
    return { loggedIn: false, state: cls.state, message: cls.message };
  }
  const t = text(res);
  return {
    loggedIn: true,
    accountId: (t.match(ACCOUNT_ID_RE) || [])[1] ?? null,
    email: (t.match(EMAIL_RE) || [])[1] ?? null,
    raw: t.trim(),
  };
}

// Create the R2 bucket. `already_exists` counts as success — setup is idempotent,
// so a re-run over an existing bucket is fine. A cold account fails here with
// r2_not_enabled / needs_payment / email_unverified — exactly the gates the
// command surfaces as a dashboard checklist (SPEC §8 step 3).
export async function createBucket({ runWrangler, name }) {
  const res = await runWrangler(['r2', 'bucket', 'create', name]);
  if (isCreateSuccess(res)) return { ok: true, name };
  return { ok: false, name, ...classifyWranglerError(res) };
}

// Turn on the bucket's managed `*.r2.dev` public URL — the zero-DNS public origin
// for the HLS bytes (SPEC §8 custom-domain-deferred). Parse the URL it prints;
// that becomes `publicBaseUrl` in the creds.
export async function enablePublicAccess({ runWrangler, name }) {
  const res = await runWrangler(['r2', 'bucket', 'dev-url', 'enable', name]);
  if (res.code !== 0 && !isCreateSuccess(res)) {
    return { ok: false, name, ...classifyWranglerError(res) };
  }
  const url = (text(res).match(R2_DEV_RE) || [])[0] ?? null;
  return { ok: true, name, publicBaseUrl: url ? stripSlash(url) : null };
}

// Create the Pages project the player pages deploy to. Production branch defaults
// to "main" (matches our git default + deploy's default). `already_exists` = ok.
// The stable site base is `https://<name>.pages.dev` (a project name can't contain
// dots, so this is always well-formed).
export async function createPagesProject({ runWrangler, name, branch = 'main' }) {
  const res = await runWrangler(['pages', 'project', 'create', name, '--production-branch', branch]);
  if (isCreateSuccess(res)) return { ok: true, name, pagesBaseUrl: `https://${name}.pages.dev` };
  return { ok: false, name, ...classifyWranglerError(res) };
}

// Orchestrate the whole sub-sequence. Stops at the first hard failure and reports
// the stage + classified state so the command knows whether to re-login, surface a
// dashboard gate, or retry. Does NOT touch the filesystem — it returns the facts;
// the CLI assembles + writes the creds (keeps this offline-testable).
//
// `mintR2Token` is the one piece that can't be done through wrangler (it has no
// command to mint an S3-compatible R2 API token — access key id + secret). It's an
// injected seam: the real implementation (a Cloudflare API call) is wired in the
// live-account session. Absent, provisioning still completes and reports the token
// as a pending manual step rather than fabricating credentials.
export async function provisionCloudflare({
  runWrangler,
  mintR2Token,
  bucket = 'shroom',
  pagesProject = 'shroom-site',
  branch = 'main',
  log = () => {},
}) {
  const me = await whoami({ runWrangler });
  if (!me.loggedIn) {
    log('cf_login_required', { state: me.state });
    return { ok: false, stage: 'login', ...me };
  }
  log('cf_whoami', { accountId: me.accountId, email: me.email });

  const b = await createBucket({ runWrangler, name: bucket });
  if (!b.ok) {
    log('cf_bucket_failed', { state: b.state, needsDashboard: b.needsDashboard });
    return { ok: false, stage: 'bucket', accountId: me.accountId, ...b };
  }
  log('cf_bucket_ready', { bucket });

  const pub = await enablePublicAccess({ runWrangler, name: bucket });
  if (!pub.ok) {
    log('cf_public_failed', { state: pub.state });
    return { ok: false, stage: 'public_access', accountId: me.accountId, ...pub };
  }
  log('cf_public_ready', { publicBaseUrl: pub.publicBaseUrl });

  const pg = await createPagesProject({ runWrangler, name: pagesProject, branch });
  if (!pg.ok) {
    log('cf_pages_failed', { state: pg.state });
    return { ok: false, stage: 'pages', accountId: me.accountId, ...pg };
  }
  log('cf_pages_ready', { pagesProject, pagesBaseUrl: pg.pagesBaseUrl });

  let token = { deferred: true };
  if (typeof mintR2Token === 'function') {
    token = await mintR2Token({ accountId: me.accountId, bucket });
    log(token?.accessKeyId ? 'cf_token_ready' : 'cf_token_failed', {});
  } else {
    log('cf_token_deferred', {});
  }

  return {
    ok: true,
    accountId: me.accountId,
    email: me.email,
    bucket,
    publicBaseUrl: pub.publicBaseUrl,
    pagesProject,
    pagesBaseUrl: pg.pagesBaseUrl,
    token, // { accessKeyId, secretAccessKey } | { deferred: true }
  };
}

const stripSlash = (s) => (typeof s === 'string' ? s.replace(/\/+$/, '') : s);

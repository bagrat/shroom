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
export async function whoami({ runWrangler, env }) {
  const res = await runWrangler(['whoami'], { env });
  const t = text(res);
  // `wrangler whoami` exits 0 even when unauthenticated, printing "You are not
  // authenticated. Please run `wrangler login`." — so a clean exit code is NOT
  // proof of a session. Key on "not authenticated"/"not logged in" ONLY: a *logged-
  // in* narrow-scope session ALSO prints "...run `wrangler login` to refresh" (the
  // benign missing-scopes warning), so matching "wrangler login" would false-positive.
  const unauthed = /not authenticated|you are not logged in/i.test(t);
  if (res.code !== 0 || unauthed) {
    const cls = classifyWranglerError({ ...res, code: res.code || 1 });
    // Text that explicitly says "not authenticated" → not_logged_in; otherwise let
    // the classifier name it (e.g. a node-engine error must NOT read as logged-out).
    return { loggedIn: false, state: unauthed ? 'not_logged_in' : cls.state, message: cls.message };
  }
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
export async function createBucket({ runWrangler, name, env }) {
  const res = await runWrangler(['r2', 'bucket', 'create', name], { env });
  if (isCreateSuccess(res)) return { ok: true, name };
  return { ok: false, name, ...classifyWranglerError(res) };
}

// Turn on the bucket's managed `*.r2.dev` public URL — the zero-DNS public origin
// for the HLS bytes (SPEC §8 custom-domain-deferred). Parse the URL it prints;
// that becomes `publicBaseUrl` in the creds.
export async function enablePublicAccess({ runWrangler, name, env }) {
  // `--force` skips wrangler's interactive "make this public?" confirm — safe here
  // ONLY because the command has already taken the user's explicit consent for
  // public access (a security-weakening step it must never auto-confirm silently).
  const res = await runWrangler(['r2', 'bucket', 'dev-url', 'enable', name, '--force'], { env });
  if (res.code !== 0 && !isCreateSuccess(res)) {
    return { ok: false, name, ...classifyWranglerError(res) };
  }
  const url = (text(res).match(R2_DEV_RE) || [])[0] ?? null;
  return { ok: true, name, publicBaseUrl: url ? stripSlash(url) : null };
}

// Create the Pages project the player pages deploy to. Production branch defaults
// to "main" (matches our git default + deploy's default). `already_exists` = ok.
//
// We deliberately do NOT shell out to `wrangler pages project create`. In a
// non-interactive shell (Claude Code spawns wrangler non-interactively,
// isInteractive:false) wrangler's Pages code path HARD-REQUIRES a
// CLOUDFLARE_API_TOKEN and refuses the OAuth session — even with a fresh, in-scope
// token (VM-proven 2026-06-28: the same OAuth token succeeded for an R2 API call
// seconds earlier, then `pages project create` errored "necessary to set a
// CLOUDFLARE_API_TOKEN"). So Pages is created via the Cloudflare REST API directly
// with the OAuth token as a Bearer — it carries `pages:write`, so the API authorizes
// it; we only bypass wrangler's CLI interactivity guard. `createPages` is the
// injected HTTP seam (real impl reads the wrangler OAuth token; see setup.mjs), so
// this stays offline-testable. It returns a normalized result:
//   { ok:true, subdomain } | { ok:true, alreadyExists:true } |
//   { ok:false, state, message, needsDashboard? }
export async function createPagesProject({ createPages, name, branch = 'main', accountId }) {
  const res = await createPages({ accountId, name, branch });
  if (res.ok) {
    // The project subdomain is NOT `<name>.pages.dev` — Cloudflare appends a random
    // suffix when the name isn't globally unique (e.g. `shroom-site-eym.pages.dev`,
    // verified live). The create API returns the real subdomain; use it. An
    // already_exists re-run has none, so fall back to the constructed form and flag
    // it unparsed so the caller won't clobber a stored, suffixed URL.
    const sub = res.alreadyExists ? null : res.subdomain;
    const pagesBaseUrl = sub ? `https://${stripSlash(sub)}` : `https://${name}.pages.dev`;
    return { ok: true, name, pagesBaseUrl, parsed: Boolean(sub) };
  }
  return { ok: false, name, state: res.state, message: res.message, needsDashboard: res.needsDashboard ?? false };
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
  // Injected Pages-via-API seam (see createPagesProject for why Pages can't go
  // through wrangler non-interactively). Real impl is wired in setup.mjs; absent,
  // we report a non-fatal "unwired" state rather than throwing.
  createPages = async () => ({ ok: false, state: 'pages_seam_unwired', message: 'Pages API seam not provided.' }),
  // The R2 API token (CF API token value) the user created in the dashboard. R2
  // cannot be managed over the wrangler OAuth session (no r2 scope exists — live-
  // verified), so R2 ops run with this token as CLOUDFLARE_API_TOKEN. Its derived
  // S3 keys (accessKeyId/secretAccessKey) are what the uploader needs and are
  // passed through to the creds by the caller. Absent → we stop with a dashboard
  // gate rather than attempting a call we know returns code 10000.
  r2Token,
  r2AccessKeyId,
  r2SecretAccessKey,
  bucket = 'shroom',
  pagesProject = 'shroom-site',
  branch = 'main',
  baseEnv = process.env,
  log = () => {},
}) {
  // R2 calls authenticate with the dashboard R2 token; whoami uses the OAuth
  // session, so we must strip any inherited CLOUDFLARE_API_TOKEN from its env (an R2
  // token has no account/user scope and would mis-answer whoami). `oauthEnv` also
  // carries the node>=22 PATH from baseEnv, so wrangler runs under a new-enough node.
  // (Pages no longer goes through wrangler at all — see createPagesProject.)
  const { CLOUDFLARE_API_TOKEN: _drop, ...oauthEnv } = baseEnv;

  const me = await whoami({ runWrangler, env: oauthEnv });
  if (!me.loggedIn) {
    log('cf_login_required', { state: me.state });
    return { ok: false, stage: 'login', ...me };
  }
  log('cf_whoami', { accountId: me.accountId, email: me.email });

  if (!r2Token) {
    log('cf_r2_token_required', {});
    return {
      ok: false, stage: 'bucket', accountId: me.accountId,
      ...STATE_R2_TOKEN_REQUIRED,
    };
  }
  const r2Env = { ...oauthEnv, CLOUDFLARE_API_TOKEN: r2Token };

  const b = await createBucket({ runWrangler, name: bucket, env: r2Env });
  if (!b.ok) {
    log('cf_bucket_failed', { state: b.state, needsDashboard: b.needsDashboard });
    return { ok: false, stage: 'bucket', accountId: me.accountId, ...b };
  }
  log('cf_bucket_ready', { bucket });

  const pub = await enablePublicAccess({ runWrangler, name: bucket, env: r2Env });
  if (!pub.ok) {
    log('cf_public_failed', { state: pub.state });
    return { ok: false, stage: 'public_access', accountId: me.accountId, ...pub };
  }
  log('cf_public_ready', { publicBaseUrl: pub.publicBaseUrl });

  const pg = await createPagesProject({ createPages, name: pagesProject, branch, accountId: me.accountId });
  if (!pg.ok) {
    log('cf_pages_failed', { state: pg.state });
    return { ok: false, stage: 'pages', accountId: me.accountId, ...pg };
  }
  log('cf_pages_ready', { pagesProject, pagesBaseUrl: pg.pagesBaseUrl });

  // The S3 keys come straight from the dashboard token (Access Key ID + Secret).
  const token = r2AccessKeyId && r2SecretAccessKey
    ? { accessKeyId: r2AccessKeyId, secretAccessKey: r2SecretAccessKey }
    : { deferred: true };
  log(token.accessKeyId ? 'cf_token_ready' : 'cf_token_deferred', {});

  return {
    ok: true,
    accountId: me.accountId,
    email: me.email,
    bucket,
    publicBaseUrl: pub.publicBaseUrl,
    pagesProject,
    pagesBaseUrl: pg.pagesBaseUrl,
    // false when pagesBaseUrl is only the constructed fallback (already_exists
    // re-run) — the caller should not overwrite a stored, parsed URL with a guess.
    pagesBaseUrlParsed: pg.parsed,
    token, // { accessKeyId, secretAccessKey } | { deferred: true }
  };
}

// Shape returned when no R2 token is available — routes the command to the
// dashboard token-creation gate (the one unavoidable manual step).
const STATE_R2_TOKEN_REQUIRED = {
  ok: false,
  state: 'r2_token_required',
  needsDashboard: true,
  action: 'create_r2_token',
  message: 'R2 cannot be provisioned over OAuth. Create an R2 API token in the dashboard (Object Read & Write) and re-run.',
};

const stripSlash = (s) => (typeof s === 'string' ? s.replace(/\/+$/, '') : s);

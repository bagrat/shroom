// The wrangler error-shape catalogue (SPEC §8 capability-probe, §11 build task).
//
// Setup's whole Cloudflare flow is "attempt the real gated op, branch on the
// result" (SPEC §8): we don't try to *detect* whether the account is verified /
// R2-enabled / has a card — there's no clean flag — we just run `r2 bucket create`
// (etc.) and read the failure. That makes the distinctions here load-bearing:
// "not logged in" vs "email unverified" vs "R2 not enabled" vs "needs a card" vs
// "insufficient scope" each route to a *different* next step (re-login, a
// dashboard gate, a retry), so they must be told apart.
//
// IMPORTANT — these patterns are best-effort until validated against real wrangler
// output. The first live-account setup session catalogues the actual strings/exit
// codes and tightens these matchers; the classifier is structured (ordered
// matchers, one state each) precisely so that's a localized edit. Until then treat
// `unknown` as "surface stderr to the user and stop", never as success.

// State → what the caller should do. `terminal` failures need a human/dashboard
// step; `retryable` failures are worth re-attempting after that step or a wait.
export const STATES = {
  ok: { ok: true },
  already_exists: { ok: true, idempotent: true },
  not_logged_in: { retryable: true, action: 'login' },
  insufficient_scope: { retryable: true, action: 'login', note: 'token lacks R2/Pages scope' },
  // R2 cannot be managed over the wrangler OAuth session — there is NO r2 OAuth
  // scope (verified live), so `r2 bucket create` returns "Authentication error
  // [code: 10000]" against /r2/buckets even when logged in with a verified,
  // R2-enabled account. The ONLY fix is a dashboard-minted R2 API token used as
  // CLOUDFLARE_API_TOKEN — re-login can never help, so this must not be mistaken
  // for not_logged_in. (Live-account session finding.)
  r2_token_required: { needsDashboard: true, action: 'create_r2_token', note: 'OAuth cannot manage R2; needs a dashboard R2 API token' },
  email_unverified: { retryable: true, needsDashboard: true, action: 'verify_email' },
  r2_not_enabled: { retryable: true, needsDashboard: true, action: 'enable_r2' },
  needs_payment: { retryable: true, needsDashboard: true, action: 'add_payment' },
  not_found: { action: 'create_first' },
  network: { retryable: true, action: 'retry' },
  unknown: { action: 'surface' },
};

// Ordered matchers — first hit wins, so put the specific ones before the generic.
// Each tests the combined lower-cased stdout+stderr (+ exit code).
const MATCHERS = [
  // R2-over-OAuth (code 10000 against an /r2/ endpoint). MUST come first: the
  // failure text often also contains "wrangler login" (the missing-scopes warning),
  // which would otherwise mis-fire not_logged_in and trigger a useless re-login loop.
  { state: 'r2_token_required', test: (t) => /\[code:\s*10000\]/.test(t) && /\/r2\/|r2\/buckets|r2 bucket/.test(t) },
  // Auth. wrangler tends to say "not authenticated" / "must be logged in" /
  // "run `wrangler login`" / "Authentication error [code: 10000]".
  { state: 'not_logged_in', test: (t) => /not authenticated|must be logged in|wrangler login|you are not logged in|no account id|authentication.*required/.test(t) },
  // Scope: authed but the OAuth grant/token lacks R2 or Pages permission.
  { state: 'insufficient_scope', test: (t) => /insufficient (permissions|scope)|not authorized to|lacks the required|missing the following permissions|forbidden|\[code:\s*10000\].*authoriz/.test(t) },

  // Account gates (these come back from the first gated op, not from login).
  { state: 'email_unverified', test: (t) => /verify your email|email.*not.*verified|unverified email|confirm your email/.test(t) },
  // R2 subscription / ToS not accepted yet. Each alternative is self-contained
  // (no redundant leading "r2.*") so "Please enable R2" matches on its own.
  { state: 'r2_not_enabled', test: (t) => /r2 (is )?not enabled|not enabled.*r2|sign up for r2|enable r2|subscribe to r2|r2.*(subscription|terms of service|not subscribed)/.test(t) },
  // Payment method required.
  { state: 'needs_payment', test: (t) => /payment method|add a card|billing.*(required|profile)|enter your (billing|payment)|requires? a (valid )?payment/.test(t) },

  // Idempotency: the resource is already there → treat as success.
  { state: 'already_exists', test: (t) => /already exists|already taken|name is not available|a bucket with this name|conflict.*exists/.test(t) },
  { state: 'not_found', test: (t) => /not found|does not exist|no such (bucket|project)|couldn'?t find/.test(t) },

  // Transient transport errors — worth a retry.
  { state: 'network', test: (t) => /enotfound|etimedout|econnreset|network error|fetch failed|getaddrinfo|socket hang up|503|502|temporarily unavailable/.test(t) },
];

// Classify a wrangler invocation result.
//   { code, stdout, stderr } → { state, ok, retryable, needsDashboard, action, message, raw }
// A 0 exit is `ok`. Otherwise the first matching pattern wins; nothing matches →
// `unknown` (surface the stderr tail, don't guess).
export function classifyWranglerError({ code = 0, stdout = '', stderr = '' } = {}) {
  const raw = `${stdout}\n${stderr}`.trim();
  if (code === 0) return { state: 'ok', ...STATES.ok, message: '', raw };

  const hay = raw.toLowerCase();
  const hit = MATCHERS.find((m) => m.test(hay));
  const state = hit ? hit.state : 'unknown';
  return {
    state,
    ok: Boolean(STATES[state].ok), // already_exists is a non-zero exit we treat as success
    retryable: Boolean(STATES[state].retryable),
    needsDashboard: Boolean(STATES[state].needsDashboard),
    action: STATES[state].action,
    note: STATES[state].note,
    message: tail(raw),
    raw,
  };
}

// `already_exists` is a *failure exit* that we want to treat as success for
// create-style ops (idempotent re-run). This helper folds that in.
export function isCreateSuccess(result) {
  return result.code === 0 || classifyWranglerError(result).state === 'already_exists';
}

function tail(s, n = 500) {
  const str = String(s ?? '').trim();
  return str.length > n ? '…' + str.slice(-n) : str;
}

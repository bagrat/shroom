#!/usr/bin/env node
// setup — the deterministic backend for `/shroom:setup` (SPEC §8).
//
// The judgment (what to ask, when to install, how to phrase the gates) lives in
// the setup *command*; this script is the exact, repeatable mechanism it calls.
// It never mutates the machine on its own — `probe` only reads, and prints the
// install plan as a *proposal* the command surfaces for one approval.
//
// Subcommands:
//   probe [--json]   Check the local env (git/ffmpeg/wrangler/whisper) and print
//                    a consolidated install plan for whatever's missing.
//   init-library     Create the library dir, git init it, record it, vendor
//        --dir P      hls.min.js, and compile the macOS shim — the whole local-setup
//                     bundle as one call, so the command never hand-assembles shell.
//   check-verified   Is the account email verified? (Write-probe: an unverified
//                     account returns API code 8000077.) A gate to run right after
//                     login, before the R2 page that throws "verification required".
//   provision [...]  Run the Cloudflare sub-sequence (whoami → bucket → public
//                    access → Pages project) over wrangler and merge the results
//                    into ~/.shroom/credentials.json.
//
// Both are mechanism only — the command (judgment) decides when to call them and
// how to present gates. Neither mutates the machine beyond the explicit op the
// command has already approved (probe never mutates; provision runs wrangler).

import path from 'node:path';
import fs from 'node:fs';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

import { probeEnv, spawnRun, findNodeBinDir } from './lib/env-probe.mjs';
import { buildInstallPlan } from './lib/install-plan.mjs';
import { provisionCloudflare } from './lib/cloudflare.mjs';
import { buildCredentials, writeCreds, credsPath, wranglerPathEnv } from './lib/credentials.mjs';
import { spawnWrangler } from '../deploy/lib/wrangler.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));

async function haveBrew(run = spawnRun) {
  const res = await run('brew', ['--version']);
  return res.code === 0;
}

async function cmdProbe({ json }) {
  const env = await probeEnv();
  const plan = buildInstallPlan(env.results, { haveBrew: await haveBrew() });
  const out = { ...env, plan };

  if (json) {
    process.stdout.write(JSON.stringify(out, null, 2) + '\n');
    return env.ready ? 0 : 1;
  }

  for (const r of env.results) {
    const mark = r.present ? '✓' : r.required ? '✗' : '○';
    const ver = r.version ? ` ${r.version}` : '';
    const note = r.present ? '' : r.required ? '  (required)' : '  (optional)';
    process.stdout.write(`  ${mark} ${r.name}${ver}${note}\n`);
  }
  if (plan.nothingToInstall) {
    process.stdout.write('\nAll tools present.\n');
  } else {
    process.stdout.write('\nProposed install:\n');
    for (const step of plan.steps) {
      process.stdout.write(`  # ${step.label}${step.tools.length ? ` — ${step.tools.join(', ')}` : ''}\n`);
      process.stdout.write(`  ${step.command}\n`);
    }
  }
  return env.ready ? 0 : 1;
}

async function cmdProvision({ json, opts }) {
  // The seam forwards a per-call env so R2 ops can carry CLOUDFLARE_API_TOKEN while
  // Pages/whoami use the OAuth session (cloudflare.mjs splits them).
  const runWrangler = (args, extra = {}) =>
    spawnWrangler(args, { bin: opts.wrangler ?? 'wrangler', tee: !json, ...extra });
  // ndjson events to stderr so --json keeps stdout clean for the result object.
  const log = (event, fields = {}) =>
    process.stderr.write(JSON.stringify({ ts: new Date().toISOString(), event, ...fields }) + '\n');

  // Persist a Node >=22 bin dir up front so the wrangler seam runs under it, and
  // run THIS provision's wrangler calls under it too (baseEnv with prefixed PATH).
  const nodeBinDir = findNodeBinDir();
  if (nodeBinDir) writeCreds(buildCredentials({ nodeBinDir }));
  const baseEnv = wranglerPathEnv(process.env);

  // R2 creds may come via a file: the command writes the dashboard token + keys there with
  // the Write tool so they never land on a shell command line (consent prompt / ps /
  // history). Read it here. Keep it across retries — a `needsDashboard` failure auto-polls
  // by re-running this command, which must re-read it — and delete it on success or a
  // terminal failure so plaintext secrets don't linger.
  const credsFile = opts['r2-creds-file'] && opts['r2-creds-file'] !== 'true'
    ? opts['r2-creds-file'].replace(/^~(?=\/)/, os.homedir())
    : null;
  let fileCreds = {};
  if (credsFile) {
    try {
      fileCreds = JSON.parse(fs.readFileSync(credsFile, 'utf8'));
      fs.chmodSync(credsFile, 0o600); // it holds secrets — lock it down while it lives
    } catch (e) {
      const out = { ok: false, stage: 'r2-creds-file', state: 'unreadable', message: e.message };
      process.stdout.write(JSON.stringify(out, null, 2) + '\n');
      return 1;
    }
  }
  const rmCredsFile = () => { if (credsFile) try { fs.unlinkSync(credsFile); } catch { /* already gone */ } };

  const res = await provisionCloudflare({
    runWrangler,
    // Pages is created via the CF REST API with the OAuth token (wrangler refuses
    // the OAuth session for Pages non-interactively — see pagesApiCreate).
    createPages: pagesApiCreate,
    baseEnv,
    // The R2 API token (+ its derived S3 keys) the user created in the dashboard — passed
    // via --r2-creds-file (preferred) or, for scripting, flags/env.
    r2Token: opts['r2-token'] ?? fileCreds.r2Token ?? process.env.CLOUDFLARE_R2_TOKEN,
    r2AccessKeyId: opts['r2-access-key-id'] ?? fileCreds.r2AccessKeyId ?? process.env.SHROOM_S3_ACCESS_KEY_ID,
    r2SecretAccessKey: opts['r2-secret-access-key'] ?? fileCreds.r2SecretAccessKey ?? process.env.SHROOM_S3_SECRET_ACCESS_KEY,
    bucket: opts.bucket ?? 'shroom',
    pagesProject: opts['pages-project'] ?? 'shroom-site',
    branch: opts.branch ?? 'main',
    log,
  });

  if (!res.ok) {
    // Keep the creds file only when the user will retry from a dashboard gate (auto-poll
    // re-runs this); any other failure is terminal here, so don't leave secrets on disk.
    if (!res.needsDashboard) rmCredsFile();
    const out = { ok: false, stage: res.stage, state: res.state, needsDashboard: res.needsDashboard ?? false, message: res.message };
    if (json) process.stdout.write(JSON.stringify(out, null, 2) + '\n');
    else process.stderr.write(`\nProvisioning stopped at "${res.stage}" (${res.state}).\n${res.message ?? ''}\n`);
    return 1;
  }

  // Merge everything we learned into the creds. The S3 API token is the deferred
  // live-session piece (no wrangler command mints one) — when absent the file is
  // written without S3 keys and we say so, rather than fabricating credentials.
  const token = res.token?.accessKeyId ? res.token : {};
  writeCreds(
    buildCredentials({
      accountId: res.accountId,
      bucket: res.bucket,
      publicBaseUrl: res.publicBaseUrl,
      pagesProject: res.pagesProject,
      // Only persist the Pages URL when it came from the create API response — a
      // bare fallback on an already_exists re-run must not clobber the stored URL
      // (which may carry Cloudflare's random subdomain suffix).
      pagesBaseUrl: res.pagesBaseUrlParsed ? res.pagesBaseUrl : undefined,
      accessKeyId: token.accessKeyId,
      secretAccessKey: token.secretAccessKey,
    }),
  );
  rmCredsFile(); // success — the secrets are now in the (mode-600) creds; consume the temp file

  const out = {
    ok: true,
    accountId: res.accountId,
    bucket: res.bucket,
    publicBaseUrl: res.publicBaseUrl,
    pagesProject: res.pagesProject,
    pagesBaseUrl: res.pagesBaseUrl,
    s3Token: res.token?.accessKeyId ? 'written' : 'deferred',
    credentials: credsPath(),
  };
  if (json) {
    process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  } else {
    process.stdout.write(`\nProvisioned. Wrote ${out.credentials}\n`);
    process.stdout.write(`  bucket:   ${out.bucket}  (${out.publicBaseUrl ?? 'no public URL'})\n`);
    process.stdout.write(`  pages:    ${out.pagesProject}  (${out.pagesBaseUrl})\n`);
    if (out.s3Token === 'deferred')
      process.stdout.write('  S3 token: DEFERRED — creating an R2 S3 API token needs the live-account session.\n');
  }
  return 0;
}

// Persist the chosen library dir (where <id>.md records live) so /shroom:record
// and write-meta resolve it without re-asking. Pure creds write, no machine
// mutation (the command does the git init, propose→confirm→run).
function cmdSetLibrary({ json, opts }) {
  const dir = opts.dir && opts.dir !== 'true' ? path.resolve(opts.dir) : null;
  if (!dir) {
    process.stderr.write('Usage: setup.mjs set-library --dir <path>\n');
    return 2;
  }
  // Capture a Node >=22 bin dir while we're here (set-library runs early in setup),
  // so the wrangler seam has it before the Cloudflare phase.
  writeCreds(buildCredentials({ library: dir, nodeBinDir: findNodeBinDir() ?? undefined }));
  const out = { ok: true, library: dir, credentials: credsPath() };
  if (json) process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  else process.stdout.write(`Library set to ${dir} (${out.credentials})\n`);
  return 0;
}

// One deterministic call for the whole local-setup bundle the command used to stitch
// together in an inline shell blob (which tripped "can't statically analyze" and read
// as scary): create the library dir, `git init` it, record it in the creds, vendor
// hls.min.js, and compile the macOS shim. Idempotent — every step is a no-op if already
// done, so re-running after `xcode-select --install` just finishes the shim. Returns a
// JSON the command branches on; `shim: "needs-xcode-clt"` is the one human gate (a GUI
// installer the command must propose), everything else is mechanism.
async function cmdInitLibrary({ json, opts, run = (c, a) => spawnRun(c, a, { timeoutMs: 180000 }) }) {
  const dir = opts.dir && opts.dir !== 'true' ? path.resolve(opts.dir) : null;
  if (!dir) {
    process.stderr.write('Usage: setup.mjs init-library --dir <path>\n');
    return 2;
  }
  const fail = (stage, message, detail) => {
    const out = { ok: false, stage, message, detail: detail || undefined };
    if (json) process.stdout.write(JSON.stringify(out, null, 2) + '\n');
    else process.stderr.write(`\ninit-library failed at "${stage}": ${message}\n${detail ?? ''}\n`);
    return 1;
  };

  // 1. library dir + git init (only if it isn't already a repo).
  fs.mkdirSync(dir, { recursive: true });
  const isRepo = (await run('git', ['-C', dir, 'rev-parse', '--is-inside-work-tree'])).code === 0;
  let gitInitialized = false;
  if (!isRepo) {
    const gi = await run('git', ['-C', dir, 'init', '-q']);
    if (gi.code !== 0) return fail('git', 'git init failed', gi.stderr);
    gitInitialized = true;
  }

  // 2. record the library dir (+ a Node >=22 bin dir for the wrangler seam later).
  writeCreds(buildCredentials({ library: dir, nodeBinDir: findNodeBinDir() ?? undefined }));

  // 3. vendor hls.min.js (pinned + SHA-256 verified inside the script; idempotent).
  const hls = await run(process.execPath, [path.join(HERE, '../page/vendor/fetch-hls.mjs')]);
  if (hls.code !== 0) return fail('hls', 'vendoring hls.min.js failed', `${hls.stdout}\n${hls.stderr}`);

  // 4. compile the macOS control shim (macOS only). swiftc-missing is the one gate the
  //    command handles (propose `xcode-select --install`), surfaced as a clean flag.
  let shim = 'skipped';
  if (process.platform === 'darwin') {
    const build = await run('/bin/sh', [path.join(HERE, '../shim/macos/build.sh')]);
    const out = `${build.stdout}\n${build.stderr}`;
    if (build.code === 0) shim = 'built';
    else if (/swiftc not found/i.test(out)) shim = 'needs-xcode-clt';
    else return fail('shim', 'shim build failed', out);
  }

  const result = { ok: true, library: dir, gitInitialized, hlsVendored: true, shim, credentials: credsPath() };
  if (json) process.stdout.write(JSON.stringify(result, null, 2) + '\n');
  else {
    process.stdout.write(`Library ${dir}${gitInitialized ? ' (git init)' : ''}\n`);
    process.stdout.write('  hls.min.js vendored\n');
    process.stdout.write(`  shim: ${shim}\n`);
  }
  return 0;
}

// wrangler stores its OAuth token in a per-OS config TOML. We read it to make one
// authenticated API call (the verification probe below) — the same token wrangler
// itself uses. Best-effort across the known locations; null if not found.
function readWranglerOAuthToken({ home = os.homedir(), fsmod = fs } = {}) {
  const candidates = [
    path.join(home, 'Library/Preferences/.wrangler/config/default.toml'), // macOS
    path.join(home, '.config/.wrangler/config/default.toml'), // XDG / Linux
    path.join(home, '.wrangler/config/default.toml'), // legacy
  ];
  for (const p of candidates) {
    try {
      const m = fsmod.readFileSync(p, 'utf8').match(/oauth_token\s*=\s*"([^"]+)"/);
      if (m) return m[1];
    } catch { /* not here — keep looking */ }
  }
  return null;
}

// Create a Pages project via the Cloudflare REST API, authenticating with the
// wrangler OAuth token (which carries `pages:write`). This is the real seam wired
// into provisionCloudflare's `createPages`. We go direct to the API rather than
// `wrangler pages project create` because that command hard-requires a
// CLOUDFLARE_API_TOKEN in a non-interactive shell and refuses the OAuth session even
// with a fresh in-scope token (VM-proven 2026-06-28). The same token `check-verified`
// already creates probe projects with. Returns a normalized result the orchestrator
// classifies: { ok:true, subdomain } | { ok:true, alreadyExists:true } |
// { ok:false, state, message, needsDashboard? }.
async function pagesApiCreate({ accountId, name, branch = 'main', token = readWranglerOAuthToken(), fetchImpl = fetch }) {
  if (!token) return { ok: false, state: 'not_logged_in', message: 'No wrangler OAuth token found — run `wrangler login`.' };
  const api = 'https://api.cloudflare.com/client/v4';
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };

  let account = accountId && accountId !== 'true' ? accountId : null;
  if (!account) {
    try {
      const r = await fetchImpl(`${api}/accounts`, { headers });
      const d = await r.json();
      account = d?.result?.[0]?.id ?? null;
    } catch { /* fall through to the no_account gate */ }
  }
  if (!account) return { ok: false, state: 'no_account', message: 'Could not resolve the Cloudflare account id.' };

  let res, data;
  try {
    res = await fetchImpl(`${api}/accounts/${account}/pages/projects`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ name, production_branch: branch }),
    });
    data = await res.json().catch(() => ({}));
  } catch (e) {
    return { ok: false, state: 'network', message: String(e?.message || e) };
  }
  if (res.ok && data.success) return { ok: true, subdomain: data?.result?.subdomain ?? null };

  const errs = data.errors ?? [];
  const blob = JSON.stringify(errs);
  // A duplicate project name is success for us — setup is idempotent (re-run over a
  // partial provision). Match the code and a text backstop in case the code shifts.
  if (errs.some((e) => e.code === 8000007) || /already exists|already taken|duplicate/i.test(blob)) {
    return { ok: true, alreadyExists: true };
  }
  // Unverified email blocks the write (code 8000077) — route to the dashboard gate.
  if (errs.some((e) => e.code === 8000077) || /must be(en)? verified|verify your email/i.test(blob)) {
    return { ok: false, state: 'email_unverified', needsDashboard: true, message: 'Your Cloudflare email must be verified before creating the video site.' };
  }
  return { ok: false, state: 'pages_create_failed', message: blob || `HTTP ${res.status}` };
}

// Is the account's email verified? Reads don't reveal it (validated live: GET /user has
// no flag), but an in-scope WRITE does — an unverified account returns API error code
// 8000077 "Your user email must been verified". So we attempt a throwaway Pages project
// create and branch on that, deleting it again if it actually succeeds (verified). This
// is the one place we *detect* rather than attempt-the-real-op, because the verification
// wall sits BEFORE the user can even reach the R2/token steps — so it must be a gate
// right after login, not a mid-provision surprise. Result: { verified: true|false|null }
// (null = couldn't determine; the command proceeds and lets provision catch it later).
async function cmdCheckVerified({ json, opts }) {
  const emit = (o) => {
    if (json) process.stdout.write(JSON.stringify(o, null, 2) + '\n');
    else process.stdout.write(`email verified: ${o.verified}\n`);
    return 0;
  };
  const token = readWranglerOAuthToken();
  if (!token) return emit({ verified: null, reason: 'no_oauth_token' });

  const api = 'https://api.cloudflare.com/client/v4';
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };

  let account = opts.account && opts.account !== 'true' ? opts.account : null;
  if (!account) {
    try {
      const r = await fetch(`${api}/accounts`, { headers });
      const d = await r.json();
      account = d?.result?.[0]?.id ?? null;
    } catch { /* fall through */ }
  }
  if (!account) return emit({ verified: null, reason: 'no_account' });

  const probe = 'shroom-verify-probe';
  try {
    const res = await fetch(`${api}/accounts/${account}/pages/projects`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ name: probe, production_branch: 'main' }),
    });
    const data = await res.json().catch(() => ({}));
    if (res.ok && data.success) {
      // Verified — the probe really created a project; clean it back up.
      await fetch(`${api}/accounts/${account}/pages/projects/${probe}`, { method: 'DELETE', headers }).catch(() => {});
      return emit({ verified: true, accountId: account });
    }
    const errs = data.errors ?? [];
    if (errs.some((e) => e.code === 8000077) || /must be(en)? verified|verify your email/i.test(JSON.stringify(errs))) {
      return emit({ verified: false, accountId: account });
    }
    return emit({ verified: null, reason: 'unexpected', errors: errs });
  } catch (e) {
    return emit({ verified: null, reason: 'network', message: String(e?.message || e) });
  }
}

const [sub, ...rest] = process.argv.slice(2);
const json = rest.includes('--json');
const opts = parseArgs(rest);

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : 'true';
  }
  return o;
}

let code = 0;
switch (sub) {
  case 'probe':
    code = await cmdProbe({ json });
    break;
  case 'provision':
    code = await cmdProvision({ json, opts });
    break;
  case 'set-library':
    code = cmdSetLibrary({ json, opts });
    break;
  case 'init-library':
    code = await cmdInitLibrary({ json, opts });
    break;
  case 'check-verified':
    code = await cmdCheckVerified({ json, opts });
    break;
  default:
    process.stderr.write('Usage: setup.mjs <probe|provision|set-library|init-library|check-verified> [--json] [--dir PATH] [--account ID] [--bucket N] [--pages-project N] [--branch N] [--wrangler BIN]\n');
    code = 2;
}
process.exit(code);

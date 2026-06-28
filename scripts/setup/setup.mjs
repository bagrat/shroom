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
//   provision [...]  Run the Cloudflare sub-sequence (whoami → bucket → public
//                    access → Pages project) over wrangler and merge the results
//                    into ~/.shroom/credentials.json.
//
// Both are mechanism only — the command (judgment) decides when to call them and
// how to present gates. Neither mutates the machine beyond the explicit op the
// command has already approved (probe never mutates; provision runs wrangler).

import path from 'node:path';
import fs from 'node:fs';
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

  const res = await provisionCloudflare({
    runWrangler,
    baseEnv,
    // The R2 API token (+ its derived S3 keys) the user created in the dashboard.
    r2Token: opts['r2-token'] ?? process.env.CLOUDFLARE_R2_TOKEN,
    r2AccessKeyId: opts['r2-access-key-id'] ?? process.env.SHROOM_S3_ACCESS_KEY_ID,
    r2SecretAccessKey: opts['r2-secret-access-key'] ?? process.env.SHROOM_S3_SECRET_ACCESS_KEY,
    bucket: opts.bucket ?? 'shroom',
    pagesProject: opts['pages-project'] ?? 'shroom-site',
    branch: opts.branch ?? 'main',
    log,
  });

  if (!res.ok) {
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
      // Only persist the Pages URL when it was actually parsed from wrangler — a
      // bare fallback on an already_exists re-run must not clobber the stored URL
      // (which may carry Cloudflare's random subdomain suffix).
      pagesBaseUrl: res.pagesBaseUrlParsed ? res.pagesBaseUrl : undefined,
      accessKeyId: token.accessKeyId,
      secretAccessKey: token.secretAccessKey,
    }),
  );

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
  default:
    process.stderr.write('Usage: setup.mjs <probe|provision|set-library|init-library> [--json] [--dir PATH] [--bucket N] [--pages-project N] [--branch N] [--wrangler BIN]\n');
    code = 2;
}
process.exit(code);

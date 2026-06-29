// Offline tests for the whole setup backend: env probe + install plan (fake run +
// PATH-lookup seams), the wrangler error catalogue, the Cloudflare provisioning
// orchestration (fake runWrangler), and the credentials writer (temp HOME). No
// real binary, no real PATH, no network. Run: node scripts/setup/test/setup.test.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

import { probeTool, probeEnv, TOOLS } from '../lib/env-probe.mjs';
import { buildInstallPlan } from '../lib/install-plan.mjs';
import { detectNode } from '../lib/node-detect.mjs';
import { classifyWranglerError, isCreateSuccess } from '../lib/wrangler-errors.mjs';
import {
  whoami,
  createBucket,
  enablePublicAccess,
  createPagesProject,
  provisionCloudflare,
} from '../lib/cloudflare.mjs';
import { buildCredentials, writeCreds, readCreds, credsPath, r2Endpoint } from '../lib/credentials.mjs';

let passed = 0;
function test(name, fn) {
  return fn()
    .then(() => { passed++; })
    .catch((e) => { console.error(`✗ ${name}\n  ${e.stack || e}`); process.exitCode = 1; });
}

// A fake runner driven by a map of cmd -> {code, stdout, stderr}. Anything not in
// the map is "not on PATH" (code 127), like a real ENOENT.
function fakeRun(table) {
  return async (cmd) => table[cmd] ?? { code: 127, stdout: '', stderr: `${cmd}: command not found` };
}

// A fake PATH lookup: present iff the cmd is a key in the same table (presence-only
// tools like whisper are detected this way, never executed).
function fakeLookup(table) {
  return (cmd) => (cmd in table ? `/fake/bin/${cmd}` : null);
}

// Probe env against a single table that drives BOTH seams.
function probeWith(table) {
  return probeEnv({ run: fakeRun(table), lookupPath: fakeLookup(table) });
}

const REAL_OUTPUT = {
  node: { code: 0, stdout: 'v22.23.1\n', stderr: '' },
  git: { code: 0, stdout: 'git version 2.39.3 (Apple Git-146)\n', stderr: '' },
  ffmpeg: { code: 0, stdout: '', stderr: 'ffmpeg version 7.1.1 Copyright (c) 2000-2025\n' },
  wrangler: { code: 0, stdout: ' ⛅️ wrangler 3.90.0\n', stderr: '' },
  whisper: { code: 0, stdout: 'usage: whisper [-h] ...\n', stderr: '' },
  brew: { code: 0, stdout: 'Homebrew 4.2.0\n', stderr: '' },
};
// Present-node baseline for tables that only care about other tools.
const NODE_OK = { node: REAL_OUTPUT.node };

await test('probeTool: present tool reports version', async () => {
  const git = TOOLS.find((t) => t.name === 'git');
  const r = await probeTool(git, { run: fakeRun(REAL_OUTPUT) });
  assert.equal(r.present, true);
  assert.equal(r.version, '2.39.3');
});

await test('probeTool: version parsed from stderr (ffmpeg)', async () => {
  const ff = TOOLS.find((t) => t.name === 'ffmpeg');
  const r = await probeTool(ff, { run: fakeRun(REAL_OUTPUT) });
  assert.equal(r.present, true);
  assert.equal(r.version, '7.1.1');
});

await test('probeTool: missing binary (ENOENT/127) is absent', async () => {
  const wr = TOOLS.find((t) => t.name === 'wrangler');
  const r = await probeTool(wr, { run: fakeRun({}) });
  assert.equal(r.present, false);
  assert.equal(r.version, null);
});

await test('probeTool: whisper present via PATH lookup, never executed', async () => {
  const w = TOOLS.find((t) => t.name === 'whisper');
  let ran = false;
  const run = async () => { ran = true; return { code: 0, stdout: '', stderr: '' }; };
  const r = await probeTool(w, { run, lookupPath: fakeLookup(REAL_OUTPUT) });
  assert.equal(r.present, true);
  assert.equal(r.version, null);
  assert.equal(ran, false, 'presence-only tool must not be executed');
});

await test('probeTool: whisper absent when not on PATH', async () => {
  const w = TOOLS.find((t) => t.name === 'whisper');
  const r = await probeTool(w, { run: fakeRun({}), lookupPath: fakeLookup({}) });
  assert.equal(r.present, false);
});

await test('probeEnv: all present → ready, nothing missing', async () => {
  const env = await probeWith(REAL_OUTPUT);
  assert.equal(env.ready, true);
  assert.deepEqual(env.missingRequired, []);
  assert.deepEqual(env.missingOptional, []);
});

await test('probeEnv: missing required → not ready', async () => {
  const env = await probeWith({ ...NODE_OK, git: REAL_OUTPUT.git, whisper: REAL_OUTPUT.whisper });
  assert.equal(env.ready, false);
  assert.deepEqual(env.missingRequired.sort(), ['ffmpeg', 'wrangler']);
  assert.deepEqual(env.missingOptional, []); // whisper present
});

await test('probeEnv: whisper now required → missing whisper is not ready', async () => {
  const { whisper, brew, ...noWhisper } = REAL_OUTPUT;
  const env = await probeWith(noWhisper);
  assert.equal(env.ready, false);
  assert.deepEqual(env.missingRequired, ['whisper']);
  assert.deepEqual(env.missingOptional, []);
});

await test('probeTool: node below minMajor reads as not present', async () => {
  const node = TOOLS.find((t) => t.name === 'node');
  const r = await probeTool(node, { run: fakeRun({ node: { code: 0, stdout: 'v20.5.0\n' } }) });
  assert.equal(r.present, false);
  assert.equal(r.version, '20.5.0');
  assert.equal(r.reason, 'below_min_v22');
});

await test('probeTool: wrangler on old node (engine error) is not present', async () => {
  const wr = TOOLS.find((t) => t.name === 'wrangler');
  const r = await probeTool(wr, { run: fakeRun({ wrangler: { code: 1, stderr: 'Wrangler requires at least Node.js v22.0.0. You are using v20.5.0.' } }) });
  assert.equal(r.present, false, 'a version number scraped from the node-engine error must not count as present');
  assert.equal(r.reason, 'unhealthy');
});

await test('installPlan: nothing missing → empty plan', async () => {
  const env = await probeWith(REAL_OUTPUT);
  const plan = buildInstallPlan(env.results, { haveBrew: true });
  assert.equal(plan.nothingToInstall, true);
  assert.deepEqual(plan.steps, []);
  assert.equal(plan.combinedCommand, '');
});

await test('installPlan: groups by manager, batches packages', async () => {
  // ffmpeg+git(brew) and wrangler(npm) missing.
  const env = await probeWith({ ...NODE_OK, whisper: REAL_OUTPUT.whisper });
  const plan = buildInstallPlan(env.results, { haveBrew: true });
  const brew = plan.steps.find((s) => s.manager === 'brew');
  const npm = plan.steps.find((s) => s.manager === 'npm');
  assert.equal(brew.command, 'brew install git ffmpeg');
  assert.equal(npm.command, 'npm install -g wrangler');
  assert.deepEqual(plan.requiredMissing.sort(), ['ffmpeg', 'git', 'wrangler']);
  assert.ok(plan.combinedCommand.includes(' && '));
});

await test('installPlan: whisper (now required) appears in requiredMissing', async () => {
  const env = await probeWith({ ...NODE_OK, git: REAL_OUTPUT.git, ffmpeg: REAL_OUTPUT.ffmpeg, wrangler: REAL_OUTPUT.wrangler });
  const plan = buildInstallPlan(env.results, { haveBrew: true });
  assert.deepEqual(plan.requiredMissing, ['whisper']);
  assert.deepEqual(plan.optionalMissing, []);
  assert.equal(plan.steps.find((s) => s.manager === 'brew').command, 'brew install openai-whisper');
});

await test('installPlan: brew absent → bootstrap step prepended', async () => {
  const env = await probeWith({ wrangler: REAL_OUTPUT.wrangler });
  const plan = buildInstallPlan(env.results, { haveBrew: false });
  assert.equal(plan.needsBrew, true);
  assert.equal(plan.steps[0].manager, 'brew-bootstrap');
  assert.ok(plan.steps[0].command.includes('Homebrew/install'));
});

await test('installPlan: brew present → no bootstrap step', async () => {
  const env = await probeWith({ wrangler: REAL_OUTPUT.wrangler });
  const plan = buildInstallPlan(env.results, { haveBrew: true });
  assert.equal(plan.needsBrew, false);
  assert.ok(!plan.steps.some((s) => s.manager === 'brew-bootstrap'));
});

// ─── node detection (upgrade-command builder) ────────────────────────────────

// A fake fs with just the bits detectNode touches: existsSync (for nvm.sh) and
// realpathSync (symlink resolution). `links` maps a path → its realpath; `files`
// is the set of paths that "exist".
function fakeFs({ files = [], links = {} } = {}) {
  const set = new Set(files);
  return {
    existsSync: (p) => set.has(p),
    realpathSync: (p) => links[p] ?? p,
  };
}
// The node entry env-probe would produce for each state.
const NODE_OLD = { name: 'node', present: false, version: '20.5.0', reason: 'below_min_v22' };
const NODE_GONE = { name: 'node', present: false, version: null };
const NODE_NEW = { name: 'node', present: true, version: '22.9.0' };

await test('detectNode: brew node too old → brew upgrade command', async () => {
  const d = detectNode(NODE_OLD, {
    lookupPath: (c) => (c === 'node' ? '/opt/homebrew/bin/node' : null),
    env: {}, home: '/home/u', fsmod: fakeFs(), haveBrew: true,
  });
  assert.equal(d.belowMin, true);
  assert.equal(d.absent, false);
  assert.equal(d.source, 'brew');
  assert.equal(d.recommendedManager, 'brew');
  assert.equal(d.recommendedCommand, 'brew install node@22 && brew link --overwrite --force node@22');
});

await test('detectNode: nvm node too old → sourced nvm install command', async () => {
  const d = detectNode(NODE_OLD, {
    lookupPath: (c) => (c === 'node' ? '/home/u/.nvm/versions/node/v20.5.0/bin/node' : null),
    env: {}, home: '/home/u', fsmod: fakeFs({ files: ['/home/u/.nvm/nvm.sh'] }), haveBrew: false,
  });
  assert.equal(d.source, 'nvm');
  assert.equal(d.nvmAvailable, true);
  assert.equal(d.recommendedManager, 'nvm');
  assert.ok(d.recommendedCommand.includes('. "$NVM_DIR/nvm.sh"'));
  assert.ok(d.recommendedCommand.includes('nvm install 22'));
  assert.ok(d.recommendedCommand.includes('nvm alias default 22'));
});

await test('detectNode: brew symlink resolved via realpath (Intel /usr/local/bin)', async () => {
  const d = detectNode(NODE_OLD, {
    lookupPath: (c) => (c === 'node' ? '/usr/local/bin/node' : null),
    env: {}, home: '/home/u',
    fsmod: fakeFs({ links: { '/usr/local/bin/node': '/usr/local/Cellar/node/20.5.0/bin/node' } }),
    haveBrew: true,
  });
  assert.equal(d.source, 'brew');
  assert.equal(d.recommendedManager, 'brew');
});

await test('detectNode: system node, nvm present → prefer nvm over a system clash', async () => {
  // A real /usr/local/bin (not brew) node, but nvm is installed — recommend nvm so
  // we don't try to overwrite a system binary.
  const d = detectNode(NODE_OLD, {
    lookupPath: (c) => (c === 'node' ? '/usr/local/bin/node' : null),
    env: { NVM_DIR: '/home/u/.nvm' }, home: '/home/u',
    fsmod: fakeFs({ files: ['/home/u/.nvm/nvm.sh'] }), haveBrew: false,
  });
  assert.equal(d.source, 'system');
  assert.equal(d.nvmAvailable, true);
  assert.equal(d.recommendedManager, 'nvm');
});

await test('detectNode: neither nvm nor brew → bootstrap nvm', async () => {
  const d = detectNode(NODE_GONE, {
    lookupPath: () => null, env: {}, home: '/home/u', fsmod: fakeFs(), haveBrew: false,
  });
  assert.equal(d.absent, true);
  assert.equal(d.belowMin, false);
  assert.equal(d.nvmAvailable, false);
  assert.equal(d.brewAvailable, false);
  assert.equal(d.recommendedManager, 'nvm-bootstrap');
  assert.ok(d.recommendedCommand.includes('nvm-sh/nvm'));
  assert.ok(d.recommendedCommand.includes('nvm install 22'));
});

await test('detectNode: node already new enough → nothing to recommend', async () => {
  const d = detectNode(NODE_NEW, {
    lookupPath: (c) => (c === 'node' ? '/opt/homebrew/bin/node' : null),
    env: {}, home: '/home/u', fsmod: fakeFs(), haveBrew: true,
  });
  assert.equal(d.present, true);
  assert.equal(d.belowMin, false);
  assert.equal(d.recommendedCommand, null);
  assert.equal(d.recommendedManager, null);
});

// ─── wrangler error catalogue ────────────────────────────────────────────────

const ERR = (stderr, code = 1) => classifyWranglerError({ code, stderr });

await test('classify: exit 0 is ok', async () => {
  const c = classifyWranglerError({ code: 0, stdout: 'done' });
  assert.equal(c.state, 'ok');
  assert.equal(c.ok, true);
});

await test('classify: not logged in', async () => {
  const c = ERR('✘ You are not logged in. Run `wrangler login`.');
  assert.equal(c.state, 'not_logged_in');
  assert.equal(c.action, 'login');
  assert.equal(c.retryable, true);
});

await test('classify: R2 code 10000 → r2_token_required, not not_logged_in', async () => {
  // The real failure text also contains "wrangler login" (the missing-scopes
  // warning) — the r2 matcher must still win over not_logged_in.
  const c = ERR('A request to the Cloudflare API (/accounts/abc/r2/buckets) failed.\n  Authentication error [code: 10000]\n  run `wrangler login` to refresh');
  assert.equal(c.state, 'r2_token_required');
  assert.equal(c.needsDashboard, true);
  assert.equal(c.action, 'create_r2_token');
});

await test('classify: email unverified needs dashboard', async () => {
  const c = ERR('Error: Please verify your email address before continuing.');
  assert.equal(c.state, 'email_unverified');
  assert.equal(c.needsDashboard, true);
});

await test('classify: R2 not enabled needs dashboard', async () => {
  const c = ERR('You need to sign up for R2 and accept the terms of service.');
  assert.equal(c.state, 'r2_not_enabled');
  assert.equal(c.needsDashboard, true);
});

await test('classify: payment method required', async () => {
  const c = ERR('A valid payment method is required to enable this feature.');
  assert.equal(c.state, 'needs_payment');
  assert.equal(c.needsDashboard, true);
});

await test('classify: already exists is idempotent success', async () => {
  const c = ERR('A bucket with this name already exists.');
  assert.equal(c.state, 'already_exists');
  assert.equal(c.ok, true);
});

await test('classify: network error is retryable', async () => {
  const c = ERR('request to https://api.cloudflare.com failed, reason: getaddrinfo ENOTFOUND');
  assert.equal(c.state, 'network');
  assert.equal(c.retryable, true);
});

await test('classify: unrecognized → unknown/surface', async () => {
  const c = ERR('💥 some brand-new error wrangler invented this week');
  assert.equal(c.state, 'unknown');
  assert.equal(c.action, 'surface');
  assert.ok(c.message.length > 0);
});

await test('isCreateSuccess: 0, already_exists ok; other not', async () => {
  assert.equal(isCreateSuccess({ code: 0 }), true);
  assert.equal(isCreateSuccess({ code: 1, stderr: 'bucket already exists' }), true);
  assert.equal(isCreateSuccess({ code: 1, stderr: 'r2 is not enabled' }), false);
});

// ─── credentials writer (temp HOME) ──────────────────────────────────────────

function tmpHome() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-home-'));
}

await test('buildCredentials: derives endpoint + region, strips slashes, drops undefined', async () => {
  const c = buildCredentials({
    accountId: 'abc123',
    bucket: 'shroom',
    publicBaseUrl: 'https://x.r2.dev/',
    pagesBaseUrl: 'https://shroom-site.pages.dev//',
  });
  assert.equal(c.endpoint, r2Endpoint('abc123'));
  assert.equal(c.region, 'auto');
  assert.equal(c.publicBaseUrl, 'https://x.r2.dev');
  assert.equal(c.pagesBaseUrl, 'https://shroom-site.pages.dev');
  assert.ok(!('accessKeyId' in c)); // undefined dropped
});

await test('writeCreds: creates file 600 and merges without clobbering', async () => {
  const home = tmpHome();
  writeCreds({ accountId: 'acc', bucket: 'shroom', publicBaseUrl: 'https://x.r2.dev' }, { home });
  // A second write (e.g. token arrives later) must preserve earlier fields.
  const merged = writeCreds({ accessKeyId: 'AK', secretAccessKey: 'SK' }, { home });
  assert.equal(merged.bucket, 'shroom');
  assert.equal(merged.accessKeyId, 'AK');
  assert.equal(merged.publicBaseUrl, 'https://x.r2.dev');

  const onDisk = readCreds(credsPath({ home }));
  assert.deepEqual(onDisk, merged);
  if (process.platform !== 'win32') {
    assert.equal(fs.statSync(credsPath({ home })).mode & 0o777, 0o600);
    assert.equal(fs.statSync(path.dirname(credsPath({ home }))).mode & 0o777, 0o700);
  }
});

await test('writeCreds: undefined patch fields never erase existing values', async () => {
  const home = tmpHome();
  writeCreds({ bucket: 'shroom', accessKeyId: 'AK' }, { home });
  const merged = writeCreds(buildCredentials({ bucket: 'shroom' }), { home }); // no token fields
  assert.equal(merged.accessKeyId, 'AK');
});

await test('set-library: persists library to creds without clobbering, --json', async () => {
  const home = tmpHome();
  writeCreds({ bucket: 'shroom', accessKeyId: 'AK' }, { home });
  const setupCli = path.resolve(path.dirname(new URL(import.meta.url).pathname), '../setup.mjs');
  const out = execFileSync('node', [setupCli, 'set-library', '--dir', '/tmp/my-lib', '--json'],
    { env: { ...process.env, HOME: home }, encoding: 'utf8' });
  const res = JSON.parse(out.trim());
  assert.equal(res.ok, true);
  assert.equal(res.library, '/tmp/my-lib');
  const onDisk = readCreds(credsPath({ home }));
  assert.equal(onDisk.library, '/tmp/my-lib');
  assert.equal(onDisk.accessKeyId, 'AK'); // merge, not clobber
});

// ─── Cloudflare provisioning (fake runWrangler) ──────────────────────────────

// Route wrangler calls by their args; records calls for ordering assertions.
function router(routes) {
  const calls = [];
  const fn = async (args) => {
    calls.push(args.join(' '));
    const hit = routes.find((r) => r.match(args));
    return hit ? hit.res : { code: 1, stdout: '', stderr: `unrouted: ${args.join(' ')}` };
  };
  fn.calls = calls;
  return fn;
}
const has = (sub) => (args) => args.join(' ').includes(sub);

const WHOAMI_OK = {
  code: 0,
  stdout: `👋 You are logged in with an OAuth Token, associated with the email dev@example.com.
┌ Account Name ┬ Account ID ┐
│ Dev's Account │ 0123456789abcdef0123456789abcdef │
└──────────────┴────────────┘`,
  stderr: '',
};

await test('whoami: parses account id + email', async () => {
  const run = router([{ match: has('whoami'), res: WHOAMI_OK }]);
  const me = await whoami({ runWrangler: run });
  assert.equal(me.loggedIn, true);
  assert.equal(me.accountId, '0123456789abcdef0123456789abcdef');
  assert.equal(me.email, 'dev@example.com');
});

await test('whoami: not logged in when whoami fails', async () => {
  const run = router([{ match: has('whoami'), res: { code: 1, stderr: 'not authenticated' } }]);
  const me = await whoami({ runWrangler: run });
  assert.equal(me.loggedIn, false);
  assert.equal(me.state, 'not_logged_in');
});

await test('whoami: narrow-scope login is NOT mistaken for logged out', async () => {
  // A logged-in narrow login prints the benign "missing scopes … run `wrangler
  // login` to refresh" warning — must still read as logged in.
  const withWarning = {
    code: 0,
    stdout: WHOAMI_OK.stdout + '\n▲ Wrangler is missing some expected OAuth scopes. To fix this, run `wrangler login` to refresh your token.',
  };
  const me = await whoami({ runWrangler: router([{ match: has('whoami'), res: withWarning }]) });
  assert.equal(me.loggedIn, true);
  assert.equal(me.accountId, '0123456789abcdef0123456789abcdef');
});

await test('createBucket: success and idempotent already-exists', async () => {
  const ok = await createBucket({ runWrangler: router([{ match: has('bucket create'), res: { code: 0, stdout: 'Created bucket shroom' } }]), name: 'shroom' });
  assert.equal(ok.ok, true);
  const exists = await createBucket({ runWrangler: router([{ match: has('bucket create'), res: { code: 1, stderr: 'bucket already exists' } }]), name: 'shroom' });
  assert.equal(exists.ok, true);
});

await test('createBucket: r2 not enabled surfaces dashboard gate', async () => {
  const r = await createBucket({ runWrangler: router([{ match: has('bucket create'), res: { code: 1, stderr: 'Please enable R2 in the dashboard.' } }]), name: 'shroom' });
  assert.equal(r.ok, false);
  assert.equal(r.state, 'r2_not_enabled');
  assert.equal(r.needsDashboard, true);
});

await test('enablePublicAccess: parses the r2.dev url', async () => {
  const r = await enablePublicAccess({
    runWrangler: router([{ match: has('dev-url enable'), res: { code: 0, stdout: 'Public access enabled at https://pub-abc.r2.dev' } }]),
    name: 'shroom',
  });
  assert.equal(r.ok, true);
  assert.equal(r.publicBaseUrl, 'https://pub-abc.r2.dev');
});

await test('createPagesProject: uses the API subdomain (with CF suffix)', async () => {
  const r = await createPagesProject({
    createPages: async () => ({ ok: true, subdomain: 'shroom-site-eym.pages.dev' }),
    name: 'shroom-site',
  });
  assert.equal(r.ok, true);
  assert.equal(r.pagesBaseUrl, 'https://shroom-site-eym.pages.dev'); // real suffix, not <name>.pages.dev
  assert.equal(r.parsed, true);
});

await test('createPagesProject: already-exists falls back to <name>.pages.dev, unparsed', async () => {
  const r = await createPagesProject({
    createPages: async () => ({ ok: true, alreadyExists: true }),
    name: 'shroom-site',
  });
  assert.equal(r.ok, true);
  assert.equal(r.pagesBaseUrl, 'https://shroom-site.pages.dev');
  assert.equal(r.parsed, false); // fallback guess — caller must not clobber a stored URL
});

await test('createPagesProject: API failure surfaces classified state', async () => {
  const r = await createPagesProject({
    createPages: async () => ({ ok: false, state: 'email_unverified', needsDashboard: true, message: 'verify your email' }),
    name: 'shroom-site',
  });
  assert.equal(r.ok, false);
  assert.equal(r.state, 'email_unverified');
  assert.equal(r.needsDashboard, true);
});

// Pages is created via the API seam, not wrangler — fakes inject createPages.
const pagesOk = async () => ({ ok: true, subdomain: 'shroom-site.pages.dev' });

await test('provisionCloudflare: happy path with R2 token, ordered calls', async () => {
  const run = router([
    { match: has('whoami'), res: WHOAMI_OK },
    { match: has('bucket create'), res: { code: 0, stdout: 'Created' } },
    { match: has('dev-url enable'), res: { code: 0, stdout: 'at https://pub-xyz.r2.dev' } },
  ]);
  const events = [];
  let pagesArgs = null;
  const createPages = async (a) => { pagesArgs = a; return { ok: true, subdomain: 'shroom-site.pages.dev' }; };
  // r2Token given but S3 keys omitted → token reported deferred.
  const r = await provisionCloudflare({ runWrangler: run, createPages, r2Token: 'cfat_x', log: (e) => events.push(e) });
  assert.equal(r.ok, true);
  assert.equal(r.accountId, '0123456789abcdef0123456789abcdef');
  assert.equal(r.publicBaseUrl, 'https://pub-xyz.r2.dev');
  assert.equal(r.pagesBaseUrl, 'https://shroom-site.pages.dev');
  assert.deepEqual(r.token, { deferred: true });
  assert.ok(events.includes('cf_token_deferred'));
  // Pages create gets the account id parsed from whoami; never the R2 token.
  assert.equal(pagesArgs.accountId, '0123456789abcdef0123456789abcdef');
  assert.ok(!('CLOUDFLARE_API_TOKEN' in pagesArgs) && !('r2Token' in pagesArgs), 'Pages create must not receive the R2 token');
  // whoami before bucket before public (pages runs after, via the API seam).
  assert.ok(run.calls[0].includes('whoami'));
  assert.ok(run.calls[1].includes('bucket create'));
  // dev-url enable runs with --force (consent already taken by the command).
  assert.ok(run.calls.some((c) => c.includes('dev-url enable') && c.includes('--force')));
});

await test('provisionCloudflare: no R2 token → stops at dashboard token gate', async () => {
  const run = router([{ match: has('whoami'), res: WHOAMI_OK }]);
  const r = await provisionCloudflare({ runWrangler: run });
  assert.equal(r.ok, false);
  assert.equal(r.stage, 'bucket');
  assert.equal(r.state, 'r2_token_required');
  assert.equal(r.needsDashboard, true);
  assert.ok(!run.calls.some((c) => c.includes('bucket create'))); // short-circuits before any R2 call
});

await test('provisionCloudflare: stops at bucket gate, reports stage + dashboard', async () => {
  const run = router([
    { match: has('whoami'), res: WHOAMI_OK },
    { match: has('bucket create'), res: { code: 1, stderr: 'sign up for R2 and accept the terms of service' } },
  ]);
  let pagesCalled = false;
  const createPages = async () => { pagesCalled = true; return pagesOk(); };
  const r = await provisionCloudflare({ runWrangler: run, createPages, r2Token: 'cfat_x' });
  assert.equal(r.ok, false);
  assert.equal(r.stage, 'bucket');
  assert.equal(r.state, 'r2_not_enabled');
  assert.equal(r.needsDashboard, true);
  assert.equal(pagesCalled, false); // never reached
});

await test('provisionCloudflare: Pages API failure surfaces as the pages stage', async () => {
  const run = router([
    { match: has('whoami'), res: WHOAMI_OK },
    { match: has('bucket create'), res: { code: 0 } },
    { match: has('dev-url enable'), res: { code: 0, stdout: 'https://pub-xyz.r2.dev' } },
  ]);
  const createPages = async () => ({ ok: false, state: 'pages_create_failed', message: 'boom' });
  const r = await provisionCloudflare({ runWrangler: run, createPages, r2Token: 'cfat_x' });
  assert.equal(r.ok, false);
  assert.equal(r.stage, 'pages');
  assert.equal(r.state, 'pages_create_failed');
});

await test('provisionCloudflare: writes S3 keys from the dashboard token', async () => {
  const run = router([
    { match: has('whoami'), res: WHOAMI_OK },
    { match: has('bucket create'), res: { code: 0 } },
    { match: has('dev-url enable'), res: { code: 0, stdout: 'https://pub-xyz.r2.dev' } },
  ]);
  const r = await provisionCloudflare({ runWrangler: run, createPages: pagesOk, r2Token: 'cfat_x', r2AccessKeyId: 'ak-shroom', r2SecretAccessKey: 'sk' });
  assert.equal(r.token.accessKeyId, 'ak-shroom');
  assert.equal(r.token.secretAccessKey, 'sk');
});

console.log(`\n${passed} passed`);

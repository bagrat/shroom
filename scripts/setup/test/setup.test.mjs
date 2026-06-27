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
  git: { code: 0, stdout: 'git version 2.39.3 (Apple Git-146)\n', stderr: '' },
  ffmpeg: { code: 0, stdout: '', stderr: 'ffmpeg version 7.1.1 Copyright (c) 2000-2025\n' },
  wrangler: { code: 0, stdout: ' ⛅️ wrangler 3.90.0\n', stderr: '' },
  whisper: { code: 0, stdout: 'usage: whisper [-h] ...\n', stderr: '' },
  brew: { code: 0, stdout: 'Homebrew 4.2.0\n', stderr: '' },
};

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
  const env = await probeWith({ git: REAL_OUTPUT.git, whisper: REAL_OUTPUT.whisper });
  assert.equal(env.ready, false);
  assert.deepEqual(env.missingRequired.sort(), ['ffmpeg', 'wrangler']);
  assert.deepEqual(env.missingOptional, []); // whisper present
});

await test('probeEnv: only optional missing → still ready', async () => {
  const { whisper, ...noWhisper } = REAL_OUTPUT;
  const env = await probeWith(noWhisper);
  assert.equal(env.ready, true);
  assert.deepEqual(env.missingOptional, ['whisper']);
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
  const env = await probeWith({ whisper: REAL_OUTPUT.whisper });
  const plan = buildInstallPlan(env.results, { haveBrew: true });
  const brew = plan.steps.find((s) => s.manager === 'brew');
  const npm = plan.steps.find((s) => s.manager === 'npm');
  assert.equal(brew.command, 'brew install git ffmpeg');
  assert.equal(npm.command, 'npm install -g wrangler');
  assert.deepEqual(plan.requiredMissing.sort(), ['ffmpeg', 'git', 'wrangler']);
  assert.ok(plan.combinedCommand.includes(' && '));
});

await test('installPlan: optional whisper appears in plan but not requiredMissing', async () => {
  const env = await probeWith({ git: REAL_OUTPUT.git, ffmpeg: REAL_OUTPUT.ffmpeg, wrangler: REAL_OUTPUT.wrangler });
  const plan = buildInstallPlan(env.results, { haveBrew: true });
  assert.deepEqual(plan.requiredMissing, []);
  assert.deepEqual(plan.optionalMissing, ['whisper']);
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

await test('createPagesProject: success yields pages base', async () => {
  const r = await createPagesProject({
    runWrangler: router([{ match: has('pages project create'), res: { code: 0, stdout: 'Created project shroom-site' } }]),
    name: 'shroom-site',
  });
  assert.equal(r.ok, true);
  assert.equal(r.pagesBaseUrl, 'https://shroom-site.pages.dev');
});

await test('provisionCloudflare: happy path, token deferred, ordered calls', async () => {
  const run = router([
    { match: has('whoami'), res: WHOAMI_OK },
    { match: has('bucket create'), res: { code: 0, stdout: 'Created' } },
    { match: has('dev-url enable'), res: { code: 0, stdout: 'at https://pub-xyz.r2.dev' } },
    { match: has('pages project create'), res: { code: 0, stdout: 'Created' } },
  ]);
  const events = [];
  const r = await provisionCloudflare({ runWrangler: run, log: (e) => events.push(e) });
  assert.equal(r.ok, true);
  assert.equal(r.accountId, '0123456789abcdef0123456789abcdef');
  assert.equal(r.publicBaseUrl, 'https://pub-xyz.r2.dev');
  assert.equal(r.pagesBaseUrl, 'https://shroom-site.pages.dev');
  assert.deepEqual(r.token, { deferred: true });
  assert.ok(events.includes('cf_token_deferred'));
  // whoami before bucket before public before pages.
  assert.ok(run.calls[0].includes('whoami'));
  assert.ok(run.calls[1].includes('bucket create'));
});

await test('provisionCloudflare: stops at bucket gate, reports stage + dashboard', async () => {
  const run = router([
    { match: has('whoami'), res: WHOAMI_OK },
    { match: has('bucket create'), res: { code: 1, stderr: 'sign up for R2 and accept the terms of service' } },
  ]);
  const r = await provisionCloudflare({ runWrangler: run });
  assert.equal(r.ok, false);
  assert.equal(r.stage, 'bucket');
  assert.equal(r.state, 'r2_not_enabled');
  assert.equal(r.needsDashboard, true);
  assert.ok(!run.calls.some((c) => c.includes('pages project create'))); // never reached
});

await test('provisionCloudflare: uses injected mintR2Token when present', async () => {
  const run = router([
    { match: has('whoami'), res: WHOAMI_OK },
    { match: has('bucket create'), res: { code: 0 } },
    { match: has('dev-url enable'), res: { code: 0, stdout: 'https://pub-xyz.r2.dev' } },
    { match: has('pages project create'), res: { code: 0 } },
  ]);
  const mintR2Token = async ({ accountId, bucket }) => ({ accessKeyId: `ak-${bucket}`, secretAccessKey: 'sk' });
  const r = await provisionCloudflare({ runWrangler: run, mintR2Token });
  assert.equal(r.token.accessKeyId, 'ak-shroom');
});

console.log(`\n${passed} passed`);

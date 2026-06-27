// Offline tests for the setup env probe + install plan. No real binaries are
// spawned — every probe goes through a fake `run` seam that returns canned
// version-command output. Run: node scripts/setup/test/setup.test.mjs

import assert from 'node:assert/strict';

import { probeTool, probeEnv, TOOLS } from '../lib/env-probe.mjs';
import { buildInstallPlan } from '../lib/install-plan.mjs';

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

console.log(`\n${passed} passed`);

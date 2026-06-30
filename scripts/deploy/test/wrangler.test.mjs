// Behaviour tests for the wrangler seam's timeout — the bound that stops a wedged
// `wrangler pages deploy` from hanging the whole publish chain forever. We don't run
// real wrangler; we spawn a tiny node script standing in for a hung / chatty /
// fast-exiting child. Run: node scripts/deploy/test/wrangler.test.mjs

import assert from 'node:assert/strict';

import { spawnWrangler } from '../lib/wrangler.mjs';

const NODE = process.execPath;
// A child that prints a line then sleeps "forever" — the stand-in for a wedged upload.
const HANG = ['-e', "process.stdout.write('Uploading... (6/7)\\n'); setInterval(()=>{}, 1e9);"];
// A child that exits 0 immediately, printing a deploy-like URL line.
const QUICK = ['-e', "process.stdout.write('done https://h.proj.pages.dev\\n'); process.exit(0);"];

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('times out a wedged deploy: kills the child and resolves with code 124 + timedOut', async () => {
  const t0 = Date.now();
  const res = await spawnWrangler(HANG, { bin: NODE, tee: false, timeoutMs: 300 });
  const elapsed = Date.now() - t0;
  assert.equal(res.code, 124);
  assert.equal(res.timedOut, true);
  assert.ok(/stalled with no response/.test(res.stderr), `stderr carries the timeout note: ${res.stderr}`);
  // It actually returned promptly (didn't hang) and captured the partial stdout.
  assert.ok(elapsed < 5000, `resolved promptly, not hung (took ${elapsed}ms)`);
  assert.match(res.stdout, /Uploading/);
});

test('a fast-exiting deploy resolves normally with code 0 and no timedOut', async () => {
  const res = await spawnWrangler(QUICK, { bin: NODE, tee: false, timeoutMs: 5000 });
  assert.equal(res.code, 0);
  assert.notEqual(res.timedOut, true);
  assert.match(res.stdout, /pages\.dev/);
});

test('a missing binary resolves as command-not-found (127), never hangs', async () => {
  const res = await spawnWrangler(['x'], { bin: '/no/such/wrangler-binary', tee: false, timeoutMs: 1000 });
  assert.equal(res.code, 127);
});

for (const [name, fn] of tests) {
  try {
    await fn();
    passed++;
    process.stdout.write(`ok   ${name}\n`);
  } catch (e) {
    process.stdout.write(`FAIL ${name}\n  ${e?.stack || e}\n`);
    process.exitCode = 1;
  }
}
process.stdout.write(`\n${passed}/${tests.length} passed\n`);

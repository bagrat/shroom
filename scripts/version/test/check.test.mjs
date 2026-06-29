// version-check tests: semver precedence is correct, and the whole check is
// fail-soft — an unreachable URL must yield ok:true / updateAvailable:false and
// exit 0, never throw or block. Run: node scripts/version/test/check.test.mjs

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { compareSemver } from '../check.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT = path.resolve(HERE, '..', 'check.mjs');

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('compareSemver orders by numeric core, not lexically', () => {
  assert.equal(compareSemver('0.1.13', '0.1.12'), 1);
  assert.equal(compareSemver('0.1.2', '0.1.10'), -1); // not string compare
  assert.equal(compareSemver('0.1.12', '0.1.12'), 0);
  assert.equal(compareSemver('1.0.0', '0.9.9'), 1);
});

test('a release outranks a prerelease of the same core', () => {
  assert.equal(compareSemver('0.2.0', '0.2.0-beta.1'), 1);
  assert.equal(compareSemver('0.2.0-beta.1', '0.2.0'), -1);
});

test('missing patch is treated as zero', () => {
  assert.equal(compareSemver('1.2', '1.2.0'), 0);
});

test('an unreachable URL fails soft (exit 0, updateAvailable false)', () => {
  const out = execFileSync(
    process.execPath,
    [SCRIPT, '--local', '0.1.0', '--url', 'https://nonexistent.invalid.example/x', '--timeout', '1000'],
    { encoding: 'utf8' },
  );
  const res = JSON.parse(out);
  assert.equal(res.ok, true);
  assert.equal(res.updateAvailable, false);
  assert.equal(res.error, 'fetch_failed');
  assert.equal(res.local, '0.1.0'); // local still resolves even when remote fails
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();

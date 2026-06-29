// run-node tests: the wrapper must (1) dispatch to a Node >= 22 (one with global
// `fetch`, the reason it exists), (2) pass arguments through verbatim, and (3) fail
// loudly with a product-voiced hint when no usable Node can be found — never silently
// fall back to a too-old Node. Run: node scripts/runtime/test/run-node.test.mjs

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WRAPPER = path.resolve(HERE, '..', 'run-node');

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// A throwaway script we can ask the wrapper to run.
function withScript(body, fn) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'run-node-'));
  const file = path.join(dir, 's.mjs');
  fs.writeFileSync(file, body);
  try { return fn(file); }
  finally { fs.rmSync(dir, { recursive: true, force: true }); }
}

test('dispatches to a Node >= 22 with global fetch', () => {
  withScript(
    'process.stdout.write(JSON.stringify([process.version, typeof fetch]));',
    (file) => {
      const out = execFileSync(WRAPPER, [file], { encoding: 'utf8' });
      const [version, fetchType] = JSON.parse(out);
      const major = Number(version.replace(/^v/, '').split('.')[0]);
      assert.ok(major >= 22, `expected Node >= 22, got ${version}`);
      assert.equal(fetchType, 'function'); // the whole point: fetch exists
    },
  );
});

test('passes arguments through verbatim', () => {
  withScript(
    'process.stdout.write(JSON.stringify(process.argv.slice(2)));',
    (file) => {
      const out = execFileSync(WRAPPER, [file, '--flag', 'a b', '42'], { encoding: 'utf8' });
      assert.deepEqual(JSON.parse(out), ['--flag', 'a b', '42']);
    },
  );
});

test('exits non-zero with a product-voiced hint when no usable Node exists', () => {
  // Isolate every discovery path: PATH has only a too-old fake node, NVM_DIR points
  // at an empty dir (no nvm.sh), and the Homebrew keg list is overridden to nothing.
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'run-node-fail-'));
  const fakeNode = path.join(dir, 'node');
  fs.writeFileSync(fakeNode, '#!/bin/sh\necho v16.0.0\n');
  fs.chmodSync(fakeNode, 0o755);
  try {
    let err;
    try {
      execFileSync(WRAPPER, ['whatever.mjs'], {
        encoding: 'utf8',
        // Fake node first so it's the only one found; /usr/bin:/bin only for the
        // shell itself (no nvm/brew dirs → no real node leaks in).
        env: { PATH: `${dir}:/usr/bin:/bin`, HOME: dir, NVM_DIR: dir, SHROOM_RUN_NODE_KEGS: '/no/such/node' },
      });
    } catch (e) { err = e; }
    assert.ok(err, 'expected a non-zero exit');
    assert.equal(err.status, 1);
    assert.match(String(err.stderr), /newer version of Node/);
    assert.match(String(err.stderr), /\/shroom:setup/); // product-voiced, points at setup
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();

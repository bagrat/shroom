// Control-fifo tests — including the regression for THE exit bug: a process that
// opens the control fifo must still be able to process.exit() cleanly. The fifo was
// opened with a blocking read (fs.createReadStream 'r+'), which wedged libuv and
// made process.exit() hang forever — so the recorder finalized on /stop but never
// exited, and its harness-tracked task never completed (publish flow stalled). The
// fix opens the fifo O_RDWR|O_NONBLOCK via net.Socket. Run:
// node scripts/recorder/test/control.test.mjs

import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CONTROL = path.resolve(HERE, '../lib/control.mjs');

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// A tiny child that uses the REAL watchControl: prints each command, and exits 0
// on `stop`. If the fifo wedges exit, the child hangs and the parent times out.
function childSource(fifo) {
  return `
import { watchControl } from ${JSON.stringify(CONTROL)};
const c = watchControl(${JSON.stringify(fifo)});
c.on('command', (cmd) => {
  process.stdout.write('CMD:' + cmd + '\\n');
  if (cmd === 'stop') { c.close(); process.exit(0); }
});
c.on('error', (e) => process.stderr.write('err ' + e.message + '\\n'));
`;
}

function runChild(commands) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-ctl-'));
  const fifo = path.join(dir, 'control.fifo');
  execFileSync('mkfifo', [fifo]);
  const childFile = path.join(dir, 'child.mjs');
  fs.writeFileSync(childFile, childSource(fifo));

  return new Promise((resolve) => {
    const child = spawn('node', [childFile]);
    const got = [];
    let exited = false;
    child.stdout.on('data', (d) => String(d).trim().split('\n').forEach((l) => l && got.push(l)));
    child.on('exit', (code) => { exited = true; resolve({ got, code, hung: false }); });

    // Feed commands with small gaps so each `echo > fifo` is a distinct writer
    // connect/disconnect (the case that used to EOF a naive 'r' reader).
    commands.forEach((cmd, i) => setTimeout(() => {
      try { fs.writeFileSync(fifo, cmd + '\n'); } catch { /* child may have exited */ }
    }, 200 * (i + 1)));

    // Watchdog: if it doesn't exit, that's the regression.
    setTimeout(() => { if (!exited) { try { child.kill('SIGKILL'); } catch {} resolve({ got, code: null, hung: true }); } }, 5000);
  });
}

test('reads newline commands across separate writers, survives disconnects', async () => {
  const { got, hung } = await runChild(['pause', 'resume', 'stop']);
  assert.equal(hung, false, 'child must not hang');
  assert.deepEqual(got, ['CMD:pause', 'CMD:resume', 'CMD:stop']);
});

test('REGRESSION: a process holding the control fifo can process.exit() cleanly', async () => {
  const { code, hung } = await runChild(['stop']);
  assert.equal(hung, false, 'process.exit() must not hang while the fifo is open (the bug)');
  assert.equal(code, 0);
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();

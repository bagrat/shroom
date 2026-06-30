// The wrangler seam — the real `runWrangler` injected into lib/deploy.mjs in
// production. Kept tiny and side-effect-only so the deploy orchestration stays
// testable with a fake (tests never spawn wrangler). Deploys are slow and chatty,
// so we tee wrangler's output to our stderr live AND capture it for URL parsing.
//
// A deploy can wedge mid-upload (observed: wrangler stuck at "Uploading... (6/7)"
// forever) and `spawn` has no built-in timeout, so without one the promise never
// resolves and the whole publish chain hangs. We bound every call: on timeout we
// kill the child (SIGTERM, then SIGKILL if it ignores us) and resolve with a
// distinct `timedOut` flag so the caller can retry or fall back gracefully.

import { spawn } from 'node:child_process';

// A few-MB site bundle deploys in seconds; 90s is generous headroom for a slow
// link while still bounding a wedge. The conventional 124 exit code marks a timeout.
export const DEFAULT_TIMEOUT_MS = 90_000;
const KILL_GRACE_MS = 5_000;
const TIMEOUT_CODE = 124;

export function spawnWrangler(
  args,
  { cwd, env = process.env, bin = 'wrangler', tee = true, timeoutMs = DEFAULT_TIMEOUT_MS } = {},
) {
  return new Promise((resolve) => {
    let stdout = '';
    let stderr = '';
    let child;
    let timer = null;
    let killTimer = null;
    let timedOut = false;
    let settled = false;

    const settle = (result) => {
      if (settled) return;
      settled = true;
      if (timer) clearTimeout(timer);
      if (killTimer) clearTimeout(killTimer);
      resolve(result);
    };

    try {
      child = spawn(bin, args, { cwd, env });
    } catch (e) {
      // ENOENT etc. before the process exists — treat as "command not found".
      settle({ code: 127, stdout: '', stderr: String(e?.message || e) });
      return;
    }

    if (timeoutMs && timeoutMs > 0) {
      timer = setTimeout(() => {
        timedOut = true;
        const note = `wrangler stalled with no response after ${Math.round(timeoutMs / 1000)}s — aborting this deploy.`;
        stderr += `\n${note}\n`;
        if (tee) process.stderr.write(note + '\n');
        try { child.kill('SIGTERM'); } catch { /* already gone */ }
        // Escalate if it ignores the polite signal.
        killTimer = setTimeout(() => { try { child.kill('SIGKILL'); } catch { /* gone */ } }, KILL_GRACE_MS);
      }, timeoutMs);
    }

    child.stdout.on('data', (d) => {
      stdout += d;
      if (tee) process.stderr.write(d);
    });
    child.stderr.on('data', (d) => {
      stderr += d;
      if (tee) process.stderr.write(d);
    });
    child.on('error', (e) => settle({ code: 127, stdout, stderr: stderr + String(e?.message || e) }));
    child.on('close', (code) => settle({ code: timedOut ? TIMEOUT_CODE : (code ?? 1), stdout, stderr, timedOut }));
  });
}

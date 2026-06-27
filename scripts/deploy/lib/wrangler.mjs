// The wrangler seam — the real `runWrangler` injected into lib/deploy.mjs in
// production. Kept tiny and side-effect-only so the deploy orchestration stays
// testable with a fake (tests never spawn wrangler). Deploys are slow and chatty,
// so we tee wrangler's output to our stderr live AND capture it for URL parsing.

import { spawn } from 'node:child_process';

export function spawnWrangler(args, { cwd, env = process.env, bin = 'wrangler', tee = true } = {}) {
  return new Promise((resolve) => {
    let stdout = '';
    let stderr = '';
    let child;
    try {
      child = spawn(bin, args, { cwd, env });
    } catch (e) {
      // ENOENT etc. before the process exists — treat as "command not found".
      resolve({ code: 127, stdout: '', stderr: String(e?.message || e) });
      return;
    }
    child.stdout.on('data', (d) => {
      stdout += d;
      if (tee) process.stderr.write(d);
    });
    child.stderr.on('data', (d) => {
      stderr += d;
      if (tee) process.stderr.write(d);
    });
    child.on('error', (e) => resolve({ code: 127, stdout, stderr: stderr + String(e?.message || e) }));
    child.on('close', (code) => resolve({ code: code ?? 1, stdout, stderr }));
  });
}

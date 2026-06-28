// The silent local-env check (SPEC §8 step 1) — deterministic mechanism, so a
// script. It only *reports*; it never installs (that's a gated mutation the
// command proposes, see install-plan.mjs). Runs first so the setup command only
// prompts about what's actually missing.
//
// Determinism boundary + testability: the side effects are running a tool's
// version command (behind an injected `run` seam) and scanning PATH (behind an
// injected `lookupPath` seam). Tests pass fakes and never touch a real binary or
// the real PATH; production uses the spawn-based runner + PATH scan below.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// One source of truth for the tools shroom needs. `required` gates setup (a missing
// required tool makes the env not-ready). The mechanism still supports optional tools,
// but every tool shroom ships with is currently required — whisper included, since the
// agent layer (titles / chapters / transcript search) depends on it. `detect` is a
// presence+version probe: a 0 exit (or a versionRe match in the output) means present.
// `install` feeds the consolidated install plan.
export const TOOLS = [
  {
    name: 'node',
    required: true,
    // wrangler 4.x hard-requires Node >=22 (it exits with a node-version error
    // before doing anything). Detected here so the probe gates on it instead of
    // letting wrangler fail downstream — and so an old node reads as not-ready,
    // not as a missing binary. `minMajor` makes "present but too old" a failure.
    purpose: 'wrangler 4.x requires Node >=22 to run (SPEC §8)',
    detect: { cmd: 'node', args: ['--version'] },
    versionRe: /v?(\d+\.\d+\.\d+)/,
    minMajor: 22,
    // No auto-install: node is managed per-environment (nvm/fnm/volta/brew), so we
    // can't safely batch it into a brew/npm one-liner. `manual` is skipped by the
    // install plan; the command guides the upgrade instead.
    install: { manager: 'manual', package: 'node>=22 (e.g. `nvm install 22`)' },
  },
  {
    name: 'git',
    required: true,
    purpose: 'the video library is a git repo (SPEC §3)',
    detect: { cmd: 'git', args: ['--version'] },
    versionRe: /git version (\S+)/,
    install: { manager: 'brew', package: 'git' },
  },
  {
    name: 'ffmpeg',
    required: true,
    purpose: 'screen + mic capture and HLS segmenting (SPEC §4)',
    detect: { cmd: 'ffmpeg', args: ['-version'] },
    versionRe: /ffmpeg version (\S+)/,
    install: { manager: 'brew', package: 'ffmpeg' },
  },
  {
    name: 'wrangler',
    required: true,
    purpose: 'Cloudflare login, R2 + Pages provisioning, and deploy (SPEC §8)',
    detect: { cmd: 'wrangler', args: ['--version'] },
    versionRe: /(\d+\.\d+\.\d+)/,
    // On Node <22, `wrangler --version` prints "Wrangler requires at least
    // Node.js v22.0.0…" — whose version number our versionRe would otherwise
    // scrape as wrangler's, falsely reporting it present+working. `unhealthyRe`
    // marks that as NOT present (with a reason), so the node gate is what surfaces.
    unhealthyRe: /requires at least Node/i,
    install: { manager: 'npm', package: 'wrangler' },
  },
  {
    name: 'whisper',
    required: true,
    purpose: 'local transcription → titles / chapters / search (SPEC §7)',
    detect: { cmd: 'whisper', args: ['--help'] },
    versionRe: null, // whisper --help has no version line; presence is enough
    install: { manager: 'brew', package: 'openai-whisper' },
  },
];

// Real runner: spawn `cmd args`, capture output, never throw. ENOENT (binary not
// on PATH) resolves to code 127 — the "not installed" signal. A short timeout
// guards against a tool that hangs waiting on input.
export function spawnRun(cmd, args, { timeoutMs = 5000 } = {}) {
  return new Promise((resolve) => {
    let stdout = '';
    let stderr = '';
    let child;
    try {
      child = spawn(cmd, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    } catch (e) {
      resolve({ code: 127, stdout: '', stderr: String(e?.message || e) });
      return;
    }
    const timer = setTimeout(() => child.kill('SIGKILL'), timeoutMs);
    child.stdout.on('data', (d) => (stdout += d));
    child.stderr.on('data', (d) => (stderr += d));
    child.on('error', (e) => {
      clearTimeout(timer);
      resolve({ code: 127, stdout, stderr: stderr + String(e?.message || e) });
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      resolve({ code: code ?? 1, stdout, stderr });
    });
  });
}

// Pure PATH lookup — is `cmd` an executable file on PATH? No subprocess, so it's
// fast and immune to a tool's startup cost (whisper imports torch on every
// invocation; a cold `whisper --help` can blow a multi-second timeout and flap to
// "absent"). Used for presence-only tools, where we never need to parse a version.
export function pathLookup(cmd, { env = process.env } = {}) {
  if (cmd.includes('/')) return fs.existsSync(cmd) ? cmd : null;
  for (const dir of (env.PATH || '').split(path.delimiter)) {
    if (!dir) continue;
    const full = path.join(dir, cmd);
    try {
      fs.accessSync(full, fs.constants.X_OK);
      return full;
    } catch { /* not here / not executable — keep scanning */ }
  }
  return null;
}

// Find a Node >=minMajor bin directory, without changing the user's default node.
// shroom shells out to wrangler (which needs Node >=22); persisting this dir in the
// creds lets the wrangler seam prefix PATH so wrangler runs under a new-enough node
// even when the machine default is older. Best-effort + side-effect-free:
//   1. the node we're already running, if it qualifies (the common case);
//   2. the newest nvm-installed node >=minMajor (version is encoded in the path);
//   3. null — caller falls back to bare `wrangler` and surfaces the node error.
export function findNodeBinDir({ minMajor = 22, home = os.homedir(), fsmod = fs } = {}) {
  const major = Number((process.versions?.node || '0').split('.')[0]);
  if (Number.isFinite(major) && major >= minMajor) return path.dirname(process.execPath);

  const nvmRoot = path.join(home, '.nvm', 'versions', 'node');
  let best = null;
  try {
    for (const name of fsmod.readdirSync(nvmRoot)) {
      const m = name.match(/^v(\d+)\.(\d+)\.(\d+)$/);
      if (!m) continue;
      const v = [Number(m[1]), Number(m[2]), Number(m[3])];
      if (v[0] < minMajor) continue;
      const bin = path.join(nvmRoot, name, 'bin');
      if (!fsmod.existsSync(path.join(bin, 'node'))) continue;
      if (!best || cmpVer(v, best.v) > 0) best = { v, bin };
    }
  } catch { /* no nvm dir — fall through */ }
  return best ? best.bin : null;
}
function cmpVer(a, b) {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

function extractVersion(tool, out) {
  if (!tool.versionRe) return null;
  const m = String(out).match(tool.versionRe);
  return m ? m[1] : null;
}

// Probe one tool. Two strategies:
//   - version-bearing (`versionRe` set): run the version command — a 0 exit (or a
//     version match even on non-zero) proves presence AND yields the version.
//   - presence-only (`versionRe: null`): check PATH directly, never execute. Right
//     for slow-to-start tools we don't need a version from.
export async function probeTool(tool, { run = spawnRun, lookupPath = pathLookup } = {}) {
  if (tool.versionRe == null) {
    const present = lookupPath(tool.detect.cmd) != null;
    return { name: tool.name, required: tool.required, purpose: tool.purpose, present, version: null, install: tool.install };
  }
  const res = await run(tool.detect.cmd, tool.detect.args);
  const out = `${res.stdout ?? ''}\n${res.stderr ?? ''}`;
  const version = extractVersion(tool, out);

  // A tool can be on PATH yet non-functional: e.g. wrangler on Node <22 exits with
  // a node-version error that happens to contain a version number. `unhealthyRe`
  // catches that so we don't count a broken binary as present.
  if (tool.unhealthyRe && tool.unhealthyRe.test(out)) {
    return { name: tool.name, required: tool.required, purpose: tool.purpose, present: false, version: null, reason: 'unhealthy', install: tool.install };
  }
  // "Present but too old": a parsed version below `minMajor` is a failure, not a
  // missing binary — the command guides an upgrade rather than an install.
  if (tool.minMajor != null && version != null) {
    const major = Number(version.split('.')[0]);
    if (Number.isFinite(major) && major < tool.minMajor) {
      return { name: tool.name, required: tool.required, purpose: tool.purpose, present: false, version, reason: `below_min_v${tool.minMajor}`, install: tool.install };
    }
  }

  const present = res.code === 0 || version != null;
  return {
    name: tool.name,
    required: tool.required,
    purpose: tool.purpose,
    present,
    version,
    install: tool.install,
  };
}

// Probe every tool. Probes are independent → run them in parallel.
export async function probeEnv({ run = spawnRun, lookupPath = pathLookup, tools = TOOLS } = {}) {
  const results = await Promise.all(tools.map((t) => probeTool(t, { run, lookupPath })));
  const missingRequired = results.filter((r) => r.required && !r.present);
  return {
    results,
    ready: missingRequired.length === 0,
    missingRequired: missingRequired.map((r) => r.name),
    missingOptional: results.filter((r) => !r.required && !r.present).map((r) => r.name),
  };
}

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
import path from 'node:path';

// One source of truth for the tools shroom needs. `required` gates setup; an
// optional tool missing is a soft note (whisper powers the agent layer, but you
// can record + publish without it). `detect` is a presence+version probe: a
// 0 exit (or a versionRe match in the output) means present. `install` feeds the
// consolidated install plan.
export const TOOLS = [
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
    install: { manager: 'npm', package: 'wrangler' },
  },
  {
    name: 'whisper',
    required: false,
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

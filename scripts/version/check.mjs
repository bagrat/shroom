#!/usr/bin/env node
// version check — the deterministic half of "suggest an update" (green-set item
// 3). It NEVER updates anything: it reads the installed plugin version, fetches
// the latest published version, semver-compares, and prints the verdict as JSON.
// Whether/how to surface an update is the command's judgment (the determinism
// boundary, CLAUDE.md) — this just reports the fact.
//
// The latest version is authoritative in plugin.json on shroom's main branch:
// the marketplace tracks the repo UNPINNED, so what's on main is what a user gets
// on `/plugin marketplace update`. We read it from the raw GitHub URL.
//
// Fail-soft is the whole point: offline, a slow network, a moved file, a parse
// error — ANY problem yields { ok:true, updateAvailable:false, error:... } and
// exit 0, so a version check can never block or slow a record/setup.
//
// Output (always JSON):
//   { ok, local, latest, updateAvailable, error?, detail?, source }
//
// Flags:
//   --url <u>      override the remote plugin.json URL (testing)
//   --local <v>    override the local version (testing)
//   --timeout <ms> fetch timeout (default 2500)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));

// Canonical source of the latest published version (see header).
const DEFAULT_URL =
  'https://raw.githubusercontent.com/bagrat/shroom/main/.claude-plugin/plugin.json';

function parseArgs(argv) {
  const a = { timeout: 2500 };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === '--url') a.url = argv[++i];
    else if (k === '--local') a.local = argv[++i];
    else if (k === '--timeout') a.timeout = Number(argv[++i]) || 2500;
  }
  return a;
}

// Resolve the installed plugin's own version from its plugin.json. The script
// lives at <root>/scripts/version/check.mjs, so the manifest is two dirs up.
function localVersion(override) {
  if (override) return override;
  const root = process.env.CLAUDE_PLUGIN_ROOT || path.resolve(HERE, '..', '..');
  const manifest = path.join(root, '.claude-plugin', 'plugin.json');
  return JSON.parse(fs.readFileSync(manifest, 'utf8')).version;
}

// Compare two "x.y.z" (optionally "-prerelease") versions. Returns 1 if a>b,
// -1 if a<b, 0 if equal. Numeric core compare; a release outranks a prerelease
// of the same core (standard semver precedence, kept minimal — no deps).
export function compareSemver(a, b) {
  const split = (v) => {
    const [core, pre] = String(v).split('-', 2);
    const nums = core.split('.').map((n) => parseInt(n, 10) || 0);
    while (nums.length < 3) nums.push(0);
    return { nums, pre };
  };
  const A = split(a);
  const B = split(b);
  for (let i = 0; i < 3; i++) {
    if (A.nums[i] !== B.nums[i]) return A.nums[i] > B.nums[i] ? 1 : -1;
  }
  if (A.pre && !B.pre) return -1; // a is a prerelease of b's release → older
  if (!A.pre && B.pre) return 1;
  if (A.pre && B.pre) return A.pre === B.pre ? 0 : A.pre > B.pre ? 1 : -1;
  return 0;
}

async function fetchLatest(url, timeout) {
  if (typeof fetch !== 'function') throw new Error('no_fetch'); // Node < 18
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), timeout);
  try {
    const res = await fetch(url, { signal: ctrl.signal, redirect: 'follow' });
    if (!res.ok) throw new Error(`http_${res.status}`);
    const json = JSON.parse(await res.text());
    if (!json.version) throw new Error('no_version_field');
    return json.version;
  } finally {
    clearTimeout(timer);
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const url = args.url || DEFAULT_URL;

  let local;
  try {
    local = localVersion(args.local);
  } catch (e) {
    // Can't even read our own version — report soft and stop. Never throw.
    return { ok: true, local: null, latest: null, updateAvailable: false, error: 'no_local_version', detail: e.message, source: url };
  }

  try {
    const latest = await fetchLatest(url, args.timeout);
    return {
      ok: true,
      local,
      latest,
      updateAvailable: compareSemver(latest, local) > 0,
      source: url,
    };
  } catch (e) {
    return { ok: true, local, latest: null, updateAvailable: false, error: 'fetch_failed', detail: e.message, source: url };
  }
}

// Run only when invoked directly — importing this module (e.g. for compareSemver
// in tests) must not trigger the network check or process.exit.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().then((out) => {
    process.stdout.write(JSON.stringify(out) + '\n');
    process.exit(0);
  });
}

#!/usr/bin/env node
// publish — the one deterministic publish step the record skill calls (SPEC §6).
//
// Folds the three mechanical steps that used to be separate command invocations —
// build the page, `wrangler pages deploy`, and commit the `<id>.md` to the git
// library — into a SINGLE process, so the user approves one command instead of
// three (and the commit is automatic, per Bagrat's call: the record belongs in his
// own library, it's part of the normal lifecycle, not a surprising mutation).
//
// It reuses build-page.mjs and deploy.mjs verbatim as child processes (no logic
// duplication; recovery still works through those standalone entry points). The
// judgment bits — title, chapters — are authored upstream (skills) and arrive via
// --meta; this step is pure mechanism.
//
// Usage:
//   node publish.mjs --session <dir> [--meta <id.md>] [--title <t>] [--id <id>]
//                    [--branch <name>] [--wrangler <bin>]
// Reads ~/.shroom/credentials.json for the Pages project + library. If Cloudflare
// isn't provisioned (no pagesProject), it builds locally and skips deploy. Commit
// is best-effort: a missing/uninitialized library never fails the publish.
// Output: ndjson events on stdout; the terminal `published` (or `publish_local`)
// carries the shareable link, and `committed`/`commit_skipped` reports the git step.

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { readCreds, credsPath } from '../setup/lib/credentials.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const BUILD_PAGE = path.join(HERE, 'build-page.mjs');
const DEPLOY = path.resolve(HERE, '../deploy/deploy.mjs');

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : 'true';
  }
  return o;
}
const flag = (o, k) => (o[k] !== undefined && o[k] !== 'true' ? o[k] : undefined);

const log = (event, fields = {}) =>
  process.stdout.write(JSON.stringify({ ts: new Date().toISOString(), event, ...fields }) + '\n');

// Run a child node script: stream its stderr through (live progress) and capture
// stdout so we can pull out its terminal JSON/ndjson event.
function runNode(scriptPath, args) {
  const res = spawnSync(process.execPath, [scriptPath, ...args], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  return { code: res.status ?? 1, stdout: res.stdout ?? '' };
}

// Last parseable JSON object emitted on a child's stdout (its summary/terminal event).
function lastJson(stdout, predicate = () => true) {
  let found = null;
  for (const line of stdout.split('\n')) {
    if (!line.trim()) continue;
    try {
      const e = JSON.parse(line);
      if (predicate(e)) found = e;
    } catch { /* non-JSON progress line */ }
  }
  return found;
}

function git(libraryDir, args) {
  const res = spawnSync('git', ['-C', libraryDir, ...args], { encoding: 'utf8' });
  return { code: res.status ?? 1, stdout: res.stdout ?? '', stderr: res.stderr ?? '' };
}

// Commit <id>.md to the library. Best-effort by design — never throws, never fails
// the publish. Returns the event fields to log.
function commitRecord({ libraryDir, id, title }) {
  if (!libraryDir) return ['commit_skipped', { reason: 'no_library' }];
  if (git(libraryDir, ['rev-parse', '--is-inside-work-tree']).code !== 0)
    return ['commit_skipped', { reason: 'not_a_git_repo', library: libraryDir }];
  const rel = `${id}.md`;
  if (!fs.existsSync(path.join(libraryDir, rel)))
    return ['commit_skipped', { reason: 'no_record_file', file: rel }];

  const add = git(libraryDir, ['add', rel]);
  if (add.code !== 0) return ['commit_skipped', { reason: 'git_add_failed', detail: add.stderr.trim() }];

  // Nothing staged (re-publish with no metadata change, e.g. an enrich no-op) → not
  // an error; the record is already committed.
  if (git(libraryDir, ['diff', '--cached', '--quiet']).code === 0)
    return ['commit_noop', { reason: 'already_committed', file: rel }];

  const msg = `Add recording: ${title || id}`;
  const c = git(libraryDir, ['commit', '-m', msg]);
  if (c.code !== 0) return ['commit_skipped', { reason: 'git_commit_failed', detail: c.stderr.trim() }];
  return ['committed', { file: rel, message: msg }];
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  const sessionDir = flag(opts, 'session');
  if (!sessionDir) { process.stderr.write('A session dir is required: --session <dir>.\n'); process.exit(2); }
  const metaPath = flag(opts, 'meta');
  const branch = flag(opts, 'branch');
  const wrangler = flag(opts, 'wrangler');

  const creds = readCreds(credsPath());
  const pagesProject = creds.pagesProject;
  const libraryDir = creds.library;

  // 1. Build the per-video page (poster + baked og: tags). Deterministic, offline.
  const buildArgs = ['--session', sessionDir];
  if (metaPath) buildArgs.push('--meta', metaPath);
  const build = runNode(BUILD_PAGE, buildArgs);
  const built = lastJson(build.stdout, (e) => e.event === 'page_built');
  if (build.code !== 0 || !built) {
    log('publish_failed', { stage: 'build', detail: 'build-page did not produce a page' });
    process.exit(1);
  }
  const id = flag(opts, 'id') ?? built.id;

  // 2. Deploy — only if Cloudflare is provisioned. Otherwise it's a local render.
  let playbackUrl = null;
  let deployFailReason = null;
  if (pagesProject) {
    const deployArgs = ['--project', pagesProject, '--session', sessionDir];
    if (branch) deployArgs.push('--branch', branch);
    if (wrangler) deployArgs.push('--wrangler', wrangler);
    const dep = runNode(DEPLOY, deployArgs);
    const pub = lastJson(dep.stdout, (e) => e.event === 'published');
    if (dep.code !== 0 || !pub?.playbackUrl) {
      // The deploy didn't complete (e.g. wrangler wedged/timed out). Don't hang and
      // don't hard-fail: the recording's bytes are already in the bucket and the page
      // is built locally, so degrade to a local preview and let the user re-publish to
      // get the live link. We carry the deploy's own reason for the user-facing wording.
      const failEvt = lastJson(dep.stdout, (e) => e.event === 'deploy_failed');
      deployFailReason = failEvt?.reason || 'deploy_incomplete';
    } else {
      playbackUrl = pub.playbackUrl;
    }
  }

  // 3. Commit the record to the git library — automatic, best-effort.
  const title = flag(opts, 'title');
  const [commitEvent, commitFields] = commitRecord({ libraryDir, id, title });
  log(commitEvent, commitFields);

  // Terminal event the skill reads for the link. A local render is the terminal
  // result both when Cloudflare isn't provisioned AND when a provisioned deploy
  // didn't complete — `deployFailed` distinguishes the two so the skill can say
  // "couldn't reach the publish service, try again" rather than "set up sharing".
  if (playbackUrl) {
    log('published', { id, playbackUrl, committed: commitEvent === 'committed' || commitEvent === 'commit_noop' });
  } else {
    log('publish_local', {
      id,
      indexPath: built.indexPath,
      preview: path.join(path.resolve(sessionDir), 'preview.mp4'),
      ...(deployFailReason ? { deployFailed: true, reason: deployFailReason } : {}),
    });
  }
}

main().catch((e) => {
  process.stderr.write(`publish fatal: ${e?.stack || e}\n`);
  process.exit(1);
});

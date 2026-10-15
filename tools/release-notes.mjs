#!/usr/bin/env node
// A release's notes (DECISIONS B152): what a person reads behind "What's new".
//
//   node tools/release-notes.mjs check <version>               the notes are there and readable
//   node tools/release-notes.mjs check --all                   every notes file is readable
//   node tools/release-notes.mjs body <version> <generated.md> the release's body, on stdout
//
// The notes are written by hand, one file per version: docs/release-notes/<version>.md, a
// short list of what someone using shroom would notice, in shroom's voice. The body is that
// list under "What's new", then everything else that changed — GitHub's generated list of
// pull requests — folded away under UNDER_THE_HOOD. release.yml refuses to build a version
// whose notes are missing or name the build's own insides.
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'docs', 'release-notes');
export const UNDER_THE_HOOD = 'Under the hood';

// Words that belong to how shroom is built, never to what a person sees (CLAUDE.md, product
// voice): the record's numbering, the repo's files, the build's own machinery.
const INSIDES = [
  [/\bB\d+\b/, 'a decision number (B…)'],
  [/DEVIATIONS/, 'the deviations'],
  [/#\d+/, 'a pull request or issue number'],
  [/CLAUDE\.md|machine\.json|MACHINE\.md|\.rs\b|\.mjs\b|\.ya?ml\b/, 'a file in the repo'],
  [/\bPRs?\b|pull request/i, 'pull requests'],
  [/\bCI\b|workflow|heartbeat|e2e\b/i, 'the build and its tests'],
  [/\bWorker\b|\bcrate\b|\bTauri\b|\bwrangler\b/i, 'an internal name'],
  [/@[A-Za-z0-9-]+/, 'a handle'],
];

/** What's wrong with a notes file's text, as lines a person can fix; empty when it's fine. */
export function problems(text) {
  const out = [];
  const items = text.split('\n').filter((l) => /^[-*] \S/.test(l));
  if (items.length === 0) out.push('no list: each change a person would notice is a line starting "- "');
  text.split('\n').forEach((line, i) => {
    for (const [re, what] of INSIDES) {
      if (re.test(line)) out.push(`line ${i + 1} names ${what}: ${line.trim()}`);
    }
  });
  if (/<\/?details|<summary/i.test(text)) out.push(`the "${UNDER_THE_HOOD}" fold is added for you; the file holds only what's new`);
  return out;
}

export function notesPath(version) {
  return join(DIR, `${version}.md`);
}

/** The release body: what's new, then GitHub's generated list folded away. */
export function body(notes, generated) {
  const rest = generated
    .replace(/\r\n/g, '\n')
    .replace(/^## What's Changed\n+/m, '')
    .trim();
  let out = `## What's new\n\n${notes.trim()}\n`;
  if (rest) out += `\n<details>\n<summary>${UNDER_THE_HOOD}</summary>\n\n${rest}\n\n</details>\n`;
  return out;
}

function check(version) {
  const path = notesPath(version);
  if (!existsSync(path)) {
    return [`docs/release-notes/${version}.md isn't written: a release's notes are written by hand before it's cut (docs/RELEASING.md).`];
  }
  return problems(readFileSync(path, 'utf8')).map((p) => `docs/release-notes/${version}.md: ${p}`);
}

function main(argv) {
  const [cmd, a, b] = argv;
  if (cmd === 'check' && a) {
    const versions = a === '--all' ? readdirSync(DIR).filter((f) => f.endsWith('.md')).map((f) => f.slice(0, -3)) : [a];
    const errs = versions.flatMap(check);
    for (const e of errs) console.error(e);
    if (errs.length) process.exit(1);
    console.log(`release notes readable: ${versions.join(', ')}`);
    return;
  }
  if (cmd === 'body' && a && b) {
    const errs = check(a);
    if (errs.length) {
      for (const e of errs) console.error(e);
      process.exit(1);
    }
    process.stdout.write(body(readFileSync(notesPath(a), 'utf8'), readFileSync(b, 'utf8')));
    return;
  }
  console.error('usage: release-notes.mjs check <version>|--all · body <version> <generated.md>');
  process.exit(64);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) main(process.argv.slice(2));

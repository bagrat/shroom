#!/usr/bin/env node
// search — the deterministic retrieve half of transcript search (green-set item
// 8). It indexes the git-library corpus (every `<id>.md`: title, TL;DR, chapter
// labels, transcript body) and returns the best lexical candidates for a query,
// each with a snippet + the public link + any matching chapters.
//
// The DETERMINISM split (CLAUDE.md): this does the repeatable, dependency-free
// retrieval — tokenize, weighted term-frequency scoring, snippet extraction. The
// SEMANTIC half — turning the user's natural-language question into search terms,
// then judging which candidate actually answers it — is the `search` skill. No
// external service / embeddings for v1 (SPEC: your bytes, your machine).
//
// Subcommand:
//   query --q "<terms>" [--limit N] [--library <dir>]
//     → { ok, query, library, count, results: [ { id, title, link, score,
//         matchedTerms, snippet, chapters: [{ t, time, label }] } ] }

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { parseMetadata } from '../page/lib/metadata.mjs';
import { formatDuration } from '../page/lib/render.mjs';
import { readCreds, credsPath } from '../setup/lib/credentials.mjs';

const HOME = os.homedir();

// Tiny stoplist — terms too common to discriminate. The skill picks query terms,
// so this only guards against a bare "the"/"and" dominating the score.
const STOP = new Set('a an and are as at be but by for from has have how in into is it its of on or that the their then there these this to was were what when which who why with you your'.split(' '));

export function tokenize(text) {
  return (String(text || '').toLowerCase().match(/[a-z0-9]+/g)) || [];
}

function counts(text) {
  const m = new Map();
  for (const tok of tokenize(text)) m.set(tok, (m.get(tok) || 0) + 1);
  return m;
}

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : true;
  }
  return o;
}

// Load the corpus: every `<id>.md` → { id, title, tldr, chapters, transcript, createdAt }.
export function loadCorpus(libraryDir) {
  const records = [];
  let files;
  try { files = fs.readdirSync(libraryDir); } catch { return records; }
  for (const f of files) {
    if (!f.endsWith('.md') || f === 'README.md') continue;
    try {
      const { meta, transcript } = parseMetadata(fs.readFileSync(path.join(libraryDir, f), 'utf8'));
      records.push({
        id: meta.id || f.replace(/\.md$/, ''),
        title: meta.title || '',
        tldr: meta.tldr || '',
        chapters: Array.isArray(meta.chapters) ? meta.chapters : [],
        transcript: transcript || '',
        createdAt: meta.createdAt || null,
      });
    } catch { /* skip an unreadable record */ }
  }
  return records;
}

// ~180-char window around the first transcript hit, with ellipses. Plain text.
function snippetFor(transcript, terms) {
  const lower = transcript.toLowerCase();
  let at = -1;
  for (const t of terms) {
    const i = lower.indexOf(t);
    if (i >= 0 && (at < 0 || i < at)) at = i;
  }
  if (at < 0) return transcript.slice(0, 160).trim() + (transcript.length > 160 ? '…' : '');
  const start = Math.max(0, at - 70);
  const end = Math.min(transcript.length, at + 110);
  return (start > 0 ? '…' : '') + transcript.slice(start, end).replace(/\s+/g, ' ').trim() + (end < transcript.length ? '…' : '');
}

// Score one record against the query terms (weighted term frequency + a phrase
// bonus when the raw query appears verbatim). Title/TL;DR/chapters outweigh body.
function scoreRecord(rec, terms, rawQuery) {
  const fields = [
    [counts(rec.title), 4],
    [counts(rec.tldr), 2],
    [counts(rec.chapters.map((c) => c.label).join(' ')), 2],
    [counts(rec.transcript), 1],
  ];
  let score = 0;
  const matched = new Set();
  for (const t of terms) {
    for (const [cmap, w] of fields) {
      const c = cmap.get(t) || 0;
      if (c) { score += c * w; matched.add(t); }
    }
  }
  if (score === 0) return null;
  const q = rawQuery.trim().toLowerCase();
  if (q.length >= 4) {
    const hay = `${rec.title}\n${rec.tldr}\n${rec.transcript}`.toLowerCase();
    if (hay.includes(q)) score += 6; // exact phrase present
  }
  return { score, matched: [...matched] };
}

export function searchCorpus(records, rawQuery, { limit = 5 } = {}) {
  const terms = [...new Set(tokenize(rawQuery).filter((t) => !STOP.has(t)))];
  if (!terms.length) return [];
  const scored = [];
  for (const rec of records) {
    const s = scoreRecord(rec, terms, rawQuery);
    if (!s) continue;
    const chapters = rec.chapters
      .filter((c) => c && c.label && terms.some((t) => String(c.label).toLowerCase().includes(t)))
      .map((c) => ({ t: Number(c.t) || 0, time: formatDuration(Number(c.t) || 0), label: c.label }));
    scored.push({
      id: rec.id, title: rec.title || 'Untitled recording',
      score: s.score, matchedTerms: s.matched,
      snippet: snippetFor(rec.transcript, terms),
      chapters,
      createdAt: rec.createdAt,
    });
  }
  scored.sort((a, b) => b.score - a.score || (Date.parse(b.createdAt || 0) || 0) - (Date.parse(a.createdAt || 0) || 0));
  return scored.slice(0, limit);
}

function cmdQuery(opts) {
  const rawQuery = opts.q === true || opts.q == null ? '' : String(opts.q);
  if (!rawQuery.trim()) return { ok: false, reason: 'no_query', hint: 'pass --q "<search terms>"' };
  const creds = readCreds(credsPath());
  const libraryDir = opts.library ? path.resolve(String(opts.library)) : (creds.library || path.join(HOME, 'shroom'));
  const pagesBaseUrl = creds.pagesBaseUrl || '';
  const records = loadCorpus(libraryDir);
  const limit = Number(opts.limit) > 0 ? Number(opts.limit) : 5;
  const results = searchCorpus(records, rawQuery, { limit }).map((r) => ({
    ...r,
    link: pagesBaseUrl ? `${pagesBaseUrl.replace(/\/+$/, '')}/${r.id}/` : null,
  }));
  return { ok: true, query: rawQuery, library: libraryDir, corpusSize: records.length, count: results.length, results };
}

function main() {
  const [sub, ...rest] = process.argv.slice(2);
  const opts = parseArgs(rest);
  if (sub !== 'query') {
    process.stderr.write(`unknown subcommand: ${sub || '(none)'}\nexpected: query --q "<terms>"\n`);
    process.exit(2);
  }
  const out = cmdQuery(opts);
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  process.exit(out.ok === false ? 1 : 0);
}

// argv[1] may be a symlink (e.g. a skills-dir symlink); resolve it so it matches
// import.meta.url, which Node resolves through symlinks — else main() is skipped.
const entryPath = process.argv[1] && fs.realpathSync(process.argv[1]);
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  main();
}

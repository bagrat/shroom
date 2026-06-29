// The per-video metadata record (SPEC §3): a tiny `<id>.md` in the git library
// holding frontmatter (metadata) + the transcript as the body. This is the
// substrate the page renderer reads from — the agent (a skill, M5) writes title /
// tldr / chapters here; this module is the deterministic read/write half so both
// sides agree on the shape.
//
// Frontmatter is a deliberately *small* YAML subset (dependency-free, fully
// round-trippable): `key: value` scalar lines, where a value that begins with `[`
// or `{` is parsed as inline JSON (used for `chapters`). Everything else is a
// string. Anything richer is out of scope on purpose.

import fs from 'node:fs';

const FENCE = '---';

// Scalars we serialize as bare/quoted strings; chapters round-trips as inline JSON.
function serializeValue(v) {
  if (Array.isArray(v) || (v && typeof v === 'object')) return JSON.stringify(v);
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  const s = String(v ?? '');
  // Quote if it could be mis-read (leading/trailing space, or starts like JSON/quote).
  if (s === '' || /^[\[{"']/.test(s) || s !== s.trim() || /[:#]/.test(s)) {
    return JSON.stringify(s);
  }
  return s;
}

function parseValue(raw) {
  const s = raw.trim();
  if (s === '') return '';
  if (s[0] === '[' || s[0] === '{' || s[0] === '"') {
    try { return JSON.parse(s); } catch { /* fall through to string */ }
  }
  if (/^-?\d+(\.\d+)?$/.test(s)) return Number(s);
  if (s === 'true' || s === 'false') return s === 'true';
  return s;
}

// Parse a `<id>.md` document → { meta: {...}, transcript: "..." }.
export function parseMetadata(text) {
  const meta = {};
  let transcript = text ?? '';
  const lines = (text ?? '').split('\n');
  if (lines[0]?.trim() === FENCE) {
    let i = 1;
    for (; i < lines.length; i++) {
      if (lines[i].trim() === FENCE) break;
      const m = lines[i].match(/^([A-Za-z0-9_]+):\s?(.*)$/);
      if (m) meta[m[1]] = parseValue(m[2]);
    }
    transcript = lines.slice(i + 1).join('\n').trim();
  }
  return { meta, transcript };
}

// Serialize { meta, transcript } → a `<id>.md` document. Key order is stable so
// re-saving an unchanged record produces an unchanged file (clean git diffs).
const KEY_ORDER = ['id', 'title', 'tldr', 'durationSec', 'createdAt', 'chapters', 'mp4'];
export function serializeMetadata({ meta = {}, transcript = '' } = {}) {
  const keys = [
    ...KEY_ORDER.filter((k) => k in meta),
    ...Object.keys(meta).filter((k) => !KEY_ORDER.includes(k)),
  ];
  const front = keys.map((k) => `${k}: ${serializeValue(meta[k])}`).join('\n');
  const body = (transcript ?? '').trim();
  return `${FENCE}\n${front}\n${FENCE}\n${body ? body + '\n' : ''}`;
}

export function readMetadataFile(filePath) {
  if (!fs.existsSync(filePath)) return null;
  return parseMetadata(fs.readFileSync(filePath, 'utf8'));
}

export function writeMetadataFile(filePath, record) {
  fs.writeFileSync(filePath, serializeMetadata(record));
}

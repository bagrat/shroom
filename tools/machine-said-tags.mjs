// The said-lines on the shroom UI canvas's screen boards, held to spec/machine.json (B133,
// machine-json phase 4). Each line a board shows is tagged with its said-line's id
// (`data-said="paused.resume"`); a tagged line whose words differ from the JSON's is
// **drift**: retyped on the canvas, or changed in the JSON. Drift is reported, never
// silently overwritten — the session proposes the canvas's words into the JSON, or reverts
// the board (docs/DESIGN.md, the two lanes).
//
//   node tools/machine-said-tags.mjs <root>           # report: tagged, drifted, untagged
//   node tools/machine-said-tags.mjs <root> --tag     # also tag what matches, in place
//
// <root>/project/ holds the canvas's boards as read. A text matches a said-line when its
// words are the line's exactly (spaces, curly quotes and non-breaking spaces aside); a
// line said by several states is tagged with the one whose screen board this is (its
// own state before its faces), else left untagged. The generated machine boards are skipped.
import fs from 'node:fs';
import path from 'node:path';

const REPO = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const ROOT = process.argv[2];
const TAG = process.argv.includes('--tag');
if (!ROOT) {
  console.error('usage: node tools/machine-said-tags.mjs <root> [--tag]');
  process.exit(2);
}
const PROJECT = path.join(ROOT, 'project');
const machine = JSON.parse(fs.readFileSync(path.join(REPO, 'spec/machine.json'), 'utf8'));
const S = machine.states;

const norm = (t) => t.replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'")
  .replace(/[‘’]/g, "'").replace(/[“”]/g, '"').replace(/[   ]/g, ' ').replace(/\s+/g, ' ').trim();

// Every said-line by its words, and by its id.
const byText = new Map();
const byId = new Map();
for (const [sid, s] of Object.entries(S)) {
  for (const l of s.said) {
    byId.set(l.id, { ...l, state: sid });
    const k = norm(l.text);
    if (!byText.has(k)) byText.set(k, []);
    byText.get(k).push({ id: l.id, state: sid });
  }
}
// The states a board draws: those that name it, their faces, and their parents.
const statesOf = (board) => {
  const out = new Set();
  for (const [sid, s] of Object.entries(S)) {
    if (s.board !== board) continue;
    out.add(sid);
    if (s.parent) out.add(s.parent);
    for (const [fid, f] of Object.entries(S)) if (f.parent === sid) out.add(fid);
  }
  return out;
};

const LEAF = /<(h[1-6]|span|div|button|a|p|li|label|b|strong|em)(\s[^<>]*)?>([^<>]+)<\/\1>/g;
const report = { tagged: 0, added: 0, drift: [], ambiguous: [], boards: 0 };
for (const file of fs.readdirSync(PROJECT).filter((f) => f.endsWith('.dc.html') && !/^Machine/.test(f)).sort()) {
  const board = file.replace(/\.dc\.html$/, '');
  const mine = statesOf(board);
  const src = fs.readFileSync(path.join(PROJECT, file), 'utf8');
  let changed = false;
  const out = src.replace(LEAF, (whole, tag, attrs = '', text) => {
    const words = norm(text);
    const said = attrs.match(/\sdata-said="([^"]+)"/);
    if (said) {
      report.tagged += 1;
      const line = byId.get(said[1]);
      if (!line) report.drift.push({ board, id: said[1], canvas: words, json: '(no such said-line)' });
      else if (norm(line.text) !== words) report.drift.push({ board, id: said[1], canvas: words, json: norm(line.text) });
      return whole;
    }
    const hits = byText.get(words);
    if (!hits || words.length < 2) return whole;
    // The board's own state first, then a state over its faces.
    let here = hits.filter((h) => mine.has(h.state));
    for (const keep of [(h) => S[h.state].board === board, (h) => !['face', 'tray-face'].includes(S[h.state].kind)]) {
      if (here.length > 1 && here.some(keep)) here = here.filter(keep);
    }
    const pick = here.length === 1 ? here[0] : here.length === 0 && hits.length === 1 ? hits[0] : null;
    if (!pick) {
      report.ambiguous.push({ board, words, ids: (here.length ? here : hits).map((h) => h.id) });
      return whole;
    }
    if (!TAG) return whole;
    changed = true;
    report.added += 1;
    return `<${tag}${attrs} data-said="${pick.id}">${text}</${tag}>`;
  });
  report.boards += 1;
  if (changed) fs.writeFileSync(path.join(PROJECT, file), out);
}
console.log(`${report.boards} boards · ${report.tagged} lines tagged before · ${report.added} tagged now · ${report.drift.length} drifted · ${report.ambiguous.length} said by several states, left untagged`);
for (const d of report.drift) console.log(`DRIFT ${d.board} ${d.id}\n  canvas: ${d.canvas}\n  json:   ${d.json}`);
if (process.argv.includes('--verbose')) for (const a of report.ambiguous) console.log(`AMBIGUOUS ${a.board} “${a.words}” ${a.ids.join(' ')}`);
process.exitCode = report.drift.length ? 1 : 0;

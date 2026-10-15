// A before/after picture of the regions a change to spec/machine.json touches (B133,
// machine-json phase 5): one SVG per touched region and side, and an index.html that puts
// each pair side by side. A PR that touches the machine carries it (CI's "The machine's
// change list" job uploads it); read it with tools/machine-diff.py's words.
//
//   node tools/machine-picture.mjs <out dir> [base] [head]   # default: origin/main, the working tree
//   node tools/machine-picture.mjs <out dir> --all ...        # every region, changed or not
//
// base and head are git revisions; "-" (or no head) is the working tree. Each region is
// drawn as its flow: states as boxes (faces folded into their state), its edges as arrows
// (dashed: provisional), and an edge to another region as an arrow to that region. What
// the change adds or changes is green on the after side; what it removes is red on the
// before side; what changed in place is amber on both. Needs elkjs (a root devDependency).
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import ELK from 'elkjs/lib/elk.bundled.js';

const REPO = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const [OUT, BASE = 'origin/main', HEAD = '-'] = process.argv.slice(2).filter((a) => !a.startsWith('--'));
if (!OUT) {
  console.error('usage: node tools/machine-picture.mjs <out dir> [base] [head]');
  process.exit(2);
}
const load = (rev) => JSON.parse(rev === '-'
  ? fs.readFileSync(path.join(REPO, 'spec/machine.json'), 'utf8')
  : execFileSync('git', ['show', `${rev}:spec/machine.json`], { cwd: REPO, encoding: 'utf8', maxBuffer: 1 << 26 }));
const A = load(BASE), B = load(HEAD);

const BG = '#242933', CARD = '#2E3440', LINE = '#4C566A', TEXT = '#D8DEE9', DIM = '#7B88A1';
const ADD = '#A3BE8C', GONE = '#BF616A', CHANGED = '#EBCB8B';
const FONT = '-apple-system, BlinkMacSystemFont, system-ui, sans-serif';
const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const FACE = new Set(['face', 'tray-face']);
const same = (x, y) => JSON.stringify(x) === JSON.stringify(y);
const ek = (e) => `${e.from}→${e.to}`;

// What changed: states, and edges by their ends.
const changedStates = new Set(), addedStates = new Set(), goneStates = new Set();
for (const id of new Set([...Object.keys(A.states), ...Object.keys(B.states)])) {
  const x = A.states[id], y = B.states[id];
  if (!x) addedStates.add(id);
  else if (!y) goneStates.add(id);
  else if (!same(x, y)) changedStates.add(id);
}
const ea = new Map(A.edges.map((e) => [ek(e), e])), eb = new Map(B.edges.map((e) => [ek(e), e]));
const addedEdges = new Set([...eb.keys()].filter((k) => !ea.has(k)));
const goneEdges = new Set([...ea.keys()].filter((k) => !eb.has(k)));
const changedEdges = new Set([...eb.keys()].filter((k) => ea.has(k) && !same(ea.get(k), eb.get(k))));

const regionOf = (id) => (B.states[id] || A.states[id])?.region;
const touched = new Set();
for (const id of [...changedStates, ...addedStates, ...goneStates]) touched.add(regionOf(id));
for (const k of [...addedEdges, ...goneEdges, ...changedEdges]) for (const end of k.split('→')) if (regionOf(end)) touched.add(regionOf(end));
// A changed face shows on its state's box; a face of a face, on the box at the top.
const drawnAs = (M, id) => {
  let at = id;
  while (FACE.has(M.states[at]?.kind) && M.states[at].parent && M.states[M.states[at].parent]) at = M.states[at].parent;
  return at;
};

async function draw(M, side, n) {
  const S = M.states;
  const ids = Object.keys(S).filter((id) => S[id].region === n && drawnAs(M, id) === id);
  const mark = (id) => {
    const all = [id, ...Object.keys(S).filter((f) => drawnAs(M, f) === id && f !== id)];
    if (side === 'after' && all.some((x) => addedStates.has(x))) return id === all[0] && addedStates.has(id) ? ADD : CHANGED;
    if (side === 'before' && all.some((x) => goneStates.has(x))) return id === all[0] && goneStates.has(id) ? GONE : CHANGED;
    return all.some((x) => changedStates.has(x)) ? CHANGED : null;
  };
  const nodes = ids.map((id) => {
    const faces = Object.keys(S).filter((f) => f !== id && drawnAs(M, f) === id).length;
    return { id, width: 220, height: faces ? 64 : 46, labels: [{ text: id }], faces, color: mark(id) };
  });
  const ghosts = new Map();
  const edges = [];
  const seen = new Set();
  for (const e of M.edges) {
    const f = drawnAs(M, e.from), t = drawnAs(M, e.to);
    if (!S[f] || !S[t] || f === t) continue;
    const fin = S[f].region === n, tin = S[t].region === n;
    if (!fin && !tin) continue;
    const from = fin ? f : `Region ${S[f].region}`, to = tin ? t : `Region ${S[t].region}`;
    if (!fin) ghosts.set(from, true);
    if (!tin) ghosts.set(to, true);
    const k = ek(e);
    const color = side === 'after' && addedEdges.has(k) ? ADD : side === 'before' && goneEdges.has(k) ? GONE : changedEdges.has(k) ? CHANGED : null;
    const key = `${from}→${to}`;
    if (seen.has(key) && !color) continue;
    seen.add(key);
    edges.push({ id: `e${edges.length}`, sources: [from], targets: [to], color, dashed: e.status === 'provisional' });
  }
  for (const g of ghosts.keys()) nodes.push({ id: g, width: 120, height: 34, ghost: true });
  const g = await new ELK().layout({
    id: 'root', children: nodes, edges,
    layoutOptions: {
      'elk.algorithm': 'layered', 'elk.direction': 'DOWN', 'elk.edgeRouting': 'ORTHOGONAL',
      'elk.layered.spacing.nodeNodeBetweenLayers': '50', 'elk.spacing.nodeNode': '28',
      'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES',
      'elk.layered.layering.strategy': 'COFFMAN_GRAHAM', 'elk.layered.layering.coffmanGraham.layerBound': '4',
    },
  });
  const P = 24, T = 44, W = Math.round(g.width + P * 2), H = Math.round(g.height + P * 2 + T);
  const o = [];
  for (const e of g.edges) {
    const src = edges.find((x) => x.id === e.id);
    for (const s of e.sections || []) {
      const pts = [s.startPoint, ...(s.bendPoints || []), s.endPoint].map((p) => `${(p.x + P).toFixed(1)},${(p.y + P + T).toFixed(1)}`).join(' ');
      const c = src.color || LINE;
      o.push(`<polyline points="${pts}" fill="none" stroke="${c}" stroke-width="${src.color ? 3 : 1.5}"${src.dashed ? ' stroke-dasharray="6 4"' : ''} marker-end="url(#a${c.slice(1)})"/>`);
    }
  }
  for (const node of g.children) {
    const src = nodes.find((x) => x.id === node.id);
    const x = node.x + P, y = node.y + P + T;
    if (src.ghost) {
      o.push(`<rect x="${x}" y="${y}" width="${node.width}" height="${node.height}" rx="17" fill="none" stroke="${DIM}" stroke-dasharray="4 3"/>`);
      o.push(`<text x="${x + node.width / 2}" y="${y + 22}" fill="${DIM}" font-size="13" text-anchor="middle">${esc(node.id)}</text>`);
      continue;
    }
    const c = src.color;
    o.push(`<rect x="${x}" y="${y}" width="${node.width}" height="${node.height}" rx="8" fill="${CARD}" stroke="${c || LINE}" stroke-width="${c ? 3 : 1}"/>`);
    o.push(`<text x="${x + 12}" y="${y + 28}" fill="${c || TEXT}" font-size="14" font-weight="600">${esc(node.id)}</text>`);
    if (src.faces) o.push(`<text x="${x + 12}" y="${y + 50}" fill="${DIM}" font-size="12">+ ${src.faces} face${src.faces > 1 ? 's' : ''}</text>`);
  }
  const title = `Region ${n} — ${side}`;
  const markers = [LINE, ADD, GONE, CHANGED].map((c) => `<marker id="a${c.slice(1)}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="${c}"/></marker>`).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" font-family="${FONT}"><defs>${markers}</defs><rect width="100%" height="100%" fill="${BG}"/><text x="${P}" y="${P + 14}" fill="${TEXT}" font-size="18" font-weight="700">${esc(title)}</text>${o.join('')}</svg>`;
}

fs.mkdirSync(OUT, { recursive: true });
const regions = process.argv.includes('--all') ? A.regions.map((r) => r.id) : [...touched].filter(Boolean).sort();
const rows = [];
for (const n of regions) {
  for (const [M, side] of [[A, 'before'], [B, 'after']]) fs.writeFileSync(path.join(OUT, `region-${n}.${side}.svg`), await draw(M, side, n));
  rows.push(`<h2>Region ${n}</h2><div class="pair"><img src="region-${n}.before.svg" alt="Region ${n} before"><img src="region-${n}.after.svg" alt="Region ${n} after"></div>`);
}
fs.writeFileSync(path.join(OUT, 'index.html'), `<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>The machine, before and after</title>
<style>body{background:${BG};color:${TEXT};font-family:${FONT};margin:16px}.pair{display:flex;gap:16px;flex-wrap:wrap;align-items:flex-start}img{max-width:100%}
.key span{display:inline-block;margin-right:16px}</style>
<h1>The machine, before and after</h1><p>${esc(BASE)} → ${esc(HEAD === '-' ? 'the working tree' : HEAD)}</p>
<p class="key"><span style="color:${ADD}">■ added</span><span style="color:${GONE}">■ removed</span><span style="color:${CHANGED}">■ changed</span><span>dashed: provisional</span></p>
${rows.join('\n') || '<p>Nothing: spec/machine.json is unchanged.</p>'}`);
console.log(regions.length ? `Regions ${regions.join(', ')}: ${path.join(OUT, 'index.html')}` : 'Nothing changed in the machine.');

// The machine's board on the shroom UI canvas, generated from spec/machine.json (B133,
// machine-json phase 4): one overview of the six regions and the edges between them.
// Bagrat reviews the machine in words, not on region flow boards, so the canvas carries the
// overview only. --regions also writes one flow board per region (each state a node that
// embeds its own screen board live, its faces as chips, the JSON's edges as arrows and its
// rules as labelled bands) and places them under the overview, for an agent's own look.
//
//   node tools/render-machine-canvas.mjs <root> [sha] [--regions]
//
// <root>/project/ holds the canvas as read (canvas.json and the screen boards). The script
// writes Machine.dc.html (and with --regions, MachineRegion1–6.dc.html) there and places
// them in canvas.json as the "The machine" column, beside the others; without --regions it
// takes any region boards out of canvas.json. It needs elkjs (a root devDependency).
// After every merge that touches machine.json, re-run it and publish (docs/DESIGN.md).
import fs from 'node:fs';
import path from 'node:path';
import ELK from 'elkjs/lib/elk.bundled.js';

const REPO = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const [ROOT, SHA = 'HEAD'] = process.argv.slice(2).filter((a) => !a.startsWith('--'));
const REGIONS = process.argv.includes('--regions');
if (!ROOT) {
  console.error('usage: node tools/render-machine-canvas.mjs <root> [sha] [--regions]');
  process.exit(2);
}
const PROJECT = path.join(ROOT, 'project');
const machine = JSON.parse(fs.readFileSync(path.join(REPO, 'spec/machine.json'), 'utf8'));
const canvas = JSON.parse(fs.readFileSync(path.join(PROJECT, 'canvas.json'), 'utf8'));
const S = machine.states;

// ── the look: the canvas's own Nord tokens ──
const BG = '#242933', CARD = '#2E3440', LINE = '#4C566A', TEXT = '#D8DEE9', BRIGHT = '#ECEFF4', DIM = '#7B88A1', LABEL = '#C3CAD6';
const FROST = '#88C0D0', LINK = '#81A1C1', OK = '#A3BE8C', WAIT = '#EBCB8B';
const KIND = { flow: '#88C0D0', fail: '#BF616A', side: '#B48EAD', auto: '#A3BE8C', retry: '#EBCB8B' };
const NODE_W = 340, CAP = 52, SCALE_MAX = 0.5, PAD = 64;
const FONT = '-apple-system, BlinkMacSystemFont, system-ui, sans-serif', MONO = 'ui-monospace, Menlo, monospace';

const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const wrap = (t, n = 22) => {
  const out = [];
  let l = '';
  for (const w of String(t).split(' ')) {
    if ((l + ' ' + w).trim().length > n && l) { out.push(l); l = w; } else l = (l + ' ' + w).trim();
  }
  if (l) out.push(l);
  return out;
};
const boardFile = (name) => `${name}.dc.html`;
const boardSize = (name) => {
  const b = canvas.boards[boardFile(name)];
  return b ? [b.w, b.h] : null;
};

// A face is drawn inside the state it's a face of; a state's drawn node is its nearest
// ancestor that isn't a face.
const FACE_KINDS = new Set(['face', 'tray-face']);
const drawnAs = (id) => {
  let at = id;
  while (S[at] && FACE_KINDS.has(S[at].kind) && S[at].parent && S[S[at].parent]) at = S[at].parent;
  return at;
};
const faces = {};
for (const [id, s] of Object.entries(S)) if (FACE_KINDS.has(s.kind)) (faces[drawnAs(id)] ||= []).push(id);

// The rules each state is in (from or to), a group expanded; `*` rules hold for every state
// and are listed once in the region's bands instead.
const groups = Object.fromEntries(machine.groups.map((g) => [g.id, g.states]));
const expand = (ends) => ends.flatMap((e) => (e.startsWith('@') ? groups[e.slice(1)] : [e]));
const ruleStates = (r) => new Set([...expand(r.from), ...expand(r.to)].filter((x) => S[x]));
const isEveryState = (r) => r.from.includes('*');
// A rule from named states to anywhere holds for those states.
const ruleStatesOf = (r) => (r.to.includes('*') ? new Set(expand(r.from).filter((x) => S[x])) : ruleStates(r));

const regionTitle = Object.fromEntries(machine.regions.map((r) => [r.id, r.title]));

function segs(pts, c, dash, id) {
  const out = [];
  for (let k = 0; k < pts.length - 1; k++) {
    const a = pts[k], b = pts[k + 1], last = k === pts.length - 2, h = last ? 4 : 0;
    const horiz = a.y === b.y, n = Math.max(1, horiz ? Math.abs(b.x - a.x) : Math.abs(b.y - a.y));
    const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y), fwd = horiz ? b.x > a.x : b.y > a.y;
    const d = horiz ? (fwd ? `M 0 4 L ${n - h} 4` : `M ${n} 4 L ${h} 4`) : (fwd ? `M 4 0 L 4 ${n - h}` : `M 4 ${n} L 4 ${h}`);
    const [sw, sh, l, t] = horiz ? [n, 8, x, y - 4] : [8, n, x - 4, y];
    const head = last ? `<defs><marker id="dc-arrow-head-filled-${id}" orient="auto" markerWidth="5" markerHeight="5" refX="2" refY="2" overflow="visible"><path d="M0 0 L4 2 L0 4 Z" fill="${c}" stroke="none" style="stroke: none; fill: ${c}; fill: context-stroke"></path></marker></defs>` : '';
    out.push(`<svg width="${sw}" height="${sh}" viewBox="0 0 ${sw} ${sh}" preserveAspectRatio="none" aria-hidden="true" style="position: absolute; left: ${l}px; top: ${t}px; width: ${sw}px; height: ${sh}px; overflow: visible; fill: none; stroke: ${c}; stroke-width: 2;${dash} stroke-linecap: round; stroke-linejoin: round">${head}<path d="${d}"${last ? ` marker-end="url(#dc-arrow-head-filled-${id})"` : ''}></path></svg>`);
  }
  return out;
}

function page(title, w, h, body) {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${esc(title)}</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<style>
body{margin:0}
a{color:#81a1c1}a:hover{color:#88c0d0}
</style>
</helmet>
<div style="position: relative; width: ${w}px; height: ${h}px; background: ${BG}; font-family: ${FONT}; color: ${TEXT}">
${body.join('\n')}
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{"$preview":{"width":${w},"height":${h}}}'>
class Component extends DCLogic {
renderVals() {
return {};
}
}
</script>
</body>
</html>
`;
}

const chip = (text, color) => `<span style="display: inline-block; margin: 0 6px 6px 0; padding: 2px 8px; border: 1px solid ${color}; border-radius: 10px; font-size: 13px; line-height: 18px; color: ${color}">${esc(text)}</span>`;

// ── one region ──
async function region(n) {
  const ids = Object.keys(S).filter((id) => S[id].region === n && drawnAs(id) === id);
  const inRegion = new Set(ids);
  // The edges touching the region, each end drawn as its node, or as a link to the region
  // it lives in.
  const ghost = (id) => `ghost:${S[drawnAs(id)].region}`;
  const seen = new Map();
  for (const e of machine.edges) {
    const f = drawnAs(e.from), t = drawnAs(e.to);
    if (f === t || (!inRegion.has(f) && !inRegion.has(t))) continue;
    const a = inRegion.has(f) ? f : ghost(f), b = inRegion.has(t) ? t : ghost(t);
    const k = `${a}>${b}`;
    if (!seen.has(k)) seen.set(k, { from: a, to: b, kind: e.kind, label: e.label || e.trigger, status: e.status });
  }
  const edges = [...seen.values()];
  // Each other region is one link, naming the states the edges reach there.
  const ghostStates = {};
  for (const e of machine.edges) {
    const f = drawnAs(e.from), t = drawnAs(e.to);
    if (f === t) continue;
    if (inRegion.has(f) && !inRegion.has(t)) (ghostStates[ghost(t)] ||= new Set()).add(t);
    if (inRegion.has(t) && !inRegion.has(f)) (ghostStates[ghost(f)] ||= new Set()).add(f);
  }
  for (const k of Object.keys(ghostStates)) ghostStates[k] = [...ghostStates[k]].sort();
  const ghostH = (id) => 44 + 18 * ghostStates[id].length;
  const ghosts = [...new Set(edges.flatMap((e) => [e.from, e.to]).filter((x) => x.startsWith('ghost:')))];
  const size = (id) => {
    const b = S[id].board && boardSize(S[id].board);
    const extra = chipsHeight(id);
    if (!b) return [NODE_W, 190 + CAP + extra];
    const sc = Math.min(SCALE_MAX, NODE_W / b[0]);
    return [Math.round(b[0] * sc), Math.round(b[1] * sc) + CAP + extra];
  };
  const chipRows = (list) => (list.length ? Math.ceil(list.join(' ').length / 38) : 0);
  function chipsHeight(id) {
    const f = faces[id] || [];
    return chipRows(f) * 26 + (f.length ? 10 : 0);
  }
  const graph = {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered', 'elk.direction': 'DOWN', 'elk.edgeRouting': 'ORTHOGONAL',
      'elk.layered.spacing.nodeNodeBetweenLayers': '120', 'elk.spacing.nodeNode': '72', 'elk.spacing.edgeEdge': '16', 'elk.spacing.edgeLabel': '6',
      'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES', 'elk.layered.nodePlacement.strategy': 'NETWORK_SIMPLEX',
      // Read on a phone: a few nodes a row, the flow running down.
      'elk.layered.layering.strategy': 'COFFMAN_GRAHAM', 'elk.layered.layering.coffmanGraham.layerBound': '4',
    },
    children: [
      ...ids.map((id) => { const [w, h] = size(id); return { id, width: w, height: h }; }),
      ...ghosts.map((id) => ({ id, width: 280, height: ghostH(id) })),
    ],
    edges: edges.map((e, i) => {
      const lines = wrap(e.label);
      return {
        id: 'e' + i, sources: [e.from], targets: [e.to],
        labels: e.label ? [{ text: e.label, width: Math.min(22, Math.max(...lines.map((l) => l.length))) * 9 + 10, height: lines.length * 21 }] : [],
      };
    }),
  };
  const g = await new ELK().layout(graph);

  // The rules that hold here, as bands above the flow.
  const here = machine.rules.filter((r) => !isEveryState(r) && [...ruleStatesOf(r)].some((s) => inRegion.has(drawnAs(s))));
  const everywhere = machine.rules.filter(isEveryState);
  const BAND = 64, bandsTop = 170;
  const bandH = (here.length + 1) * (BAND + 12);
  const TOP = bandsTop + bandH + 60;
  const W = Math.max(Math.round(g.width + PAD * 2), 1100), H = Math.round(g.height + TOP + PAD);
  const o = [];
  here.forEach((r, i) => {
    const members = [...ruleStatesOf(r)].map(drawnAs).filter((s) => inRegion.has(s)).filter((s, j, a) => a.indexOf(s) === j).join(' · ');
    const c = KIND[r.kind] || FROST, prov = r.status !== 'ratified';
    o.push(`<div style="position: absolute; left: ${PAD}px; top: ${bandsTop + i * (BAND + 12)}px; width: ${W - PAD * 2}px; height: ${BAND}px; box-sizing: border-box; padding: 8px 14px; background: ${CARD}; border: 1px ${prov ? 'dashed' : 'solid'} ${c}; border-radius: 10px; overflow: hidden; font-size: 15px; line-height: 21px"><b style="color: ${c}">${esc(r.label)}</b> <span style="color: ${DIM}">rule ${esc(r.id)}${prov ? ' · provisional' : ''} · ${esc(r.cites.join(', '))}</span><br><span style="font-family: ${MONO}; font-size: 13px; color: ${LABEL}">${esc(members)}</span></div>`);
  });
  o.push(`<div style="position: absolute; left: ${PAD}px; top: ${bandsTop + here.length * (BAND + 12)}px; width: ${W - PAD * 2}px; height: ${BAND}px; box-sizing: border-box; padding: 8px 14px; background: ${CARD}; border: 1px solid ${LINE}; border-radius: 10px; overflow: hidden; font-size: 15px; line-height: 21px"><b style="color: ${BRIGHT}">Everywhere</b> <span style="color: ${DIM}">rules that hold for every state</span><br><span style="font-size: 14px; color: ${LABEL}">${everywhere.map((r) => esc(r.label)).join(' · ')}</span></div>`);
  let mk = 0;
  g.edges.forEach((ge, i) => {
    const e = edges[i], c = KIND[e.kind] || FROST;
    const dash = e.status !== 'ratified' ? ' stroke-dasharray: 6 5;' : '';
    for (const sec of ge.sections || []) {
      o.push(...segs([sec.startPoint, ...(sec.bendPoints || []), sec.endPoint].map((p) => ({ x: Math.round(p.x + PAD), y: Math.round(p.y + TOP) })), c, dash, `${n}-${++mk}`));
    }
  });
  for (const node of g.children) {
    const x = Math.round(node.x + PAD), y = Math.round(node.y + TOP);
    if (node.id.startsWith('ghost:')) {
      const r = node.id.split(':')[1];
      o.push(`<a href="MachineRegion${r}.dc.html" style="position: absolute; left: ${x}px; top: ${y}px; width: ${node.width}px; height: ${node.height}px; box-sizing: border-box; padding: 8px 12px; display: flex; align-items: center; justify-content: center; text-align: center; border: 1.5px dashed #5E81AC; border-radius: 12px; font-size: 15px; line-height: 19px; color: ${LINK}; text-decoration: none"><span>Region ${r} ▸<br><span style="font-family: ${MONO}; font-size: 13px; color: #AEB6C4">${ghostStates[node.id].map(esc).join('<br>')}</span></span></a>`);
      continue;
    }
    const s = S[node.id], rat = s.status === 'ratified', b = s.board && boardSize(s.board) ? s.board : null;
    const extra = chipsHeight(node.id);
    o.push(`<div style="position: absolute; left: ${x}px; top: ${y}px; width: ${node.width}px; height: ${CAP - 6}px; display: flex; align-items: flex-end; font-size: 16px; line-height: 21px"><span><b style="font-family: ${MONO}; color: ${FROST}">${esc(node.id)}</b><br><span style="font-size: 13px; color: ${rat ? OK : WAIT}">${esc(s.status)}</span> <span style="font-size: 13px; color: ${DIM}">${esc(s.kind)} · ${esc(s.surface)}</span>${b ? ` <a href="${boardFile(b)}" style="font-size: 13px; color: ${LINK}">${esc(b)}</a>` : ''}</span></div>`);
    const bodyH = node.height - CAP - extra;
    if (b) {
      const [bw, bh] = boardSize(b), sc = Math.min(SCALE_MAX, NODE_W / bw);
      o.push(`<div style="position: absolute; left: ${x}px; top: ${y + CAP}px; width: ${node.width}px; height: ${bodyH}px; overflow: hidden; border-radius: 8px; outline: 1px solid ${rat ? LINE : WAIT}"><div style="width: ${bw}px; height: ${bh}px; transform: scale(${sc}); transform-origin: 0 0; pointer-events: none"><dc-import name="${esc(b)}" hint-size="${bw}px,${bh}px"></dc-import></div></div>`);
    } else {
      const lines = s.said.slice(0, 5).map((l) => `“${esc(l.text.split('\n')[0])}”`).join('<br>');
      o.push(`<div style="position: absolute; left: ${x}px; top: ${y + CAP}px; width: ${node.width}px; height: ${bodyH}px; box-sizing: border-box; padding: 10px 12px; background: ${CARD}; border: 1px dashed ${rat ? LINE : WAIT}; border-radius: 8px; font-size: 14px; line-height: 19px; overflow: hidden; color: ${TEXT}"><span style="color: ${DIM}">No screen board.</span><br>${lines}</div>`);
    }
    if (extra) {
      const chips = (faces[node.id] || []).map((f) => chip(f, LABEL)).join('');
      o.push(`<div style="position: absolute; left: ${x}px; top: ${y + node.height - extra + 8}px; width: ${node.width}px; font-family: ${MONO}">${chips}</div>`);
    }
  }
  g.edges.forEach((ge, i) => (ge.labels || []).forEach((L) => {
    const prov = edges[i].status !== 'ratified';
    o.push(`<div style="position: absolute; left: ${Math.round(L.x + PAD)}px; top: ${Math.round(L.y + TOP)}px; width: ${Math.round(L.width)}px; font-size: 16px; line-height: 21px; color: ${prov ? WAIT : LABEL}; text-align: center; background: ${BG}">${wrap(L.text).map(esc).join('<br>')}</div>`);
  }));
  const legend = Object.entries(KIND).map(([k, c]) => `<span style="color: ${c}">━ ${k}</span>`).join(' &nbsp; ');
  o.push(`<div style="position: absolute; left: ${PAD}px; top: 64px; font-size: 30px; font-weight: 600; color: ${BRIGHT}">Region ${n} — ${esc(regionTitle[n])}</div>`);
  o.push(`<div style="position: absolute; left: ${PAD}px; top: 110px; width: ${W - PAD * 2}px; font-size: 14px; line-height: 20px; color: ${DIM}">Generated from spec/machine.json at ${esc(SHA)}. Read-only: comment here to change the machine; edit a screen board to change its look. ${legend} &nbsp; <span style="color: ${WAIT}">┄ provisional</span> &nbsp; <a href="Machine.dc.html">▲ the overview</a></div>`);
  if (REGIONS) fs.writeFileSync(path.join(PROJECT, `MachineRegion${n}.dc.html`), page(`The machine, Region ${n}`, W, H, o));
  return { W, H, states: ids.length, edges: edges.length, boards: ids.filter((id) => S[id].board && boardSize(S[id].board)).length };
}

const stats = {};
for (let r = 1; r <= 6; r++) stats[r] = await region(r);

// ── the overview: the six regions, joined where an edge crosses between them ──
const pairs = new Map();
for (const e of machine.edges) {
  const a = S[drawnAs(e.from)].region, b = S[drawnAs(e.to)].region;
  if (a === b) continue;
  const k = `${a}>${b}`;
  const p = pairs.get(k) || { a, b, n: 0, label: e.label || e.trigger };
  p.n += 1;
  pairs.set(k, p);
}
const OW = 440, OH = 150;
const og = await new ELK().layout({
  id: 'o',
  layoutOptions: { 'elk.algorithm': 'layered', 'elk.direction': 'DOWN', 'elk.edgeRouting': 'ORTHOGONAL', 'elk.layered.spacing.nodeNodeBetweenLayers': '110', 'elk.spacing.nodeNode': '90', 'elk.spacing.edgeLabel': '6', 'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES' },
  children: [1, 2, 3, 4, 5, 6].map((r) => ({ id: 'r' + r, width: OW, height: OH })),
  edges: [...pairs.values()].map((p, i) => {
    const text = `${p.n} edge${p.n > 1 ? 's' : ''}`;
    return { id: 'p' + i, sources: ['r' + p.a], targets: ['r' + p.b], labels: [{ text, width: Math.min(22, text.length) * 9 + 10, height: wrap(text).length * 21 }] };
  }),
});
const OP = 64, OT = 200, ow = Math.round(og.width + OP * 2), oh = Math.round(og.height + OT + OP + 120);
const oo = [];
og.edges.forEach((ge, i) => (ge.sections || []).forEach((sec) => oo.push(...segs([sec.startPoint, ...(sec.bendPoints || []), sec.endPoint].map((p) => ({ x: Math.round(p.x + OP), y: Math.round(p.y + OT) })), '#5E81AC', '', 'o' + i))));
for (const node of og.children) {
  const r = +node.id.slice(1), st = stats[r];
  const [open, close] = REGIONS ? [`<a href="MachineRegion${r}.dc.html"`, '</a>'] : ['<div', '</div>'];
  oo.push(`${open} style="position: absolute; left: ${Math.round(node.x + OP)}px; top: ${Math.round(node.y + OT)}px; width: ${OW}px; height: ${OH}px; box-sizing: border-box; padding: 18px 20px; background: ${CARD}; border: 1px solid ${LINE}; border-radius: 12px; text-decoration: none; color: ${TEXT}; display: flex; align-items: flex-start"><span><b style="font-size: 22px; color: ${BRIGHT}">Region ${r}</b> <span style="color: ${FROST}">▸</span><br><span style="font-size: 17px">${esc(regionTitle[r])}</span><br><span style="font-size: 14px; color: ${DIM}">${st.states} states · ${st.edges} arrows · ${st.boards} with a screen board</span></span>${close}`);
}
og.edges.forEach((ge) => (ge.labels || []).forEach((L) => oo.push(`<div style="position: absolute; left: ${Math.round(L.x + OP)}px; top: ${Math.round(L.y + OT)}px; width: ${Math.round(L.width)}px; font-size: 16px; line-height: 21px; color: ${LABEL}; text-align: center; background: ${BG}">${wrap(L.text).map(esc).join('<br>')}</div>`)));
const total = Object.keys(S).length, rules = machine.rules.length;
oo.push(`<div style="position: absolute; left: ${OP}px; top: 64px; font-size: 34px; font-weight: 600; color: ${BRIGHT}">The machine</div>`);
oo.push(`<div style="position: absolute; left: ${OP}px; top: 116px; width: ${ow - OP * 2}px; font-size: 15px; line-height: 21px; color: ${DIM}">${total} states, ${machine.edges.length} edges and ${rules} rules, in six regions and how they hand over.${REGIONS ? ' Tap a region for its flow.' : ''} Generated from spec/machine.json at ${esc(SHA)}.</div>`);
oo.push(`<div style="position: absolute; left: ${OP}px; top: ${oh - OP - 60}px; width: ${ow - OP * 2}px; font-size: 14px; line-height: 20px; color: ${DIM}">To change what shroom does or says, say it in words: a session edits spec/machine.json in a PR and keeps this board and the screens' words in step. To change only how a screen looks, edit its screen board.</div>`);
fs.writeFileSync(path.join(PROJECT, 'Machine.dc.html'), page('The machine', ow, oh, oo));

// ── the index: a "The machine" column right of the others, the overview on top ──
const ours = (k) => k === 'Machine.dc.html' || /^MachineRegion\d\.dc\.html$/.test(k);
for (const k of Object.keys(canvas.boards)) if (ours(k)) delete canvas.boards[k];
canvas.order = canvas.order.filter((k) => !ours(k));
const right = Math.max(...Object.values(canvas.boards).map((b) => b.x + b.w));
const X = right + 400;
let y = 0;
const placed = [['Machine.dc.html', ow, oh, 'The machine — overview (generated)']];
if (REGIONS) for (let r = 1; r <= 6; r++) placed.push([`MachineRegion${r}.dc.html`, stats[r].W, stats[r].H, `The machine — Region ${r}, ${regionTitle[r]} (generated)`]);
for (const [k, w, h, title] of placed) {
  canvas.boards[k] = { x: X, y, w, h, title };
  canvas.order.push(k);
  y += h + 160;
}
canvas.notes = canvas.notes || {};
canvas.notes['t-machine'] = { kind: 'title1', maxW: Math.max(...placed.map((p) => p[1])), text: 'The machine', w: 240, x: X, y: -300 };
fs.writeFileSync(path.join(PROJECT, 'canvas.json'), JSON.stringify(canvas, null, 2) + '\n');
console.log(JSON.stringify({ overview: [ow, oh], regions: REGIONS ? stats : 'not placed (--regions)', x: X }));

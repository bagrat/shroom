// Probe: Region 3's flow board for a Design canvas — nodes embed the real screen boards
// (<dc-import>, scaled), arrows are literal per-segment svgs, everything from spec/*.json.
import fs from 'fs';
import ELK from 'elkjs/lib/elk.bundled.js';
const ROOT = new URL('../../..', import.meta.url).pathname.replace(/\/$/, '');
const OUT = process.argv[2];
const SHA = process.argv[3];
const SCALE_MAX = 0.5, NODE_W = 340;
const map = JSON.parse(fs.readFileSync(`${ROOT}/history/spec/map.json`));
const fx = {};
for (const f of fs.readdirSync(`${ROOT}/history/spec`).filter(f => /^r\d\.json$/.test(f)))
  for (const s of JSON.parse(fs.readFileSync(`${ROOT}/history/spec/${f}`)).states) fx[s.id] = s;
// state -> screen board: a guess for the probe; in the real thing a field of each state in the JSON
const BOARD = { 'all-set': 'AllSet', config: 'Main', countdown: 'Countdown', paused: 'Paused', 'discard-confirm': 'DiscardConfirm',
  'recovered-take': 'SavedTake', 'auto-pause': 'AutoPause', 'quit-confirm': 'QuitConfirm', 'perm-mic': 'WizardPerms', 'perm-screen': 'WizardPerms',
  'perm-priming': 'WizardPerms', 'idle-ready': 'TrayFaces', recording: 'TrayFaces', 'display-lost': 'AutoPause', 'disk-full-stop': 'StoppedForYou', 'recording-off': 'DevicesOff',
  landing: 'Landing', welcome: 'Welcome', idle: 'TrayFaces',
  account: 'WizardAccount', token: 'Wizard', 'token-pick-account': 'WizardPickAccount', card: 'WizardCard', 'name-taken': 'WizardName', wiring: 'WizardWiring',
  'wizard-offline': 'WizardOffline', done: 'WizardPerms', degraded: 'SpaceRepair', 'repair-mood': 'SpaceRepair',
  'owner-page': 'OwnerPage', 'fully-live': 'OwnerPageLive', 'waiting-conn': 'OwnerPagePending', live: 'RecipientLive', 'on-its-way': 'RecipientOnItsWay',
  removed: 'RecipientRemoved', 'stopped-offline': 'StoppedOffline',
  library: 'Library', 'delete-confirm': 'DeleteSheet', 'settings-general': 'SettingsGeneral', 'settings-space': 'SettingsSpace', 'exit-confirm': 'DisconnectSheet',
  'invite-born': 'MembersInvited', 'join-landing': 'JoinLanding', 'join-expired': 'JoinLine', 'join-used': 'JoinLine', 'member-welcome': 'MemberWelcome',
  'join-switch-confirm': 'SwitchConfirm', joining: 'Joining', 'member-all-set': 'AllSet' };
const TITLE = { 1: 'acquisition → first launch', 2: 'the cloud setup wizard', 3: 'the permissions chapter and the tray', 4: 'stop → the link → the recipient', 5: 'the tail: library, settings, updates', 6: 'members and the join machine' };
// port -> [the region on its far side, what it stands for]
const PORT = { r4demo: [4, 'the player page (the demo)'], laterport: [3, 'later life — any publish or self-check'], portinvite: [6, 'invite-born'],
  portlib: [5, 'Library · Settings'], portstop: [4, 'stopping → the link moment'], portcrash: [3, 'a crash mid-take — the next launch'],
  portstopin: [3, "paused's Stop, or recovered-take's Get link"], portdelete: [5, 'Library → Delete…'], portlibin: [3, "config's footer icons"],
  porttomb: [4, 'removed: the tombstone'], portmembersin: [3, 'all-set → Invite someone'], portchapter: [3, 'the permissions chapter'], portconfig6: [3, 'config at the tray'] };
const SIZE = {};
for (const b of new Set(Object.values(BOARD))) {
  const m = fs.readFileSync(`${OUT}/project/${b}.dc.html`, 'utf8').match(/"\$preview":\{"width":(\d+),"height":(\d+)\}/);
  SIZE[b] = [+m[1], +m[2]];
}
const COL = { flow: '#88C0D0', fail: '#BF616A', side: '#B48EAD', auto: '#A3BE8C', retry: '#EBCB8B', port: '#5E81AC' };
const esc = s => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const wrap = (t, n = 22) => { const out = []; let l = ''; for (const w of t.split(' ')) { if ((l + ' ' + w).trim().length > n) { out.push(l); l = w; } else l = (l + ' ' + w).trim(); } if (l) out.push(l); return out; };
let W, H; const PAD = 64, TOP = 180;
async function region3(region) {
const states = map.states.filter(s => s.region === region);
const ports = map.ports.filter(s => s.region === region);
const ids = new Set([...states, ...ports].map(s => s.name));
const edges = map.edges.filter(e => e.region === region && ids.has(e.from) && ids.has(e.to));
const CAP = 52;
const sc = b => Math.min(SCALE_MAX, NODE_W / SIZE[b][0]);
const dims = n => { const b = BOARD[n]; return b ? [Math.round(SIZE[b][0] * sc(b)), Math.round(SIZE[b][1] * sc(b)) + CAP] : [300, 190 + CAP]; };
const graph = {
  id: 'root',
  layoutOptions: { 'elk.algorithm': 'layered', 'elk.direction': 'DOWN', 'elk.edgeRouting': 'ORTHOGONAL',
    'elk.layered.spacing.nodeNodeBetweenLayers': '120', 'elk.spacing.nodeNode': '72', 'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES', 'elk.layered.crossingMinimization.forceNodeModelOrder': 'false', 'elk.spacing.edgeEdge': '16',
    'elk.spacing.edgeLabel': '6', 'elk.layered.nodePlacement.strategy': 'NETWORK_SIMPLEX' },
  children: [...states.map(s => { const [w, h] = dims(s.name); return { id: s.name, width: w, height: h }; }), ...ports.map(p => ({ id: p.name, width: 240, height: 64 }))],
  edges: edges.map((e, i) => { const lines = wrap(e.label); return { id: 'e' + i, sources: [e.from], targets: [e.to],
    labels: e.label ? [{ text: e.label, width: Math.min(22, Math.max(...lines.map(l => l.length))) * 9 + 10, height: lines.length * 21 }] : [] }; }),
};
const g = await new ELK().layout(graph);

W = Math.round(g.width + PAD * 2), H = Math.round(g.height + TOP + PAD);
const o = [];
// background, then arrows, then nodes, then labels (paint order = click order)
let mk = 0;
g.edges.forEach((ge, i) => {
  const e = edges[i], c = COL[e.kind] || '#88C0D0', dash = ['side', 'retry'].includes(e.kind) ? ' stroke-dasharray: 6 5;' : '';
  for (const sec of ge.sections) {
    const pts = [sec.startPoint, ...(sec.bendPoints || []), sec.endPoint].map(p => ({ x: Math.round(p.x + PAD), y: Math.round(p.y + TOP) }));
    for (let k = 0; k < pts.length - 1; k++) {
      const a = pts[k], b = pts[k + 1], last = k === pts.length - 2, h = last ? 4 : 0;
      const horiz = a.y === b.y, n = Math.max(1, horiz ? Math.abs(b.x - a.x) : Math.abs(b.y - a.y));
      const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y), fwd = horiz ? b.x > a.x : b.y > a.y;
      const d = horiz ? (fwd ? `M 0 4 L ${n - h} 4` : `M ${n} 4 L ${h} 4`) : (fwd ? `M 4 0 L 4 ${n - h}` : `M 4 ${n} L 4 ${h}`);
      const [sw, sh, l, t] = horiz ? [n, 8, x, y - 4] : [8, n, x - 4, y];
      const head = last ? `<defs><marker id="dc-arrow-head-filled-${++mk}" orient="auto" markerWidth="5" markerHeight="5" refX="2" refY="2" overflow="visible"><path d="M0 0 L4 2 L0 4 Z" fill="${c}" stroke="none" style="stroke: none; fill: ${c}; fill: context-stroke"></path></marker></defs>` : '';
      o.push(`<svg width="${sw}" height="${sh}" viewBox="0 0 ${sw} ${sh}" preserveAspectRatio="none" aria-hidden="true" style="position: absolute; left: ${l}px; top: ${t}px; width: ${sw}px; height: ${sh}px; overflow: visible; fill: none; stroke: ${c}; stroke-width: 2;${dash} stroke-linecap: round; stroke-linejoin: round">${head}<path d="${d}"${last ? ` marker-end="url(#dc-arrow-head-filled-${mk})"` : ''}></path></svg>`);
    }
  }
});
let missing = 0;
for (const n of g.children) {
  const x = Math.round(n.x + PAD), y = Math.round(n.y + TOP);
  if (ports.some(p => p.name === n.id)) {
    const [pr, pt] = PORT[n.id] || [0, n.id];
    o.push(`<a href="Region${pr}.dc.html" style="position: absolute; left: ${x}px; top: ${y}px; width: ${n.width}px; height: ${n.height}px; box-sizing: border-box; padding: 6px 12px; display: flex; align-items: center; justify-content: center; text-align: center; border: 1.5px dashed #5E81AC; border-radius: 12px; font-size: 15px; line-height: 19px; color: #81A1C1; text-decoration: none"><span>Region ${pr} ▸<br><span style="color: #AEB6C4">${esc(pt)}</span></span></a>`);
    continue;
  }
  const f = fx[n.id], rat = f?.status === 'ratified', b = BOARD[n.id];
  const cap = `<div style="position: absolute; left: ${x}px; top: ${y}px; width: ${n.width}px; height: ${CAP - 6}px; display: flex; align-items: flex-end; font-size: 16px; line-height: 21px"><span><b style="font-family: ui-monospace, Menlo, monospace; color: #88C0D0">${esc(n.id)}</b><br><span style="font-size: 13px; color: ${rat ? '#A3BE8C' : '#EBCB8B'}">${f ? (rat ? 'ratified' : 'provisional') : 'not in the said-lines'}</span>${b ? ` <a href="${b}.dc.html" style="font-size: 13px; color: #81A1C1">${b}</a>` : ''}</span></div>`;
  o.push(cap);
  if (b) {
    const [bw, bh] = SIZE[b];
    o.push(`<div style="position: absolute; left: ${x}px; top: ${y + CAP}px; width: ${n.width}px; height: ${n.height - CAP}px; overflow: hidden; border-radius: 8px; outline: 1px solid ${rat ? '#4C566A' : '#EBCB8B'}"><div style="width: ${bw}px; height: ${bh}px; transform: scale(${sc(b)}); transform-origin: 0 0; pointer-events: none"><dc-import name="${b}" hint-size="${bw}px,${bh}px"></dc-import></div></div>`);
  } else {
    missing++;
    const lines = (f?.said_lines || []).slice(0, 3).map(l => `“${esc(l.text)}”`).join('<br>');
    o.push(`<div style="position: absolute; left: ${x}px; top: ${y + CAP}px; width: ${n.width}px; height: ${n.height - CAP}px; box-sizing: border-box; padding: 10px 12px; background: #2E3440; border: 1px dashed #BF616A; border-radius: 8px; font-size: 14px; line-height: 19px; overflow: hidden; color: #D8DEE9; display: flex; align-items: flex-start"><span><span style="color: #BF616A">No board yet.</span> ${esc(f?.surface || '')}<br>${lines}</span></div>`);
  }
}
g.edges.forEach(ge => (ge.labels || []).forEach(L => {
  o.push(`<div style="position: absolute; left: ${Math.round(L.x + PAD)}px; top: ${Math.round(L.y + TOP)}px; width: ${Math.round(L.width)}px; font-size: 16px; line-height: 21px; color: #C3CAD6; text-align: center; background: #242933">${wrap(L.text).map(esc).join('<br>')}</div>`);
}));
const legend = Object.entries(COL).map(([k, c]) => `<span style="color: ${c}">━ ${k}</span>`).join(' &nbsp; ');
const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>The machine, Region ${region}</title>
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
<div style="position: relative; width: ${W}px; height: ${H}px; background: #242933; font-family: -apple-system, BlinkMacSystemFont, system-ui, sans-serif; color: #D8DEE9">
${o.join('\n')}
<div style="position: absolute; left: ${PAD}px; top: 64px; font-size: 30px; font-weight: 600; color: #ECEFF4">Region ${region} — ${TITLE[region]}</div>
<div style="position: absolute; left: ${PAD}px; top: 110px; font-size: 13px; color: #7B88A1">Generated from spec/ at ${SHA}. Read-only: comment to change the machine; edit a screen board to change its look. ${legend} &nbsp; <span style="color: #EBCB8B">▢ provisional</span></div>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{"$preview":{"width":${W},"height":${H}}}'>
class Component extends DCLogic {
renderVals() {
return {};
}
}
</script>
</body>
</html>
`;
fs.writeFileSync(`${OUT}/project/Region${region}.dc.html`, html);
return { W, H, states: states.length, edges: edges.length, boards: states.filter(s => BOARD[s.name]).length, missing };
}

// ── the overview: six region cards, joined where a port crosses between them ──
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
const stats = {};
for (let r = 1; r <= 6; r++) stats[r] = await region3(r);
const pairs = new Map();
for (const p of map.ports) {
  const [far] = PORT[p.name] || [];
  if (!far || far === p.region) continue;
  for (const e of map.edges.filter(e => e.from === p.name || e.to === p.name)) {
    const [a, b] = e.to === p.name ? [p.region, far] : [far, p.region];
    const k = `${a}>${b}`;
    if (!pairs.has(k)) pairs.set(k, { a, b, label: e.label.split(' — ')[0] });
  }
}
const OW = 440, OH = 150;
const og = await new ELK().layout({ id: 'o', layoutOptions: { 'elk.algorithm': 'layered', 'elk.direction': 'DOWN', 'elk.edgeRouting': 'ORTHOGONAL',
    'elk.layered.spacing.nodeNodeBetweenLayers': '110', 'elk.spacing.nodeNode': '90', 'elk.spacing.edgeLabel': '6', 'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES' },
  children: [1, 2, 3, 4, 5, 6].map(r => ({ id: 'r' + r, width: OW, height: OH })),
  edges: [...pairs.values()].map((p, i) => ({ id: 'p' + i, sources: ['r' + p.a], targets: ['r' + p.b], labels: [{ text: p.label, width: Math.min(22, p.label.length) * 9 + 10, height: wrap(p.label).length * 21 }] })) });
const OP = 64, OT = 200, ow = Math.round(og.width + OP * 2), oh = Math.round(og.height + OT + OP);
const oo = [];
og.edges.forEach((ge, i) => ge.sections.forEach(sec => oo.push(...segs([sec.startPoint, ...(sec.bendPoints || []), sec.endPoint].map(p => ({ x: Math.round(p.x + OP), y: Math.round(p.y + OT) })), '#5E81AC', '', 'o' + i))));
for (const n of og.children) {
  const r = +n.id.slice(1), st = stats[r];
  oo.push(`<a href="Region${r}.dc.html" style="position: absolute; left: ${Math.round(n.x + OP)}px; top: ${Math.round(n.y + OT)}px; width: ${OW}px; height: ${OH}px; box-sizing: border-box; padding: 18px 20px; background: #2E3440; border: 1px solid #4C566A; border-radius: 12px; text-decoration: none; color: #D8DEE9; display: flex; align-items: flex-start"><span><b style="font-size: 22px; color: #ECEFF4">Region ${r}</b> <span style="color: #88C0D0">▸</span><br><span style="font-size: 17px">${esc(TITLE[r])}</span><br><span style="font-size: 14px; color: #7B88A1">${st.states} states · ${st.edges} arrows · ${st.boards} drawn${st.missing ? `, <span style="color: #BF616A">${st.missing} not yet</span>` : ''}</span></span></a>`);
}
og.edges.forEach(ge => (ge.labels || []).forEach(L => oo.push(`<div style="position: absolute; left: ${Math.round(L.x + OP)}px; top: ${Math.round(L.y + OT)}px; width: ${Math.round(L.width)}px; font-size: 16px; line-height: 21px; color: #C3CAD6; text-align: center; background: #242933">${wrap(L.text).map(esc).join('<br>')}</div>`)));
fs.writeFileSync(`${OUT}/project/Machine.dc.html`, `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>The machine</title>
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
<div style="position: relative; width: ${ow}px; height: ${oh}px; background: #242933; font-family: -apple-system, BlinkMacSystemFont, system-ui, sans-serif; color: #D8DEE9">
${oo.join('\n')}
<div style="position: absolute; left: ${OP}px; top: 64px; font-size: 34px; font-weight: 600; color: #ECEFF4">The machine</div>
<div style="position: absolute; left: ${OP}px; top: 116px; width: ${ow - OP * 2}px; font-size: 15px; line-height: 21px; color: #7B88A1">Six regions and how they hand over. Tap a region for its flow. Generated from spec/ at ${SHA}.</div>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{"$preview":{"width":${ow},"height":${oh}}}'>
class Component extends DCLogic {
renderVals() {
return {};
}
}
</script>
</body>
</html>
`);
// the index: the overview, then each region stacked below; the screens in a column to the right
const boards = { 'Machine.dc.html': { x: 0, y: 0, w: ow, h: oh, title: 'The machine — overview (generated)' } };
let y = oh + 200, maxW = ow;
for (let r = 1; r <= 6; r++) { boards[`Region${r}.dc.html`] = { x: 0, y, w: stats[r].W, h: stats[r].H, title: `The machine — Region ${r} (generated)` }; y += stats[r].H + 200; maxW = Math.max(maxW, stats[r].W); }
let yy = 0;
for (const b of Object.keys(SIZE)) { boards[`${b}.dc.html`] = { x: maxW + 600, y: yy, w: SIZE[b][0], h: SIZE[b][1], title: b, ...(SIZE[b][0] >= 1280 ? { expand: 'fill' } : {}) }; yy += SIZE[b][1] + 120; }
const prev = JSON.parse(fs.readFileSync(`${OUT}/project/canvas.json`));
fs.writeFileSync(`${OUT}/project/canvas.json`, JSON.stringify({ ...prev, boards, order: Object.keys(boards) }, null, 2));
console.log('overview', ow, oh, JSON.stringify(Object.fromEntries(Object.entries(stats).map(([r, s]) => [r, [s.W, s.H, s.states, s.edges, s.boards, s.missing]]))));

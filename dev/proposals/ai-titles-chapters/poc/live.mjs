// The POC, live, on the dedicated test account (TESTING-PLAN D3), in one process that
// cleans up after itself: a bucket and the POC Worker (ai + bucket bindings) named
// `shroom-probe-ai`, the sample take's pieces uploaded under rec/<id>/v0/, the route run
// once per way and model, then the Worker, the objects and the bucket deleted.
//
//   SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node live.mjs <take dir>[,<take dir>…] <out.json> [way:model …]
//
// Each take is stored as rec/<its dir's name>/v0/…, and every way:model runs on every take.
//
// The token never leaves the process's environment; the route's gate is made here and
// forgotten when the run ends.

import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { randomBytes } from 'node:crypto';

const A = process.env.SHROOM_E2E_ACCOUNT_ID, T = process.env.SHROOM_E2E_TOKEN;
if (!A || !T) throw new Error('needs SHROOM_E2E_ACCOUNT_ID and SHROOM_E2E_TOKEN');
const [dirsArg, outFile, ...asked] = process.argv.slice(2);
const takes = dirsArg.split(',').map((d) => ({ dir: d, id: d.replace(/\/+$/, '').split('/').pop() }));
const runs = (asked.length ? asked : ['llm:@cf/openai/gpt-oss-20b', 'clef:@cf/openai/gpt-oss-20b']).map((r) => r.split(/:(.*)/s).slice(0, 2));
const NAME = 'shroom-probe-ai';
const pieces = (dir) => readdirSync(dir).filter((f) => f === 'init.mp4' || /^seg_\d+\.m4s$/.test(f));
const api = `https://api.cloudflare.com/client/v4/accounts/${A}`;
const H = { Authorization: `Bearer ${T}` };
const here = new URL('.', import.meta.url);

async function cf(method, path, body, headers = {}, tries = 6) {
  const r = await fetch(api + path, { method, headers: { ...H, ...headers }, body });
  const j = await r.json().catch(() => ({}));
  // A bucket made a moment ago can still answer "recreated during request lifetime".
  if (r.status === 409 && /10073/.test(JSON.stringify(j.errors)) && tries > 0) {
    await new Promise((res) => setTimeout(res, 2000));
    return cf(method, path, body, headers, tries - 1);
  }
  if (!r.ok && !(method === 'DELETE' && r.status === 404)) throw new Error(`${method} ${path}: ${r.status} ${JSON.stringify(j.errors)}`);
  return j;
}

async function up(gate) {
  await cf('POST', '/r2/buckets', JSON.stringify({ name: NAME }), { 'content-type': 'application/json' }).catch((e) => {
    if (!/10004/.test(e.message)) throw e; // already there
  });
  let files = 0;
  for (const { dir, id } of takes) for (const f of pieces(dir)) await cf('PUT', `/r2/buckets/${NAME}/objects/rec/${id}/v0/${f}`, readFileSync(`${dir}/${f}`), { 'content-type': 'application/octet-stream' }), files++;
  const fd = new FormData();
  fd.append('metadata', new Blob([JSON.stringify({ main_module: 'worker.js', compatibility_date: '2026-10-01', bindings: [{ type: 'ai', name: 'AI' }, { type: 'r2_bucket', name: 'BUCKET', bucket_name: NAME }] })], { type: 'application/json' }));
  for (const m of ['worker.js', 'ai.js', 'fmp4.js']) {
    let src = readFileSync(new URL(m, here), 'utf8');
    if (m === 'worker.js') src = src.replace('__GATE__', gate);
    fd.append(m, new Blob([src], { type: 'application/javascript+module' }), m);
  }
  await cf('PUT', `/workers/scripts/${NAME}`, fd);
  await cf('POST', `/workers/scripts/${NAME}/subdomain`, JSON.stringify({ enabled: true }), { 'content-type': 'application/json' });
  const sub = (await cf('GET', '/workers/subdomain')).result.subdomain;
  return { url: `https://${NAME}.${sub}.workers.dev`, files };
}

async function down() {
  await cf('DELETE', `/workers/scripts/${NAME}?force=true`).catch((e) => console.error('worker:', e.message));
  for (const { dir, id } of takes) {
    for (const f of pieces(dir)) await cf('DELETE', `/r2/buckets/${NAME}/objects/rec/${id}/v0/${f}`).catch(() => {});
    for (const f of ['transcript.json', 'title.json', 'chapters.vtt']) await cf('DELETE', `/r2/buckets/${NAME}/objects/rec/${id}/ai/${f}`).catch(() => {});
  }
  await cf('DELETE', `/r2/buckets/${NAME}`).catch((e) => console.error('bucket:', e.message));
}

const gate = randomBytes(16).toString('hex');
const results = [];
try {
  const { url, files } = await up(gate);
  console.log(`up: ${files} files, ${url.replace(/\/\/[^.]+\./, '//…')}`);
  for (const { id } of takes) for (const [way, llm] of runs) {
    let res, body;
    for (let attempt = 0; attempt < 8; attempt++) {
      const t0 = Date.now();
      res = await fetch(`${url}/_shroom/ai/${id}?gate=${gate}&way=${way}&llm=${encodeURIComponent(llm)}`, { method: 'POST', headers: { 'user-agent': 'shroom-ai-poc/0' } });
      const text = await res.text();
      try {
        body = JSON.parse(text);
      } catch {
        body = { error: text };
      }
      body.wallMs = Date.now() - t0;
      if (res.status !== 404 || attempt === 7) break; // a fresh Worker takes a few seconds to answer
      await new Promise((r) => setTimeout(r, 3000));
    }
    console.log(`${id} ${way} ${llm}: ${res.status} ${body.title ?? body.error?.slice(0, 300)}`);
    results.push({ take: id, way, llm, status: res.status, ...body });
  }
} finally {
  await down();
  console.log('down');
  if (outFile) writeFileSync(outFile, JSON.stringify(results, null, 2));
}

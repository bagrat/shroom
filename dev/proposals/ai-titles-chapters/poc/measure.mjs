// Neurons per recorded hour, measured on a long take, live on the dedicated test account
// (TESTING-PLAN D3), in one process that cleans up after itself:
//
//   SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node measure.mjs <take dir> <out.json> [llm …]
//
// 1. A bucket and the POC Worker (`shroom-probe-ai`), the take's pieces under rec/<id>/v0/.
// 2. Each ~60 s window heard in its own request (one Worker invocation each, as the product
//    would while the take uploads), four at a time.
// 3. The take named once per LLM given (the transcript is heard once and reused), captions
//    written from the same words.
// 4. Cloudflare's own meter read back for the run's minutes (a one-hour Account Analytics
//    Read token made here and deleted at once; never written anywhere).
// 5. The Worker, the objects and the bucket deleted.

import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { randomBytes } from 'node:crypto';

const A = process.env.SHROOM_E2E_ACCOUNT_ID, T = process.env.SHROOM_E2E_TOKEN;
if (!A || !T) throw new Error('needs SHROOM_E2E_ACCOUNT_ID and SHROOM_E2E_TOKEN');
const [dir, outFile, ...llmArgs] = process.argv[2] === '--meter' ? ['.', null] : process.argv.slice(2);
const llms = llmArgs.length ? llmArgs : ['@cf/openai/gpt-oss-120b'];
const NAME = 'shroom-probe-ai', TAKE = dir.replace(/\/+$/, '').split('/').pop();
const api = `https://api.cloudflare.com/client/v4/accounts/${A}`;
const H = { Authorization: `Bearer ${T}` };
const here = new URL('.', import.meta.url);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const files = process.argv[2] === '--meter' ? [] : readdirSync(dir).filter((f) => f === 'init.mp4' || /^seg_\d+\.m4s$/.test(f)).sort();

async function cf(method, path, body, headers = {}, tries = 6) {
  const r = await fetch(api + path, { method, headers: { ...H, ...headers }, body });
  const j = await r.json().catch(() => ({}));
  if ((r.status === 409 && /10073/.test(JSON.stringify(j.errors))) || r.status >= 500) {
    if (tries > 0) return await sleep(2000), cf(method, path, body, headers, tries - 1);
  }
  if (!r.ok && !(method === 'DELETE' && r.status === 404)) throw new Error(`${method} ${path}: ${r.status} ${JSON.stringify(j.errors)}`);
  return j;
}

async function pool(items, n, fn) {
  const out = new Array(items.length);
  let i = 0;
  await Promise.all(Array.from({ length: n }, async () => {
    while (i < items.length) {
      const k = i++;
      out[k] = await fn(items[k], k);
    }
  }));
  return out;
}

async function up(gate) {
  await cf('POST', '/r2/buckets', JSON.stringify({ name: NAME }), { 'content-type': 'application/json' }).catch((e) => {
    if (!/10004/.test(e.message)) throw e;
  });
  await pool(files, 8, (f) => cf('PUT', `/r2/buckets/${NAME}/objects/rec/${TAKE}/v0/${f}`, readFileSync(`${dir}/${f}`), { 'content-type': 'application/octet-stream' }));
  const fd = new FormData();
  fd.append('metadata', new Blob([JSON.stringify({ main_module: 'worker.js', compatibility_date: '2026-10-01', bindings: [{ type: 'ai', name: 'AI' }, { type: 'r2_bucket', name: 'BUCKET', bucket_name: NAME }] })], { type: 'application/json' }));
  for (const m of ['worker.js', 'ai.js', 'fmp4.js']) {
    let src = readFileSync(new URL(m, here), 'utf8');
    if (m === 'worker.js') src = src.replace('__GATE__', gate);
    fd.append(m, new Blob([src], { type: 'application/javascript+module' }), m);
  }
  await cf('PUT', `/workers/scripts/${NAME}`, fd);
  await cf('POST', `/workers/scripts/${NAME}/subdomain`, JSON.stringify({ enabled: true }), { 'content-type': 'application/json' });
  return `https://${NAME}.${(await cf('GET', '/workers/subdomain')).result.subdomain}.workers.dev`;
}

async function down() {
  await cf('DELETE', `/workers/scripts/${NAME}?force=true`).catch((e) => console.error('worker:', e.message));
  const known = [...files.map((f) => `rec/${TAKE}/v0/${f}`), ...['transcript.json', 'captions.vtt', 'title.json', 'chapters.vtt'].map((f) => `rec/${TAKE}/ai/${f}`), ...Array.from({ length: Math.ceil((files.length - 1) / 10) }, (_, k) => `rec/${TAKE}/ai/heard/${String(k).padStart(5, '0')}.json`)];
  await pool(known, 8, (k) => cf('DELETE', `/r2/buckets/${NAME}/objects/${k}`).catch(() => {}));
  const left = [];
  let cursor = '';
  do {
    const j = await cf('GET', `/r2/buckets/${NAME}/objects?per_page=1000${cursor ? `&cursor=${cursor}` : ''}`).catch(() => ({ result: [] }));
    left.push(...(j.result || []).map((o) => o.key));
    cursor = j.result_info?.is_truncated ? j.result_info.cursor : '';
  } while (cursor);
  await pool(left, 8, (k) => cf('DELETE', `/r2/buckets/${NAME}/objects/${k.split('/').map(encodeURIComponent).join('/')}`).catch(() => {}));
  await cf('DELETE', `/r2/buckets/${NAME}`).catch((e) => console.error('bucket:', e.message));
}

async function call(url, tries = 8) {
  for (let a = 0; ; a++) {
    const t0 = Date.now();
    const res = await fetch(url, { method: 'POST', headers: { 'user-agent': 'shroom-ai-poc/0' } });
    const text = await res.text();
    let body;
    try {
      body = JSON.parse(text);
    } catch {
      body = { error: text.slice(0, 300) };
    }
    body.status = res.status;
    body.wallMs = Date.now() - t0;
    if ((res.status === 404 || res.status >= 500) && a < tries) {
      await sleep(3000);
      continue;
    }
    return body;
  }
}

/** Cloudflare's meter for [from, to]: neurons and tokens by model, read with a short-lived token. */
async function meter(from, to) {
  const J = { ...H, 'content-type': 'application/json' };
  const groups = (await (await fetch(`${api}/tokens/permission_groups`, { headers: H })).json()).result;
  const id = groups.find((g) => g.name === 'Account Analytics Read').id;
  const m = await (await fetch(`${api}/tokens`, { method: 'POST', headers: J, body: JSON.stringify({ name: 'shroom-poc-ai-analytics', policies: [{ effect: 'allow', resources: { [`com.cloudflare.api.account.${A}`]: '*' }, permission_groups: [{ id }] }], expires_on: new Date(Date.now() + 3600e3).toISOString().replace(/\.\d+Z$/, 'Z') }) })).json();
  if (!m.success) return { error: JSON.stringify(m.errors) };
  try {
    await sleep(8000);
    const q = async (query) => (await (await fetch('https://api.cloudflare.com/client/v4/graphql', { method: 'POST', headers: { Authorization: `Bearer ${m.result.value}`, 'content-type': 'application/json' }, body: JSON.stringify({ query }) })).json());
    const dims = await q(`{ __type(name: "AccountAiInferenceAdaptiveGroupsDimensions") { fields { name } } }`);
    const names = (dims.data?.__type?.fields || []).map((f) => f.name);
    const extra = ['scriptName', 'workerName'].filter((n) => names.includes(n));
    const r = await q(`{ viewer { accounts(filter:{accountTag:"${A}"}) { aiInferenceAdaptiveGroups(limit:100, filter:{datetime_geq:"${from}", datetime_leq:"${to}"}) { count sum { totalNeurons totalInputTokens totalOutputTokens } dimensions { modelId ${extra.join(' ')} } } } } }`);
    return { dimensions: names, rows: r.data?.viewer?.accounts?.[0]?.aiInferenceAdaptiveGroups ?? r.errors };
  } finally {
    await fetch(`${api}/tokens/${m.result.id}`, { method: 'DELETE', headers: H });
  }
}

// `node measure.mjs --meter <from> <to>`: only read the meter for a past run's minutes.
if (process.argv[2] === '--meter') {
  console.log(JSON.stringify(await meter(process.argv[3], process.argv[4]), null, 1));
  process.exit(0);
}

const gate = randomBytes(16).toString('hex');
const report = { take: TAKE, pieces: files.length - 1 };
try {
  const url = await up(gate);
  console.log(`up: ${files.length} files`);
  const from = new Date(Date.now() - 60e3).toISOString().replace(/\.\d+Z$/, 'Z');
  const first = await call(`${url}/_shroom/ai/${TAKE}/hear/0?gate=${gate}`);
  if (first.error) throw new Error(`hear 0: ${first.error}`);
  const rest = await pool(Array.from({ length: first.windows - 1 }, (_, i) => i + 1), 4, (k) => call(`${url}/_shroom/ai/${TAKE}/hear/${k}?gate=${gate}`));
  const heard = [first, ...rest];
  report.hear = { windows: heard.length, failed: heard.filter((h) => h.error).length, audioSecs: heard.reduce((n, h) => n + (h.audioSecs || 0), 0), neurons: heard.reduce((n, h) => n + (h.neurons || 0), 0), msEach: heard.map((h) => h.ms), wallMsEach: heard.map((h) => h.wallMs), errors: heard.filter((h) => h.error).map((h) => h.error.slice(0, 200)) };
  console.log(`heard ${heard.length} windows, ${report.hear.failed} failed, ${report.hear.neurons.toFixed(0)} neurons computed`);
  report.name = [];
  for (const llm of llms) {
    const n = await call(`${url}/_shroom/ai/${TAKE}/name?gate=${gate}&llm=${encodeURIComponent(llm)}`, 0);
    report.name.push({ llm, ...n });
    console.log(`${llm}: ${n.status} ${n.title ?? n.error?.slice(0, 300)}`);
  }
  const to = new Date(Date.now() + 60e3).toISOString().replace(/\.\d+Z$/, 'Z');
  console.log('waiting for the meter…');
  await sleep(120e3);
  report.meter = { from, to, ...(await meter(from, to)) };
} finally {
  await down();
  console.log('down');
  if (outFile) writeFileSync(outFile, JSON.stringify(report, null, 1));
}

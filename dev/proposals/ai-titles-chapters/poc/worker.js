// POC: the route the space's Worker would grow. `POST /_shroom/ai/<id>` reads the take's
// pieces from the bucket (rec/<id>/v0/init.mp4 + seg_NNNNN.m4s), hears them, names the
// video and writes what it found beside the take:
//   rec/<id>/ai/transcript.json  {language, words: [[text, start, end]…]}
//   rec/<id>/ai/title.json       {title, by: "ai", model}
//   rec/<id>/ai/chapters.vtt     WebVTT chapters, for the page's player
// In the product it would run on the pieces as they upload and again after stop; here it
// runs once, on demand. The POC's gate is a path secret (`GATE`); the product's is the
// owner cookie and the Mac's upload pass.

import { audioTrack, pieceAudio, pieceAudioRanged } from './fmp4.js';
import { run, chaptersVtt, captionsVtt, windowPlan, planned, hear, MODELS } from './ai.js';

const GATE = '__GATE__';
const LLMS = new Set(['@cf/openai/gpt-oss-20b', '@cf/zai-org/glm-4.7-flash', '@cf/google/gemma-4-26b-a4b-it', '@cf/qwen/qwen3-30b-a3b-fp8', '@cf/meta/llama-3.2-3b-instruct', '@cf/meta/llama-3.1-8b-instruct-fp8-fast', '@cf/ibm-granite/granite-4.0-h-micro', '@cf/openai/gpt-oss-120b']);

/** The take's sound track and its pieces (keys and sizes, in order), or null. */
async function takePieces(bucket, id) {
  const init = await bucket.get(`rec/${id}/v0/init.mp4`);
  if (!init) return null;
  const track = audioTrack(new Uint8Array(await init.arrayBuffer()));
  const objs = [];
  let cursor;
  do {
    const page = await bucket.list({ prefix: `rec/${id}/v0/seg_`, cursor });
    objs.push(...page.objects);
    cursor = page.truncated ? page.cursor : undefined;
  } while (cursor);
  objs.sort((a, b) => (a.key < b.key ? -1 : 1));
  return { track, objs };
}

/** One piece's audio: a small piece in one read, a big one by ranges (never its video). */
async function readPiece(bucket, o, track) {
  if (o.size <= 512 * 1024) return pieceAudio(new Uint8Array(await (await bucket.get(o.key)).arrayBuffer()), track);
  const read = async (offset, length) => new Uint8Array(await (await bucket.get(o.key, { range: { offset, length: Math.min(length, o.size - offset) } })).arrayBuffer());
  return pieceAudioRanged(read, o.size, track);
}

/** The take's pieces' audio, in order, from the bucket. */
export async function takeAudio(bucket, id) {
  const take = await takePieces(bucket, id);
  if (!take) return null;
  const parts = [];
  for (let i = 0; i < take.objs.length; i += 8) parts.push(...(await Promise.all(take.objs.slice(i, i + 8).map((o) => readPiece(bucket, o, take.track)))));
  return parts;
}

/**
 * Hears window k of the take on its own (one invocation per window, as the product would as
 * the pieces upload) and keeps its words at rec/<id>/ai/heard/<k>.json.
 */
export async function hearWindow(env, id, k, { language } = {}) {
  const take = await takePieces(env.BUCKET, id);
  if (!take) return null;
  const w = windowPlan(take.objs.length)[k];
  if (!w) return { k, missing: true };
  const own = await Promise.all(take.objs.slice(w.from, w.to).map((o) => readPiece(env.BUCKET, o, take.track)));
  const lead = w.lead === null ? null : await readPiece(env.BUCKET, take.objs[w.lead], take.track);
  const win = planned(lead, own, w.last);
  const h = await hear(env.AI, win, { language });
  const end = own[own.length - 1].start + own[own.length - 1].duration;
  const kept = { k, end, language: h.language, audioSecs: h.audioSecs, neurons: h.neurons, ms: h.ms, words: h.words.map((x) => [x.w, +x.s.toFixed(2), +x.e.toFixed(2)]) };
  await env.BUCKET.put(`rec/${id}/ai/heard/${String(k).padStart(5, '0')}.json`, JSON.stringify(kept));
  return { k, windows: windowPlan(take.objs.length).length, words: kept.words.length, audioSecs: h.audioSecs, neurons: h.neurons, ms: h.ms };
}

/** Joins the heard windows into the transcript and captions, then names the take. */
export async function nameHeard(env, id, opts = {}) {
  const keys = [];
  let cursor;
  do {
    const page = await env.BUCKET.list({ prefix: `rec/${id}/ai/heard/`, cursor });
    keys.push(...page.objects.map((o) => o.key));
    cursor = page.truncated ? page.cursor : undefined;
  } while (cursor);
  keys.sort();
  const heard = [];
  for (const k of keys) heard.push(JSON.parse(new TextDecoder().decode(await (await env.BUCKET.get(k)).arrayBuffer())));
  const words = heard.flatMap((h) => h.words.map(([w, s, e]) => ({ w, s, e })));
  const duration = Math.max(0, ...heard.map((h) => h.end));
  const fakeParts = [{ start: 0, duration }];
  const out = await run(env.AI, fakeParts, { ...opts, heardWords: words });
  const captions = captionsVtt(words);
  await env.BUCKET.put(`rec/${id}/ai/transcript.json`, JSON.stringify({ language: heard[0]?.language, words: words.map((w) => [w.w, w.s, w.e]) }));
  await env.BUCKET.put(`rec/${id}/ai/captions.vtt`, captions);
  await env.BUCKET.put(`rec/${id}/ai/title.json`, JSON.stringify({ title: out.title, by: 'ai', model: opts.llm || MODELS.llm }));
  if (out.chapters.length) await env.BUCKET.put(`rec/${id}/ai/chapters.vtt`, chaptersVtt(out.chapters));
  return { ...out, language: heard[0]?.language, windows: heard.length, heardAudioSecs: heard.reduce((n, h) => n + h.audioSecs, 0), heardNeurons: heard.reduce((n, h) => n + h.neurons, 0), captionsBytes: captions.length, captionCues: (captions.match(/ --> /g) || []).length };
}

export async function nameTake(env, id, opts = {}) {
  const t0 = Date.now();
  const parts = await takeAudio(env.BUCKET, id);
  if (!parts) return null;
  const read = Date.now() - t0;
  // Heard once, named many times: a stored transcript is reused unless asked to hear again.
  const stored = opts.hearAgain ? null : await env.BUCKET.get(`rec/${id}/ai/transcript.json`);
  const heardWords = stored ? JSON.parse(new TextDecoder().decode(await stored.arrayBuffer())).words.map(([w, s, e]) => ({ w, s, e })) : undefined;
  const out = await run(env.AI, parts, { ...opts, heardWords });
  const llm = opts.llm || MODELS.llm;
  await env.BUCKET.put(`rec/${id}/ai/transcript.json`, JSON.stringify({ language: out.language, words: out.words.map((w) => [w.w, +w.s.toFixed(2), +w.e.toFixed(2)]) }));
  await env.BUCKET.put(`rec/${id}/ai/title.json`, JSON.stringify({ title: out.title, by: 'ai', model: llm }));
  if (out.chapters.length) await env.BUCKET.put(`rec/${id}/ai/chapters.vtt`, chaptersVtt(out.chapters));
  return { ...out, readMs: read, totalMs: Date.now() - t0 };
}

export default {
  async fetch(req, env) {
    const u = new URL(req.url);
    if (req.method === 'POST' && u.searchParams.get('gate') === GATE) {
      const h = /^\/_shroom\/ai\/([A-Za-z0-9_-]{1,64})\/hear\/(\d{1,5})$/.exec(u.pathname);
      const n = /^\/_shroom\/ai\/([A-Za-z0-9_-]{1,64})\/name$/.exec(u.pathname);
      try {
        if (h) return Response.json((await hearWindow(env, h[1], Number(h[2]), { language: u.searchParams.get('lang') || undefined })) ?? { missing: true });
        if (n) {
          const llm = u.searchParams.get('llm') || MODELS.llm;
          if (!LLMS.has(llm)) return new Response('Unknown model', { status: 400 });
          const out = await nameHeard(env, n[1], { llm, way: u.searchParams.get('way') === 'clef' ? 'clef' : 'llm' });
          return Response.json({ title: out.title, chapters: out.chapters, steps: out.steps, duration: out.duration, language: out.language, windows: out.windows, heardAudioSecs: out.heardAudioSecs, heardNeurons: out.heardNeurons, captionsBytes: out.captionsBytes, captionCues: out.captionCues, words: out.words.length, sentences: out.sents.length });
        }
      } catch (e) {
        return Response.json({ error: String((e && e.stack) || e) }, { status: 500 });
      }
    }
    const m = /^\/_shroom\/ai\/([A-Za-z0-9_-]{1,64})$/.exec(u.pathname);
    if (!m || req.method !== 'POST' || u.searchParams.get('gate') !== GATE) return new Response('Not found', { status: 404 });
    const way = u.searchParams.get('way') === 'clef' ? 'clef' : 'llm';
    const llm = u.searchParams.get('llm') || MODELS.llm;
    if (!LLMS.has(llm)) return new Response('Unknown model', { status: 400 });
    try {
      const out = await nameTake(env, m[1], { way, llm, language: u.searchParams.get('lang') || undefined, hearAgain: u.searchParams.has('hear') });
      if (!out) return new Response('No such take', { status: 404 });
      return Response.json({ title: out.title, chapters: out.chapters, steps: out.steps, duration: out.duration, readMs: out.readMs, totalMs: out.totalMs, language: out.language, chances: out.chances, transcript: out.sents.map((x) => [+x.s.toFixed(1), x.text]) });
    } catch (e) {
      return Response.json({ error: String((e && e.stack) || e) }, { status: 500 });
    }
  },
};

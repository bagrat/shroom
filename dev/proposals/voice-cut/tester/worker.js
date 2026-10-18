// The "shroom cut" tester (dev/proposals/voice-cut.md): a POC Worker on the test account.
// GET /            the page (record or pick a file, see the cuts, play it cut)
// GET /planner.wasm  crates/shroom-edit for the browser (tester/wasm)
// POST /listen     16 kHz mono WAV in, Whisper's words out; needs the throwaway key
// POST /judge      the planner's questions in, Clef's score per candidate out; same key
// Deployed by hand for a test and deleted after it. Nothing is stored.

import page from './page.html';
import planner from './planner.wasm';

const MODEL = '@cf/openai/whisper-large-v3-turbo';
const PROMPT = 'Shroom cut. Um, uh, so, so I, I mean, shroom cut.';
const MOST_BYTES = 16000 * 2 * 300 + 44; // five minutes of 16 kHz mono 16-bit
const JUDGE = '@cf/cloudflare/clef';

/** The judgment above the planner (crates/shroom-edit, judge.rs): for one cue, what was said
 * after it and the stretches before it the redo might replace, each starting further back. */
const judgeState = (q) =>
  'A screen recording, transcribed. The speaker slipped, said the command "shroom cut", and said it again, ' +
  'perhaps in other words, perhaps leaving part of it out. The editor will cut the command and the slip. ' +
  'Each numbered stretch below is what was said just before the command, each starting further back.\n\n' +
  `After the command: «${q.after}»\n\n` +
  q.candidates.map((c, k) => `Stretch ${k + 1}: «${c}»`).join('\n');
const judgeAsk = (k) =>
  `Is everything in stretch ${k + 1} said again after the command, in the same or other words, so that cutting all of ` +
  `stretch ${k + 1} would lose nothing the speaker meant to keep?`;

const b64 = (bytes) => {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  return btoa(s);
};

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === 'GET' && url.pathname === '/') {
      return new Response(page, { headers: { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' } });
    }
    if (req.method === 'GET' && url.pathname === '/planner.wasm') {
      return new Response(planner, { headers: { 'content-type': 'application/wasm' } });
    }
    if (req.method === 'POST' && url.pathname === '/listen') {
      if (req.headers.get('x-key') !== env.KEY) return new Response('That key is not right.', { status: 403 });
      const wav = new Uint8Array(await req.arrayBuffer());
      if (wav.length > MOST_BYTES) return new Response('Five minutes at most, please.', { status: 413 });
      const t0 = Date.now();
      try {
        const out = await env.AI.run(MODEL, { audio: b64(wav), language: 'en', initial_prompt: PROMPT, condition_on_previous_text: false });
        const words = (out.segments || []).flatMap((s) => (s.words || []).map((w) => ({ text: w.word.trim(), start: w.start, end: w.end })));
        return Response.json({ ms: Date.now() - t0, text: out.text, words });
      } catch (e) {
        return Response.json({ error: String(e) }, { status: 502 });
      }
    }
    if (req.method === 'POST' && url.pathname === '/judge') {
      if (req.headers.get('x-key') !== env.KEY) return new Response('That key is not right.', { status: 403 });
      let body;
      try { body = await req.json(); } catch { return new Response('Not JSON.', { status: 400 }); }
      const qs = (Array.isArray(body?.questions) ? body.questions : []).slice(0, 8)
        .filter((q) => q && typeof q.after === 'string' && Array.isArray(q.candidates) && q.candidates.length && q.candidates.length <= 8);
      const t0 = Date.now();
      try {
        const answers = [];
        for (const q of qs) {
          const questions = Object.fromEntries(q.candidates.map((_, k) => [`s${k + 1}`, { type: 'noul', instructions: judgeAsk(k) }]));
          const out = await env.AI.run(JUDGE, { model: 'clef', state: judgeState(q), questions });
          answers.push({ cue: q.cue, scores: q.candidates.map((_, k) => Number(out?.answers?.[`s${k + 1}`]?.noul ?? 0)) });
        }
        return Response.json({ ms: Date.now() - t0, model: JUDGE, answers });
      } catch (e) {
        return Response.json({ error: String(e) }, { status: 502 });
      }
    }
    return new Response('Not here.', { status: 404 });
  },
};

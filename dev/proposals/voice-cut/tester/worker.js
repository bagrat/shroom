// The "shroom cut" tester (dev/proposals/voice-cut.md): a POC Worker on the test account.
// GET /            the page (record or pick a file, see the cuts, play it cut)
// GET /planner.wasm  crates/shroom-edit for the browser (tester/wasm)
// POST /listen     16 kHz mono WAV in, Whisper's words out; needs the throwaway key
// Deployed by hand for a test and deleted after it. Nothing is stored.

import page from './page.html';
import planner from './planner.wasm';

const MODEL = '@cf/openai/whisper-large-v3-turbo';
const PROMPT = 'Shroom cut. Um, uh, so, so I, I mean, shroom cut.';
const MOST_BYTES = 16000 * 2 * 300 + 44; // five minutes of 16 kHz mono 16-bit

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
    return new Response('Not here.', { status: 404 });
  },
};

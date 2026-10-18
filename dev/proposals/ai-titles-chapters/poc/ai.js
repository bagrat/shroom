// A take's pieces → transcript → title and chapters, through a Workers AI binding (`env.AI`).
// The mechanism is exact and testable (windows, the word merge, the chapter rules, the VTT);
// the models only judge: Whisper hears, a small LLM names, a decision model (Clef) may place
// the chapter cuts. Dependency-free, like the space's Worker.

import { joinAudio } from './fmp4.js';

export const MODELS = {
  asr: '@cf/openai/whisper-large-v3-turbo',
  llm: '@cf/openai/gpt-oss-20b',
  decide: '@cf/cloudflare/clef-flash',
};

/** Neurons per unit, from Workers AI's pricing page (read 2026-10-18). $0.011 per 1,000. */
export const NEURONS = {
  '@cf/openai/whisper-large-v3-turbo': { audioMin: 46.63 },
  '@cf/openai/whisper': { audioMin: 41.14 },
  '@cf/deepgram/nova-3': { audioMin: 472.73 },
  '@cf/openai/gpt-oss-20b': { inM: 18182, outM: 27273 },
  '@cf/openai/gpt-oss-120b': { inM: 31818, outM: 68182 },
  '@cf/zai-org/glm-4.7-flash': { inM: 5500, outM: 36400 },
  '@cf/google/gemma-4-26b-a4b-it': { inM: 9091, outM: 27273 },
  '@cf/qwen/qwen3-30b-a3b-fp8': { inM: 4625, outM: 30475 },
  '@cf/meta/llama-3.2-3b-instruct': { inM: 4625, outM: 30475 },
  '@cf/meta/llama-3.1-8b-instruct-fp8-fast': { inM: 4119, outM: 34868 },
  '@cf/ibm-granite/granite-4.0-h-micro': { inM: 1542, outM: 10158 },
  '@cf/cloudflare/clef-flash': { inM: 3455 },
  '@cf/cloudflare/clef': { inM: 21818 },
  '@cf/cloudflare/clef-omni': { inM: 13636 },
};
export const DOLLARS_PER_NEURON = 0.011 / 1000;

export function neurons(model, { audioSecs = 0, inTokens = 0, outTokens = 0 }) {
  const r = NEURONS[model] || {};
  return (r.audioMin || 0) * (audioSecs / 60) + ((r.inM || 0) * inTokens + (r.outM || 0) * outTokens) / 1e6;
}

const b64 = (bytes) => {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  return btoa(s);
};

export const mmss = (t) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, '0')}`;

// ── Hearing ────────────────────────────────────────────────────────────────────────────

/**
 * The pieces' audio as Whisper windows of about `secs`, each after the first led by the
 * last piece of the window before, so a word cut at a window's edge is heard whole once.
 * `cut` is where this window's words begin to count: 1.5 s before its own first piece.
 */
export function windows(parts, secs = 60) {
  const out = [];
  let i = 0;
  while (i < parts.length) {
    let j = i, len = 0;
    while (j < parts.length && (len < secs || j === i)) len += parts[j++].duration;
    const lead = i > 0 ? [parts[i - 1]] : [];
    const own = parts.slice(i, j);
    out.push({ audio: joinAudio([...lead, ...own]), cut: i > 0 ? parts[i].start - 1.5 : -Infinity });
    i = j;
  }
  return out.map((w, k) => ({ ...w, until: k + 1 < out.length ? out[k + 1].cut : Infinity }));
}

/**
 * The windows of a take of `n` pieces, by piece index: window k hears pieces [from, to),
 * led by piece `lead` (the last of the window before; none for the first). `per` pieces of
 * 6 s make a window of about a minute. The plan needs no audio, so each window can be heard
 * in its own invocation as soon as its pieces have uploaded.
 */
export function windowPlan(n, per = 10) {
  const out = [];
  for (let k = 0, from = 0; from < n; k++, from += per) out.push({ k, lead: from > 0 ? from - 1 : null, from, to: Math.min(n, from + per), last: from + per >= n });
  return out;
}

/**
 * One planned window's audio and the span its words count for: from 1.5 s before its own
 * first piece (none for the first window) to 1.5 s before the piece after its last (none
 * for the last), so a word cut at an edge is kept from the window that heard it whole.
 */
export function planned(lead, own, last) {
  const end = own[own.length - 1].start + own[own.length - 1].duration;
  return { audio: joinAudio(lead ? [lead, ...own] : own), cut: lead ? own[0].start - 1.5 : -Infinity, until: last ? Infinity : end - 1.5 };
}

/** Whisper's words for one window, on the take's clock. */
export async function hear(ai, win, { language, prompt } = {}) {
  const t0 = Date.now();
  const out = await ai.run(MODELS.asr, {
    audio: b64(win.audio.adts),
    ...(language ? { language } : {}),
    ...(prompt ? { initial_prompt: prompt } : {}),
    condition_on_previous_text: false,
  });
  const words = [];
  for (const seg of out.segments || []) {
    for (const w of seg.words || []) {
      const text = (w.word ?? w.text ?? '').trim();
      if (text) words.push({ w: text, s: win.audio.start + w.start, e: win.audio.start + w.end });
    }
  }
  const audioSecs = win.audio.end - win.audio.start;
  return {
    words: words.filter((x) => x.s >= win.cut && x.s < win.until),
    language: out.transcription_info?.language,
    ms: Date.now() - t0,
    audioSecs,
    neurons: neurons(MODELS.asr, { audioSecs }),
  };
}

/**
 * Words into sentences: a full stop, a question, or a pause of more than `gap` seconds.
 * Whisper sometimes runs a minute together with commas only; a sentence that reaches
 * `most` seconds ends at its next comma or breath (a gap of 0.25 s), and at 1.7 × `most`
 * it ends at the next word.
 */
export function sentences(words, gap = 1.2, most = 15) {
  const out = [];
  let cur = null;
  for (let i = 0; i < words.length; i++) {
    const w = words[i];
    if (cur && w.s - cur.e > gap) out.push(cur), (cur = null);
    if (!cur) cur = { s: w.s, e: w.e, text: w.w };
    else (cur.e = w.e), (cur.text += ' ' + w.w);
    const len = cur.e - cur.s;
    const long = (len >= most && (/,$/.test(w.w) || (words[i + 1] && words[i + 1].s - w.e >= 0.25))) || len >= most * 1.7;
    if (/[.?!]["')\]]?$/.test(w.w) || long) out.push(cur), (cur = null);
  }
  if (cur) out.push(cur);
  return out;
}

export const transcriptLines = (sents) => sents.map((x) => `[${mmss(x.s)}] ${x.text}`).join('\n');

// ── Naming ─────────────────────────────────────────────────────────────────────────────

const TITLE_RULES = `The title says what the video is about (its subject, or the two or three things it covers), not the occasion, the greeting or who it is for, the way its maker would name it for a teammate: 3 to 8 words, sentence case, plain words from the video, no quotes, no emoji, no full stop at the end, no "video", "recording" or "walkthrough of" filler. Written in the language spoken.`;

const CHAPTER_RULES = `Chapters split the video where the speaker moves to a new topic. The first starts at 0:00. Each lasts at least 20 seconds. Use about one chapter per 1 to 3 minutes of talk in a short video and one per 3 to 5 minutes in a long one, never more than 12, spread over the whole video. Each chapter title is 2 to 6 words, sentence case, no full stop, written in the language spoken. A greeting or wrap-up is its own chapter only when it lasts 20 seconds or more.`;

/** The LLM's JSON, whichever answer shape the model returns. */
function llmText(out) {
  if (typeof out === 'string') return out;
  if (out?.response !== undefined) return typeof out.response === 'string' ? out.response : JSON.stringify(out.response);
  if (out?.choices?.[0]?.message?.content) return out.choices[0].message.content;
  const msg = (out?.output || []).find((o) => o.type === 'message');
  const part = msg?.content?.find((c) => c.type === 'output_text');
  if (part) return part.text;
  throw new Error(`no text in the answer: ${JSON.stringify(out).slice(0, 600)}`);
}
function llmUsage(out) {
  const u = out?.usage || {};
  return { inTokens: u.prompt_tokens ?? u.input_tokens ?? 0, outTokens: u.completion_tokens ?? u.output_tokens ?? 0 };
}
export function parseJson(text) {
  const t = text.replace(/^[\s\S]*?(\{)/, '$1').replace(/\}[^}]*$/, '}');
  return JSON.parse(t);
}

async function llmJson(ai, model, system, user, schema) {
  const t0 = Date.now();
  const out = await ai.run(model, {
    messages: [
      { role: 'system', content: system },
      { role: 'user', content: user },
    ],
    response_format: { type: 'json_schema', json_schema: schema },
    // Room for a thinking model's reasoning before its answer; thinking off where the model allows.
    max_tokens: 4000,
    temperature: 0.2,
    ...(model.includes('gpt-oss') ? { reasoning: { effort: 'low' } } : /qwen3|glm|gemma-4/.test(model) ? { chat_template_kwargs: { enable_thinking: false } } : {}),
  });
  const usage = llmUsage(out);
  const text = llmText(out);
  let json;
  try {
    json = parseJson(text);
  } catch {
    throw new Error(`not JSON: ${text.slice(0, 300)}`);
  }
  if (!json || typeof json !== 'object') throw new Error(`not a JSON object: ${JSON.stringify(out).slice(0, 600)}`);
  return { json, ms: Date.now() - t0, ...usage, neurons: neurons(model, usage) };
}

const SCHEMA = {
  type: 'object',
  properties: {
    title: { type: 'string' },
    chapters: { type: 'array', items: { type: 'object', properties: { start: { type: 'string' }, title: { type: 'string' } }, required: ['start', 'title'] } },
  },
  required: ['title', 'chapters'],
};

/** One call: a title, and chapters when `chapters` (the video is long enough to have them). */
export async function name(ai, sents, { model = MODELS.llm, chapters = true } = {}) {
  const system = `You name screen recordings from their transcript. ${TITLE_RULES}${chapters ? ' ' + CHAPTER_RULES : ' Return no chapters.'} Answer with JSON only: {"title": "...", "chapters": [{"start": "m:ss", "title": "..."}]}.`;
  const user = `Transcript, each line with its start time:\n\n${transcriptLines(sents)}`;
  return llmJson(ai, model, system, user, SCHEMA);
}

// ── The chapter rules (deterministic, over whatever a model proposed) ─────────────────────

export const MIN_CHAPTER = 20;
export const MIN_VIDEO_FOR_CHAPTERS = 90;
export const MAX_CHAPTERS = 12;

const secsOf = (v) => {
  if (typeof v === 'number') return v;
  const s = String(v).trim();
  if (!/^\d+(:\d{1,2}){0,2}(\.\d+)?$/.test(s)) return NaN;
  return s.split(':').reduce((n, p) => n * 60 + Number(p), 0);
};

/** A title made fit to show: one line, no wrapping quotes or full stop, at most 60 chars. */
export function cleanTitle(t) {
  let s = String(t || '').replace(/\s+/g, ' ').trim().replace(/^["'“‘]+|["'”’]+$/g, '').replace(/[.。]+$/, '').trim();
  if (s.length > 60) s = s.slice(0, 60).replace(/\s+\S*$/, '') + '…';
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/**
 * The model's chapters held to the rules: starts snapped to the nearest sentence start, the
 * first at 0:00, none shorter than MIN_CHAPTER (a short one gives its place to the one after
 * it: a 15 s greeting becomes the start of the first topic), the last one too short folds
 * into the one before, at most MAX_CHAPTERS (the shortest fold away first, so the end of a
 * long video keeps its chapters), and none at all for a video under MIN_VIDEO_FOR_CHAPTERS
 * or with fewer than 2.
 */
export function cleanChapters(raw, sents, duration) {
  if (duration < MIN_VIDEO_FOR_CHAPTERS || !Array.isArray(raw)) return [];
  const starts = sents.map((x) => x.s);
  const snap = (t) => starts.reduce((best, s) => (Math.abs(s - t) < Math.abs(best - t) ? s : best), 0);
  let ch = raw
    .map((c) => ({ start: secsOf(c.start), title: cleanTitle(c.title) }))
    .filter((c) => Number.isFinite(c.start) && c.start >= 0 && c.start < duration && c.title)
    .map((c) => ({ ...c, start: snap(c.start) }))
    .sort((a, b) => a.start - b.start);
  if (!ch.length) return [];
  ch[0].start = 0;
  const kept = [];
  for (const c of ch) {
    const prev = kept[kept.length - 1];
    if (prev && c.start - prev.start < MIN_CHAPTER) prev.title = c.title;
    else kept.push({ ...c });
  }
  while (kept.length > 1 && duration - kept[kept.length - 1].start < MIN_CHAPTER) kept.pop();
  // Over 12: the shortest chapter folds into the one before it (the first into the second),
  // until 12 are left, so a long video keeps chapters to its end.
  while (kept.length > MAX_CHAPTERS) {
    const len = (i) => (i + 1 < kept.length ? kept[i + 1].start : duration) - kept[i].start;
    let i = 0;
    for (let j = 1; j < kept.length; j++) if (len(j) < len(i)) i = j;
    kept.splice(i === 0 ? 1 : i, 1);
  }
  const out = kept.map((c, i, a) => ({ start: Math.floor(c.start), end: Math.floor(i + 1 < a.length ? a[i + 1].start : duration), title: c.title }));
  return out.length >= 2 ? out : [];
}

/** WebVTT chapters (kind="chapters"), what a player's track and hls.js read. */
export function chaptersVtt(chapters) {
  const ts = (t) => `${String(Math.floor(t / 3600)).padStart(2, '0')}:${String(Math.floor((t % 3600) / 60)).padStart(2, '0')}:${String(Math.floor(t % 60)).padStart(2, '0')}.000`;
  return 'WEBVTT\n\n' + chapters.map((c, i) => `${i + 1}\n${ts(c.start)} --> ${ts(c.end)}\n${c.title}\n`).join('\n');
}

const vttTime = (t) => {
  const ms = Math.max(0, Math.round(t * 1000));
  const p = (n, w = 2) => String(n).padStart(w, '0');
  return `${p(Math.floor(ms / 3600000))}:${p(Math.floor((ms % 3600000) / 60000))}:${p(Math.floor((ms % 60000) / 1000))}.${p(ms % 1000, 3)}`;
};

/**
 * Captions as WebVTT, from the same words the title and chapters came from: no second
 * hearing. A cue ends at a sentence's end, a pause over 1 s, 6 s, or two lines of 42
 * characters; its lines are broken at the space nearest the middle.
 */
export function captionsVtt(words, { most = 6, chars = 84 } = {}) {
  const cues = [];
  let cur = null;
  for (let i = 0; i < words.length; i++) {
    const w = words[i];
    if (cur && (w.s - cur.e > 1 || w.e - cur.s > most || cur.text.length + 1 + w.w.length > chars)) cues.push(cur), (cur = null);
    if (!cur) cur = { s: w.s, e: w.e, text: w.w };
    else (cur.e = w.e), (cur.text += ' ' + w.w);
    if (/[.?!]["')\]]?$/.test(w.w)) cues.push(cur), (cur = null);
  }
  if (cur) cues.push(cur);
  const lines = (t) => {
    if (t.length <= chars / 2) return t;
    const mid = t.length / 2;
    let at = -1;
    for (let i = 0; i < t.length; i++) if (t[i] === ' ' && (at < 0 || Math.abs(i - mid) < Math.abs(at - mid))) at = i;
    return at < 0 ? t : `${t.slice(0, at)}\n${t.slice(at + 1)}`;
  };
  return 'WEBVTT\n\n' + cues.map((c, i) => `${i + 1}\n${vttTime(c.s)} --> ${vttTime(Math.max(c.e, c.s + 0.5))}\n${lines(c.text)}\n`).join('\n');
}

// ── The System 1 way: a decision model places the cuts, the LLM only names them ───────────

/**
 * Sentences gathered into blocks of at least `secs`, the units a cut may fall between. A
 * pause longer than `pause` always starts a block: speakers breathe between topics.
 */
export function blocks(sents, secs = 8, pause = 0.9) {
  const out = [];
  for (const x of sents) {
    const last = out[out.length - 1];
    if (last && last.e - last.s < secs && x.s - last.e <= pause) (last.e = x.e), (last.text += ' ' + x.text);
    else out.push({ ...x });
  }
  return out;
}

/**
 * Clef's chance that each block opens a new topic: one call, the whole transcript as state,
 * one yes/no question per block edge (≤ 64 a call; longer talks go in overlapping calls).
 */
export async function cutChances(ai, blks, { model = MODELS.decide } = {}) {
  const chances = new Array(blks.length).fill(0);
  let ms = 0, inTokens = 0, calls = 0;
  for (let from = 1; from < blks.length; from += 60) {
    const to = Math.min(blks.length, from + 60);
    const lo = Math.max(0, from - 4), hi = Math.min(blks.length, to + 2);
    const state = blks.slice(lo, hi).map((b, k) => `Block ${lo + k + 1} [${mmss(b.s)}]: ${b.text}`).join('\n\n');
    const questions = {};
    for (let i = from; i < to; i++) {
      questions[`b${i + 1}`] = { type: 'noul', instructions: `Does block ${i + 1} move on to a new topic or task, clearly different from what block ${i} was about?` };
    }
    const t0 = Date.now();
    const out = await ai.run(model, { model: model.split('/').pop(), state, questions });
    ms += Date.now() - t0;
    calls++;
    inTokens += out?.usage?.input_tokens ?? 0;
    for (let i = from; i < to; i++) chances[i] = out.answers?.[`b${i + 1}`]?.noul ?? 0;
  }
  return { chances, ms, calls, inTokens, neurons: neurons(model, { inTokens }) };
}

/** The cuts: chances over `threshold`, strongest first, each MIN_CHAPTER from the others. */
export function pickCuts(blks, chances, duration, threshold = 0.5) {
  const cand = blks
    .map((b, i) => ({ i, s: b.s, p: chances[i] }))
    .filter((c) => c.i > 0 && c.p >= threshold)
    .sort((a, b) => b.p - a.p);
  const cuts = [0];
  for (const c of cand) if (cuts.every((s) => Math.abs(s - c.s) >= MIN_CHAPTER) && duration - c.s >= MIN_CHAPTER) cuts.push(c.s);
  return cuts.sort((a, b) => a - b).slice(0, 12);
}

const NAMES_SCHEMA = { type: 'object', properties: { title: { type: 'string' }, chapters: { type: 'array', items: { type: 'string' } } }, required: ['title', 'chapters'] };

/** The LLM names the video and each given section; it places nothing. */
export async function nameSections(ai, sents, cuts, { model = MODELS.llm } = {}) {
  const parts = cuts.map((c, i) => sents.filter((x) => x.s >= c && (i + 1 >= cuts.length || x.s < cuts[i + 1])));
  const system = `You name screen recordings and their sections from the transcript. ${TITLE_RULES} Each section title is 2 to 6 words, sentence case, no full stop, written in the language spoken. Answer with JSON only: {"title": "...", "chapters": ["one title per section, in order"]}.`;
  const user = parts.map((p, i) => `Section ${i + 1} [${mmss(cuts[i])}]:\n${p.map((x) => x.text).join(' ')}`).join('\n\n');
  return llmJson(ai, model, system, user, NAMES_SCHEMA);
}

// ── The whole run ──────────────────────────────────────────────────────────────────────

/**
 * parts: the take's pieces' audio (fmp4.pieceAudio). way: "llm" (one LLM places and names)
 * or "clef" (Clef places the cuts, the LLM names them). Returns everything the space stores
 * beside the take, plus what it cost.
 */
export async function run(ai, parts, { way = 'llm', llm = MODELS.llm, language, windowSecs = 60, heardWords } = {}) {
  const wins = heardWords ? [] : windows(parts, windowSecs);
  // In the product each window is heard as its pieces upload; here, four at a time.
  const heard = [];
  for (let i = 0; i < wins.length; i += 4) heard.push(...(await Promise.all(wins.slice(i, i + 4).map((w) => hear(ai, w, { language })))));
  const words = heardWords || heard.flatMap((h) => h.words);
  const sents = sentences(words);
  const duration = parts.length ? parts[parts.length - 1].start + parts[parts.length - 1].duration : 0;
  const steps = heardWords ? [] : [{ step: 'hear', model: MODELS.asr, calls: heard.length, ms: Math.max(...heard.map((h) => h.ms)), msEach: heard.map((h) => h.ms), neurons: heard.reduce((n, h) => n + h.neurons, 0) }];
  let title, chapters;
  const wantChapters = duration >= MIN_VIDEO_FOR_CHAPTERS;
  if (way === 'clef' && wantChapters) {
    const blks = blocks(sents);
    const cc = await cutChances(ai, blks);
    steps.push({ step: 'cut', model: MODELS.decide, calls: cc.calls, ms: cc.ms, inTokens: cc.inTokens, neurons: cc.neurons });
    const cuts = pickCuts(blks, cc.chances, duration);
    const n = await nameSections(ai, sents, cuts, { model: llm });
    steps.push({ step: 'name', model: llm, calls: 1, ms: n.ms, inTokens: n.inTokens, outTokens: n.outTokens, neurons: n.neurons });
    title = cleanTitle(n.json.title);
    chapters = cleanChapters(cuts.map((s, i) => ({ start: s, title: n.json.chapters?.[i] ?? '' })), sents, duration);
    return { title, chapters, sents, words, duration, steps, chances: cc.chances.map((p, i) => ({ s: blks[i].s, p })) };
  }
  const n = await name(ai, sents, { model: llm, chapters: wantChapters });
  steps.push({ step: 'name', model: llm, calls: 1, ms: n.ms, inTokens: n.inTokens, outTokens: n.outTokens, neurons: n.neurons });
  title = cleanTitle(n.json.title);
  chapters = cleanChapters(n.json.chapters, sents, duration);
  return { title, chapters, sents, words, duration, steps, language: heard[0]?.language };
}

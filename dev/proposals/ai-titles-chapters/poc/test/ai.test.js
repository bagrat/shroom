import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { windows, sentences, cleanChapters, cleanTitle, chaptersVtt, blocks, pickCuts, run, MODELS, neurons } from '../ai.js';
import { fakeAI, timeline, chat } from './fake-ai.js';

const paragraphs = readFileSync(new URL('../sample/script.txt', import.meta.url), 'utf8').split(/\n\s*\n/).map((p) => p.trim()).filter(Boolean);
const words = timeline(paragraphs);
const end = words[words.length - 1].e + 1;
const parts = Array.from({ length: Math.ceil(end / 6) }, (_, i) => ({ adts: new Uint8Array(8), start: i * 6, duration: 6 }));

test('windows: about a minute each, every one after the first led by a piece of the one before', () => {
  const ws = windows(parts, 60);
  assert.equal(ws[0].audio.start, 0);
  assert.equal(ws[0].cut, -Infinity);
  assert.equal(ws[1].audio.start, 54);
  assert.equal(ws[1].cut, 58.5);
  assert.equal(ws[0].until, 58.5);
  assert.equal(ws.at(-1).until, Infinity);
});

test('every word is heard exactly once, whole, across window edges', async () => {
  const wins = windows(parts, 60);
  const ai = fakeAI({ words, wins, llm: () => chat({ title: 'x', chapters: [] }) });
  const out = await run(ai, parts, { windowSecs: 60 });
  assert.deepEqual(out.words.map((x) => x.w), words.map((x) => x.w));
  assert.ok(out.words.every((x) => !x.w.endsWith('-')), 'no half word from an edge');
  assert.equal(ai.calls.filter((c) => c.model === MODELS.asr).length, wins.length);
});

test('sentences end at a full stop or a pause', () => {
  const s = sentences([
    { w: 'Hi', s: 0, e: 0.3 },
    { w: 'team.', s: 0.4, e: 0.7 },
    { w: 'So', s: 0.8, e: 1 },
    { w: 'first', s: 3, e: 3.3 },
  ]);
  assert.deepEqual(s.map((x) => x.text), ['Hi team.', 'So', 'first']);
});

test('chapters are held to the rules whatever the model says', () => {
  const sents = [0, 31.2, 52, 70.5, 140, 171].map((s) => ({ s, e: s + 5, text: 'x.' }));
  const got = cleanChapters(
    [
      { start: '0:03', title: '"Intro."' },
      { start: '0:30', title: 'Sign up screen' },
      { start: '0:55', title: 'Too close' },
      { start: 140, title: 'The trial numbers' },
      { start: '2:52', title: 'Wrap up' },
      { start: 'soon', title: 'Bad' },
    ],
    sents,
    180,
  );
  assert.deepEqual(got, [
    { start: 0, end: 31, title: 'Intro' },
    { start: 31, end: 52, title: 'Sign up screen' },
    { start: 52, end: 140, title: 'Too close' },
    { start: 140, end: 180, title: 'The trial numbers' },
  ], 'snapped, cleaned, the last 9 s folded into the one before');
  assert.deepEqual(
    cleanChapters([{ start: 0, title: 'Hello' }, { start: 12, title: 'Sign up' }, { start: 70, title: 'Trial' }], sents, 180).map((c) => [c.start, c.title]),
    [[0, 'Sign up'], [70, 'Trial']],
    'a short greeting gives its place to the first topic',
  );
  assert.deepEqual(cleanChapters([{ start: 0, title: 'a' }, { start: 40, title: 'b' }], sents, 80), [], 'a short video has none');
  assert.deepEqual(cleanChapters([{ start: 0, title: 'only' }], sents, 180), [], 'one chapter is none');
});

test('a title fit to show', () => {
  assert.equal(cleanTitle('  “Sign up screen,  invoice bug and trial numbers.” '), 'Sign up screen, invoice bug and trial numbers');
  assert.equal(cleanTitle('setting up and using Shroom'), 'Setting up and using Shroom');
  assert.ok(cleanTitle('word '.repeat(30)).length <= 61);
});

test('chapters as WebVTT', () => {
  assert.equal(chaptersVtt([{ start: 0, end: 75, title: 'One' }, { start: 75, end: 3725, title: 'Two' }]), 'WEBVTT\n\n1\n00:00:00.000 --> 00:01:15.000\nOne\n\n2\n00:01:15.000 --> 01:02:05.000\nTwo\n');
});

test('the LLM way: one call names the video and places the chapters', async () => {
  const wins = windows(parts, 60);
  const ai = fakeAI({
    words,
    wins,
    llm: () => chat({ title: 'Sign up, invoice fix and trial results', chapters: [{ start: '0:00', title: 'What we will cover' }, { start: '0:20', title: 'New sign up screen' }, { start: '1:15', title: 'Invoice total bug' }] }),
  });
  const out = await run(ai, parts);
  assert.equal(out.title, 'Sign up, invoice fix and trial results');
  assert.deepEqual(out.chapters.map((c) => c.title), ['New sign up screen', 'Invoice total bug'], 'the 18 s greeting gave its place');
  assert.equal(out.chapters[0].start, 0);
  const llmCall = ai.calls.find((c) => c.model === MODELS.llm);
  assert.match(llmCall.input.messages[1].content, /^Transcript, each line with its start time:\n\n\[0:00\] Hi team,/);
});

test('the Clef way: one decision call places the cuts, the LLM only names them', async () => {
  const wins = windows(parts, 60);
  const paraStarts = [];
  let i = 0;
  for (const p of paragraphs) paraStarts.push(words[i].s), (i += p.split(/\s+/).filter(Boolean).length);
  const ai = fakeAI({
    words,
    wins,
    clef: (input) => {
      const answers = {};
      for (const id of Object.keys(input.questions)) {
        const n = Number(id.slice(1));
        const at = Number(/Block (\d+) \[(\d+):(\d+)\]/g.exec(input.state.split('\n\n').find((l) => l.startsWith(`Block ${n} `)))?.slice(2).reduce((a, b) => a * 60 + Number(b), 0));
        answers[id] = { type: 'noul', noul: paraStarts.some((s) => Math.abs(s - at) < 1) ? 0.9 : 0.1 };
      }
      return { model: 'clef-flash', answers, usage: { input_tokens: 2000, output_tokens: 0 } };
    },
    llm: (input) => chat({ title: 'Sign up, invoices and the trial', chapters: input.messages[1].content.split('\n\n').map((_, k) => `Part ${k + 1}`) }),
  });
  const out = await run(ai, parts, { way: 'clef' });
  const clefCall = ai.calls.find((c) => c.model === MODELS.decide);
  assert.equal(clefCall.input.model, 'clef-flash');
  assert.ok(Object.keys(clefCall.input.questions).length <= 64);
  assert.deepEqual(out.chapters.map((c) => c.start), paraStarts.filter((s, k) => k === 0 || (s - paraStarts[k - 1] >= 20)).map(Math.floor).slice(0, out.chapters.length));
  assert.ok(out.chapters.length >= 4);
  assert.equal(out.steps.find((s) => s.step === 'cut').neurons, neurons(MODELS.decide, { inTokens: 2000 }));
});

test('pickCuts keeps the stronger of two close edges, and a strong one after a long block', () => {
  const blks = [0, 15, 30, 45, 60, 75].map((s) => ({ s, e: s + 15 }));
  assert.deepEqual(pickCuts(blks, [0, 0.2, 0.8, 0.7, 0.1, 0.9], 120), [0, 30, 75]);
  assert.deepEqual(pickCuts([0, 69, 122, 192].map((s) => ({ s })), [0, 0.87, 0.78, 0.66], 250), [0, 69, 122, 192]);
});

test('a run-on minute is broken at its commas', () => {
  const ws = Array.from({ length: 100 }, (_, i) => ({ w: i % 12 === 11 ? 'and,' : 'word', s: i * 0.4, e: i * 0.4 + 0.35 }));
  const s = sentences(ws);
  assert.ok(s.length >= 2 && s.every((x) => x.e - x.s < 20), s.map((x) => x.e - x.s).join(' '));
});

test('blocks gather sentences to at least 8 s, and a pause starts one', () => {
  const b = blocks([0, 4.5, 9, 13.5, 30].map((s) => ({ s, e: s + 4, text: 't' })));
  assert.deepEqual(b.map((x) => x.s), [0, 9, 30]);
});

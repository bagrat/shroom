// A fake `env.AI`, held to the documented answer shapes: Whisper turbo's segments[].words
// (start/end in seconds from the clip's start), chat-completions JSON for the LLM, and the
// System One answers ({answers: {id: {type: "noul", noul}}, usage}) for Clef.

/** Words on a timeline from paragraphs: `rate` s a word, `gap` s between paragraphs. */
export function timeline(paragraphs, { rate = 0.36, gap = 1.2, from = 0 } = {}) {
  const words = [];
  let t = from;
  for (const p of paragraphs) {
    for (const w of p.split(/\s+/).filter(Boolean)) words.push({ w, s: t, e: (t += rate) - 0.04 });
    t += gap;
  }
  return words;
}

/**
 * `wins`: the windows the code under test will send, in order (Whisper's answer depends on
 * which audio it hears; the fake knows by the call's turn). `llm(input)` and `clef(input)`
 * answer the other two.
 */
export function fakeAI({ words, wins, llm, clef }) {
  const calls = [];
  let turn = 0;
  return {
    calls,
    async run(model, input) {
      calls.push({ model, input });
      if (model.includes('whisper')) {
        const w = wins[turn++];
        const mine = words.filter((x) => x.s >= w.audio.start - 0.2 && x.e <= w.audio.end + 0.2 && x.s < w.audio.end);
        // A word over the clip's edge is cut: Whisper hears half of it, garbled.
        const edge = words.find((x) => x.s < w.audio.end && x.e > w.audio.end);
        const heard = mine.map((x) => ({ word: ' ' + x.w, start: x.s - w.audio.start, end: x.e - w.audio.start }));
        if (edge) heard.push({ word: ' ' + edge.w.slice(0, 2) + '-', start: edge.s - w.audio.start, end: w.audio.end - w.audio.start });
        return { text: heard.map((x) => x.word).join(''), segments: [{ start: 0, end: w.audio.end - w.audio.start, words: heard }], transcription_info: { language: 'en' } };
      }
      if (model.includes('clef')) return clef(input);
      return llm(input);
    },
  };
}

export const chat = (obj, usage = { prompt_tokens: 1000, completion_tokens: 100 }) => ({
  choices: [{ message: { content: JSON.stringify(obj) } }],
  usage,
});

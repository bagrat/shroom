// Names, chapters and captions (spec N66, N67; DECISIONS B205–B209): the space hears each
// minute of a take as it uploads, names it once it's done, places its chapters, writes its
// captions from the same words, waits for the next free day when the account's is used up,
// and follows the saved edit on every view. Whisper and the naming model are faked
// (./ai-take.js); everything else is the Worker as it runs.
import { existsSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import worker, { AI_HEAR_PRIME, AI_MODELS, COPY, aiCaptions, aiChapters, aiCleanChapters, aiKeep, aiPiece, aiRefusal, aiSentences, aiTick, aiTitle, aiToEdited, aiToSource, mp4Tracks } from "../src/worker.js";
import { GONE_MAC, HOST, M1, MHER, MHER_MAC, NOW, OTHER_PARENT_SECRET, OWNER_AIR, OWNER_MAC, REC, SEC, bodyOf, call, cookieFor, goneKey, ledger, metaKey, metaOf, parentProof, space, textOf } from "./space-fixture.js";
import { PIECE_SECS, adtsFrames, cat, chat, fakeAI, initMp4, pieceMp4, responseOf, script, takeObjects } from "./ai-take.js";

const MIN = 60 * SEC;
const API = "/_shroom/api/ai";
const LATER = `.shroom/ai/later/${REC}`;
const TRANSCRIPT = `rec/${REC}/ai/transcript.json`;
const MIDNIGHT = Date.parse("2026-10-03T00:00:00Z"); // the first 00:00 UTC after NOW

/** Sentences of unique words (`s3c`, …), each ending in a full stop: a word heard twice or
 * lost shows by name. Eight words, 3.8 s a sentence; one word straddles each minute's edge. */
const sentences = (n, from = 0) => Array.from({ length: n }, (_, i) => Array.from({ length: 8 }, (_, j) => `s${from + i}${"abcdefgh"[j]}${j === 7 ? "." : ""}`).join(" "));
const SPOKEN = script(sentences(39)); // 0.5 s … 148.2 s: a 25-piece take (150 s)

/** A record for REC, its ledger entry, its pieces and the AI. */
async function aiSpace({ pieces = 25, words = SPOKEN, name = () => chat({ title: "Planning the spring launch", chapters: [{ start: "0:00", title: "Where we are" }, { start: "1:02", title: "the budget" }] }), fail, meta = {}, member = M1, ai = true, take = {} } = {}) {
  const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { durationSec: pieces * PIECE_SECS, ...meta }), ...takeObjects(REC, pieces, take) } });
  await ledger(env, REC, member);
  env.BUCKET.ops.length = 0;
  if (ai) env.AI = fakeAI(words, { name, fail });
  return env;
}
const ask = (env, body, { auth = OWNER_AIR.bearer, now = NOW, headers } = {}) => call(env, "POST", API, { body, auth, now, headers });
const answerOf = async (env, body, opts) => { const res = await ask(env, body, opts); expect(res.status, JSON.stringify(await bodyOf(res))).toBe(200); return bodyOf(res); };
const record = (env) => env.BUCKET.json(metaKey(REC));
const whisperCalls = (env) => env.AI.calls.filter((c) => c.model === AI_MODELS.hear);
const namingCalls = (env) => env.AI.calls.filter((c) => c.model !== AI_MODELS.hear);
const near = (a, b, tol = 0.011) => Math.abs(a - b) <= tol;
/** Asks until the take is named (the Mac's loop: once more while the answer says `more`). */
async function finish(env, opts) {
  for (let i = 0; i < 12; i++) if (!(await answerOf(env, { rec: REC, done: true }, opts)).more) return;
  throw new Error("still more after 12 asks");
}

describe("the route: POST /_shroom/api/ai {rec, through} | {rec, done}", () => {
  it("answers {ai: false} at once when the space has no AI, reading nothing", async () => {
    const env = await aiSpace({ ai: false });
    const res = await ask(env, { rec: REC, through: 10 });
    expect(res.status).toBe(200);
    expect(await bodyOf(res)).toEqual({ shroom: 1, ai: false });
    expect(env.BUCKET.ops).toEqual([]);
  });

  it("refuses a bad body", async () => {
    const env = await aiSpace();
    for (const body of [{}, { rec: "short", through: 10 }, { rec: REC }, { rec: REC, through: -1 }, { rec: REC, through: 1.5 }, { rec: REC, through: "10" }, { rec: REC, done: "yes" }, { rec: REC, done: false }]) {
      const res = await ask(env, body);
      expect(res.status, JSON.stringify(body)).toBe(400);
      expect(await bodyOf(res)).toEqual({ shroom: 1, error: "bad-request" });
    }
    expect(env.AI.calls).toEqual([]);
  });

  it("takes a device key; refuses none, a removed device, a browser's request and anything but POST", async () => {
    const env = await aiSpace();
    expect((await ask(env, { rec: REC, through: 10 }, { auth: null })).status).toBe(401);
    expect((await bodyOf(await ask(env, { rec: REC, through: 10 }, { auth: GONE_MAC.bearer })))).toMatchObject({ error: "removed" });
    expect((await ask(env, { rec: REC, through: 10 }, { headers: { origin: `https://${HOST}` } })).status).toBe(403);
    expect((await call(env, "GET", API, { auth: OWNER_AIR.bearer })).status).toBe(405);
    expect(env.AI.calls).toEqual([]);
  });

  it("is for the take's maker and the owner: a member can't ask for someone else's take", async () => {
    const env = await aiSpace();
    const res = await ask(env, { rec: REC, through: 10 }, { auth: MHER_MAC.bearer });
    expect(res.status).toBe(403);
    expect(await bodyOf(res)).toEqual({ shroom: 1, error: "not-yours" });
    const theirs = await aiSpace({ member: MHER });
    expect(await answerOf(theirs, { rec: REC, through: 10 }, { auth: MHER_MAC.bearer })).toMatchObject({ ai: true, step: "heard" });
    const owners = await aiSpace({ member: MHER });
    expect(await answerOf(owners, { rec: REC, through: 10 })).toMatchObject({ ai: true, step: "heard" });
  });

  it("takes the setup Mac's parent proof, as enroll does (it may hold no device key)", async () => {
    const env = await aiSpace();
    const body = { rec: REC, through: 10 };
    expect(await answerOf(env, body, { auth: parentProof({ path: API, body }) })).toMatchObject({ ai: true, step: "heard" });
    expect((await ask(env, body, { auth: parentProof({ path: API, body, secret: OTHER_PARENT_SECRET }) })).status).toBe(401);
    expect((await ask(env, body, { auth: parentProof({ path: API, body: { rec: REC, through: 20 } }) })).status).toBe(401);
  });

  it("knows no take that isn't the space's, and none that was deleted", async () => {
    const env = await aiSpace();
    expect(await bodyOf(await ask(env, { rec: "NoSuchRecording0", through: 10 }))).toEqual({ shroom: 1, error: "absent" });
    await env.BUCKET.put(goneKey(REC), "{}");
    const res = await ask(env, { rec: REC, through: 10 });
    expect(res.status).toBe(410);
    expect(env.AI.calls).toEqual([]);
  });
});

describe("the ping (B206 a)", () => {
  it("says the page carries the AI, so the setup Mac knows naming is on; without it the ping is as it was", async () => {
    const on = await aiSpace();
    expect(await bodyOf(await call(on, "GET", "/_shroom/ping"))).toMatchObject({ shroom: true, ai: true });
    expect(on.BUCKET.ops).toEqual([]);
    const off = await aiSpace({ ai: false });
    expect(Object.keys(await bodyOf(await call(off, "GET", "/_shroom/ping")))).toEqual(["shroom", "iface", "version"]);
  });
});

describe("hearing (B205 a, b)", () => {
  // A titled take, so no early title (B205 f) steps in: these are about hearing.
  const TYPED = { title: "Mine", titled: true };

  it("tells Whisper to expect the cue and hesitations, with its voice-activity filter on, and keeps no word without a letter", async () => {
    // Measured 2026-10-18 (dev/proposals/ai-titles-chapters/poc/results/prime-2026-10-18.json):
    // unfiltered, Whisper wrote "Thank you." into silent and music minutes and ". ." into a
    // room's noise; filtered, nothing, and speech kept its words and times.
    const words = script(["s0a s0b . s0c.", "s1a … s1b s1c."]);
    const env = await aiSpace({ meta: TYPED, words });
    await answerOf(env, { rec: REC, through: 10 });
    const [{ input }] = whisperCalls(env);
    expect(input).toMatchObject({ vad_filter: true, initial_prompt: AI_HEAR_PRIME });
    expect(AI_HEAR_PRIME).toMatch(/^Shroom cut\./);
    expect(env.BUCKET.json(TRANSCRIPT).words.map((w) => w[0])).toEqual(["s0a", "s0b", "s0c.", "s1a", "s1b", "s1c."]);
  });

  it("hears each full minute as it's asked, then the rest once the take is done: every word once, on the recording's clock", async () => {
    const env = await aiSpace({ meta: TYPED });
    expect(await answerOf(env, { rec: REC, through: 9 })).toEqual({ shroom: 1, ai: true, step: "idle", more: false });
    expect(whisperCalls(env)).toHaveLength(0);
    expect(await answerOf(env, { rec: REC, through: 10 })).toEqual({ shroom: 1, ai: true, step: "heard", more: false });
    let t = env.BUCKET.json(TRANSCRIPT);
    expect(t).toMatchObject({ v: 1, model: AI_MODELS.hear, language: "en", windows: 1, done: false });
    expect(t.duration).toBeCloseTo(60, 6);
    expect(t.heard_through).toBeCloseTo(58.5, 6);
    // Final up to 1.5 s before the window's end; past that, held until the next window hears it whole.
    expect(t.words.map((w) => w[0])).toEqual(SPOKEN.filter((w) => w.s < 58.5).map((w) => w.w));
    expect(await answerOf(env, { rec: REC, through: 20 })).toMatchObject({ step: "heard" });
    expect(await answerOf(env, { rec: REC, through: 25 })).toMatchObject({ step: "idle" }); // five pieces: not a full window
    expect(await answerOf(env, { rec: REC, through: 25, done: true })).toEqual({ shroom: 1, ai: true, step: "heard", more: true });
    t = env.BUCKET.json(TRANSCRIPT);
    expect(t).toMatchObject({ windows: 3, done: true });
    expect(t.duration).toBeCloseTo(150, 6);
    expect(t.heard_through).toBeCloseTo(150, 6);
    expect(t.tail).toBeUndefined();
    expect(t.words.map((w) => w[0])).toEqual(SPOKEN.map((w) => w.w));
    for (const [i, [, s, e]] of t.words.entries()) {
      expect(near(s, SPOKEN[i].s), `${SPOKEN[i].w} starts at ${s}`).toBe(true);
      expect(near(e, SPOKEN[i].e), `${SPOKEN[i].w} ends at ${e}`).toBe(true);
    }
  });

  it("leads each window after the first with the last piece of the one before, and sends sound only, as a stream", async () => {
    const env = await aiSpace({ meta: TYPED });
    await answerOf(env, { rec: REC, through: 10 });
    await answerOf(env, { rec: REC, through: 20 });
    await answerOf(env, { rec: REC, done: true });
    const heard = whisperCalls(env).map((c) => c.heard);
    expect(heard.map((h) => [h.from, h.to].map((x) => +x.toFixed(3)))).toEqual([[0, 60], [54, 120], [114, 150]]);
    for (const h of heard) for (const f of h.frames) expect(f.payload[0], "a picture byte reached Whisper").toBe(0xaa);
    for (const c of whisperCalls(env)) expect(c.input.audio.contentType).toBe("audio/aac");
  });

  it("reads a piece's sound without its picture: ranged reads, a fraction of its bytes", async () => {
    const env = await aiSpace({ pieces: 10 });
    const pieceBytes = 10 * env.BUCKET.store.get(`rec/${REC}/v0/seg_00000.m4s`).length;
    await answerOf(env, { rec: REC, through: 10 });
    const reads = env.BUCKET.ops.filter((o) => o.op === "get" && o.key.includes("/v0/seg_"));
    for (const r of reads) expect(r.options?.range, `${r.key} was read whole`).toBeTruthy();
    const bytes = reads.reduce((n, r) => { const m = r.options.range.get("range").match(/bytes=(\d+)-(\d+)/); return n + Number(m[2]) - Number(m[1]) + 1; }, 0);
    expect(bytes / pieceBytes).toBeLessThan(1 / 3);
  });

  it("keeps the words of a take that ends on a window's edge", async () => {
    const words = script(sentences(31), { from: 2.4 }); // to 119.55 s: the last two words past 118.5 s
    const env = await aiSpace({ pieces: 20, words, meta: TYPED });
    await answerOf(env, { rec: REC, through: 10 });
    await answerOf(env, { rec: REC, through: 20 });
    // Nothing left to hear: the held words become final and the take is named in the same ask.
    expect(await answerOf(env, { rec: REC, through: 20, done: true })).toEqual({ shroom: 1, ai: true, step: "named", more: false });
    expect(env.BUCKET.json(TRANSCRIPT).words.map((w) => w[0])).toEqual(words.map((w) => w.w));
    expect(whisperCalls(env)).toHaveLength(2);
  });

  it("a take with no sound is heard as silence: no call, no words, no name", async () => {
    const env = await aiSpace({ pieces: 12, take: { sound: false } });
    await finish(env);
    expect(whisperCalls(env)).toHaveLength(0);
    expect(namingCalls(env)).toHaveLength(0);
    const t = env.BUCKET.json(TRANSCRIPT);
    expect(t).toMatchObject({ done: true, words: [], windows: 2 });
    expect(t.duration).toBeCloseTo(72, 6);
    const m = record(env);
    expect(m.title).toBe("Untitled — Oct 2");
    expect(m.named).toMatchObject({ by: "ai", title: null, chapters: [] });
    expect(m.captions ?? null).toBeNull();
  });

  it("reads both ways a fragment can say where its sound is", async () => {
    const tracks = mp4Tracks(initMp4());
    expect(tracks.sound).toMatchObject({ id: 2, timescale: 48000, objectType: 2, freqIndex: 3, channels: 1 });
    expect(tracks.picture).toMatchObject({ id: 1 });
    const one = (bytes) => ({ BUCKET: { get: async (_k, o) => { const m = o.range.get("range").match(/bytes=(\d+)-(\d+)/); const s = bytes.slice(Number(m[1]), Number(m[2]) + 1); return { body: new Blob([s]).stream() }; } } });
    const a = pieceMp4(3), b = pieceMp4(3, { legacy: true });
    const x = await aiPiece(one(a), "k", a.length, tracks), y = await aiPiece(one(b), "k", b.length, tracks);
    expect(x.secs).toBeCloseTo(6, 9);
    expect(y.secs).toBeCloseTo(6, 9);
    expect([...cat(y.parts)]).toEqual([...cat(x.parts)]);
    expect(x.bytes).toBe(cat(x.parts).length);
    const frames = adtsFrames(cat(x.parts));
    expect(frames).toHaveLength(288);
    expect(frames.map((f) => (f.payload[2] << 8) | f.payload[3])).toEqual([...Array(288).keys()]);
    // ADTS: AAC-LC, 48 kHz, mono, each frame's own length.
    expect([...frames[0].header.slice(0, 4)]).toEqual([0xff, 0xf1, 0x4c, 0x40]);
  });

  let ffmpeg = true, x264 = false;
  try { x264 = /\blibx264\b/.test(execFileSync("ffmpeg", ["-hide_banner", "-encoders"], { encoding: "utf8" })); } catch { ffmpeg = false; }
  it.skipIf(!ffmpeg)("reads a take the way shroom's encoder writes it (ffmpeg, HLS fMP4, AAC mono)", async () => {
    const dir = mkdtempSync(join(tmpdir(), "shroom-ai-"));
    // crates/shroom-encode/src/args.rs's arguments, a test picture and a tone in place of the screen and the microphone. The
    // picture's codec is beside the point (only its track is): H.264 where this ffmpeg has x264, else ffmpeg's own MPEG-4, as
    // the LGPL build CI tests with has no x264 and a hosted Mac's VideoToolbox may not encode.
    const picture = x264 ? ["-c:v", "libx264", "-preset", "ultrafast"] : ["-c:v", "mpeg4", "-q:v", "5"];
    execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=30", "-f", "lavfi", "-i", "sine=frequency=330:sample_rate=48000", "-t", "20",
      ...picture, "-pix_fmt", "yuv420p", "-g", "60", "-c:a", "aac", "-b:a", "128k", "-ac", "1",
      "-f", "hls", "-hls_segment_type", "fmp4", "-hls_time", "6", "-hls_playlist_type", "vod", "-hls_fmp4_init_filename", "init.mp4",
      "-hls_segment_filename", join(dir, "seg_%05d.m4s"), join(dir, "stream.m3u8")]);
    const tracks = mp4Tracks(new Uint8Array(readFileSync(join(dir, "init.mp4"))));
    expect(tracks.sound).toMatchObject({ objectType: 2, channels: 1, timescale: 48000 });
    let secs = 0, read = 0, size = 0;
    const parts = [];
    for (const f of readdirSync(dir).filter((x) => x.endsWith(".m4s")).sort()) {
      const bytes = new Uint8Array(readFileSync(join(dir, f)));
      size += bytes.length;
      const env = { BUCKET: { get: async (_k, o) => { const m = o.range.get("range").match(/bytes=(\d+)-(\d+)/); const s = bytes.slice(Number(m[1]), Number(m[2]) + 1); read += s.length; return { body: new Blob([s]).stream() }; } } };
      const p = await aiPiece(env, f, bytes.length, tracks);
      secs += p.secs;
      parts.push(...p.parts);
    }
    expect(Math.abs(secs - 20)).toBeLessThan(0.1);
    expect(read / size).toBeLessThan(0.5);
    // The sound as Whisper gets it: AAC frames of 1,024 samples, twenty seconds of them, that ffmpeg decodes without a complaint.
    const all = cat(parts);
    const frames = adtsFrames(all);
    for (const f of frames) expect(f.payload.length).toBeGreaterThan(0);
    expect(Math.abs((frames.length * 1024) / 48000 - 20)).toBeLessThan(0.1);
    const file = join(dir, "all.aac");
    writeFileSync(file, all);
    expect(execFileSync("ffmpeg", ["-v", "error", "-i", file, "-f", "null", "-"], { stdio: ["ignore", "pipe", "pipe"] }).toString()).toBe("");
  });
});

describe("naming (B205 c, d; B206 d)", () => {
  it("names a done take in one call: the title while untitled, chapters on sentence starts, captions from the same words", async () => {
    const env = await aiSpace();
    await finish(env);
    const calls = namingCalls(env);
    expect(calls).toHaveLength(1);
    expect(calls[0].model).toBe(AI_MODELS.name);
    expect(calls[0].input.response_format.type).toBe("json_schema");
    expect(calls[0].input.reasoning).toEqual({ effort: "low" });
    const user = calls[0].input.messages.find((m) => m.role === "user").content;
    expect(user).toContain("[0:00] s0a s0b s0c s0d s0e s0f s0g s0h.");
    expect(user).toContain("[2:24] s38a");
    const m = record(env);
    expect(m.title).toBe("Planning the spring launch");
    expect(m.titled ?? false).toBe(false);
    // 1:02 snaps to the nearest sentence start (s16a at 61.3 s); its title gets a capital.
    expect(m.named).toMatchObject({ by: "ai", model: AI_MODELS.name, title: "Planning the spring launch", words: SPOKEN.length, chapters: [[0, "Where we are"], [61.3, "The budget"]] });
    expect(m.named.at).toBe(new Date(NOW).toISOString());
    expect(m.captions).toEqual({ language: "en" });
    expect(m.naming).toBeUndefined();
    expect(env.BUCKET.json(LATER)).toBeNull();
  });

  it("never touches a typed title, but still places the chapters and writes the captions", async () => {
    const env = await aiSpace({ meta: { title: "Q3 numbers for Ani", titled: true } });
    await finish(env);
    const m = record(env);
    expect(m.title).toBe("Q3 numbers for Ani");
    expect(m.titled).toBe(true);
    expect(m.named.chapters).toHaveLength(2);
    expect(m.captions).toEqual({ language: "en" });
  });

  it("a title typed while the name was on its way wins", async () => {
    const env = await aiSpace();
    await answerOf(env, { rec: REC, through: 10 });
    expect(await answerOf(env, { rec: REC, through: 20 })).toMatchObject({ step: "titled" }); // the early title (B205 f)
    await answerOf(env, { rec: REC, through: 20 });
    await answerOf(env, { rec: REC, done: true });
    let raced = false;
    env.BUCKET.hooks.beforePut = async (key) => {
      if (raced || key !== metaKey(REC)) return;
      raced = true;
      const m = record(env);
      env.BUCKET.touch(metaKey(REC), JSON.stringify({ ...m, title: "Typed meanwhile", titled: true }));
    };
    await answerOf(env, { rec: REC, done: true });
    expect(raced).toBe(true);
    expect(record(env)).toMatchObject({ title: "Typed meanwhile", titled: true, named: { title: "Planning the spring launch" } });
  });

  it("falls back to the second model when the first fails", async () => {
    const env = await aiSpace({
      fail: (model) => { if (model === AI_MODELS.name) throw new Error("5007: No such model"); },
      name: (_input, model) => responseOf({ title: "the spring launch, planned.", chapters: [] }),
    });
    await finish(env);
    expect(namingCalls(env).map((c) => c.model)).toEqual([AI_MODELS.name, AI_MODELS.fallback]);
    expect(namingCalls(env)[1].input.chat_template_kwargs).toEqual({ enable_thinking: false });
    expect(record(env)).toMatchObject({ title: "The spring launch, planned", named: { model: AI_MODELS.fallback, chapters: [] } });
  });

  it("asks for no chapters on a video under 90 s, and keeps none it's given", async () => {
    const env = await aiSpace({ pieces: 12, words: script(sentences(18)) });
    await finish(env);
    expect(namingCalls(env)[0].input.messages[0].content).toContain("Return no chapters.");
    expect(record(env).named.chapters).toEqual([]);
  });

  it("names nothing from fewer than twelve words", async () => {
    const env = await aiSpace({ pieces: 3, words: script(["Hi there, a quick one."]) });
    await finish(env);
    expect(namingCalls(env)).toHaveLength(0);
    expect(record(env)).toMatchObject({ title: "Untitled — Oct 2", named: { by: "ai", title: null, words: 5 }, captions: { language: "en" } });
  });

  it("holds the model's chapters to the rules, whatever it says", () => {
    const sents = [0, 14, 31, 47, 62, 80, 95, 130, 170, 200].map((s) => ({ s }));
    // Snapped to sentence starts, the first at 0:00, a short one giving its place to the next.
    expect(aiCleanChapters([{ start: "0:03", title: "hello" }, { start: "0:13", title: "the plan" }, { start: "1:01", title: "Costs" }, { start: "3:18", title: "Wrap-up" }], sents, 240))
      .toEqual([[0, "The plan"], [62, "Costs"], [200, "Wrap-up"]]);
    // A last chapter under 20 s folds into the one before; under 90 s or one chapter, none.
    expect(aiCleanChapters([{ start: "0:00", title: "A" }, { start: "1:20", title: "B" }, { start: "3:20", title: "C" }], sents, 215)).toEqual([[0, "A"], [80, "B"]]);
    expect(aiCleanChapters([{ start: "0:00", title: "A" }, { start: "0:40", title: "B" }], sents, 89)).toEqual([]);
    expect(aiCleanChapters([{ start: "0:00", title: "A" }], sents, 240)).toEqual([]);
    expect(aiCleanChapters("nonsense", sents, 240)).toEqual([]);
    // At most twelve: the shortest fold into the one before.
    const many = Array.from({ length: 20 }, (_, i) => ({ s: i * 60 }));
    const twenty = aiCleanChapters(many.map((x, i) => ({ start: String(x.s), title: `Part ${i}` })), many, 1200);
    expect(twenty).toHaveLength(12);
    expect(twenty[0]).toEqual([0, "Part 0"]);
  });

  it("cleans a title to one line, at most 60 characters, no quotes or full stop, a capital first", () => {
    expect(aiTitle('  "planning   the\nlaunch."  ')).toBe("Planning the launch");
    expect(aiTitle("“Quarterly review”")).toBe("Quarterly review");
    const long = aiTitle("a very long title that goes on and on about everything the video covers in detail");
    expect(long.length).toBeLessThanOrEqual(61);
    expect(long).toMatch(/^A very long title .*[a-z]…$/);
  });

  it("splits the words into sentences: a full stop, a pause, or a run-on past fifteen seconds", () => {
    const words = [["One", 0, 0.3], ["two.", 0.4, 0.7], ["Three", 2.2, 2.5], ["four", 2.6, 2.9]];
    expect(aiSentences(words).map((x) => [x.s, x.text])).toEqual([[0, "One two."], [2.2, "Three four"]]);
    const runOn = Array.from({ length: 60 }, (_, i) => [`w${i}${i % 7 === 6 ? "," : ""}`, i * 0.4, i * 0.4 + 0.35]);
    for (const x of aiSentences(runOn)) expect(x.e - x.s).toBeLessThan(25);
  });
});

describe("the early title (B205 f)", () => {
  // Bagrat, 2026-10-18: the page opens named. Once the first minute is heard while the take
  // records, a title from it alone; the full name at done replaces it, never a typed title.
  const EARLY = "Where the launch stands";
  const FULL = "Planning the spring launch";
  /** A title-only ask (the early one) gets EARLY; the full one, FULL and its chapters. */
  const answers = (input) => (/Return no chapters/.test(input.messages[0].content) ? chat({ title: EARLY, chapters: [] }) : chat({ title: FULL, chapters: [{ start: "0:00", title: "Where we are" }, { start: "1:02", title: "the budget" }] }));
  /** The first minute up, asked as a Mac asks: again while the answer says there's more. */
  async function firstMinute(env) {
    const heard = await answerOf(env, { rec: REC, through: 10 });
    expect(heard).toMatchObject({ ai: true, step: "heard", more: true });
    return answerOf(env, { rec: REC, through: 10 });
  }
  const userOf = (call) => call.input.messages.find((m) => m.role === "user").content;

  it("names the take from its first minute while it records, so its page opens with that title", async () => {
    const env = await aiSpace({ name: answers });
    expect(await firstMinute(env)).toMatchObject({ ai: true, step: "titled", more: false });
    const calls = namingCalls(env);
    expect(calls).toHaveLength(1);
    expect(calls[0].model).toBe(AI_MODELS.name);
    expect(calls[0].input.messages[0].content).toContain("Return no chapters.");
    expect(userOf(calls[0])).toContain("[0:00] s0a");
    expect(userOf(calls[0])).toContain("s14h.");
    expect(userOf(calls[0])).not.toContain("s16a"); // 61.3 s: the next minute, not heard yet
    expect(record(env)).toMatchObject({ title: EARLY, named: { by: "ai", early: true, title: EARLY, chapters: [] } });
    expect(record(env).titled ?? false).toBe(false);
    // The owner's page has it from the start, in its field, and says where it came from.
    const html = await (await call(env, "GET", `/${REC}`, { headers: { cookie: cookieFor(M1, OWNER_MAC) } })).text();
    expect(html).toContain(`placeholder="${EARLY}"`);
    expect(textOf(html)).toContain(COPY.named);
    // Nothing more until the next minute is up: one early title a take.
    expect(await answerOf(env, { rec: REC, through: 10 })).toMatchObject({ step: "idle", more: false });
    expect(await answerOf(env, { rec: REC, through: 20 })).toMatchObject({ step: "heard" });
    expect(namingCalls(env)).toHaveLength(1);
  });

  it("the full name at done replaces it, with the chapters", async () => {
    const env = await aiSpace({ name: answers });
    await firstMinute(env);
    await finish(env);
    expect(namingCalls(env)).toHaveLength(2);
    expect(record(env)).toMatchObject({ title: FULL, named: { title: FULL, chapters: [[0, "Where we are"], [61.3, "The budget"]] } });
    expect(record(env).named.early).toBeUndefined();
  });

  it("a title typed after the early one wins for good; one typed before it costs no call", async () => {
    let env = await aiSpace({ name: answers });
    await firstMinute(env);
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), title: "Mine", titled: true }));
    await finish(env);
    expect(record(env)).toMatchObject({ title: "Mine", titled: true, named: { title: FULL } });
    env = await aiSpace({ name: answers, meta: { title: "Mine", titled: true } });
    expect(await answerOf(env, { rec: REC, through: 10 })).toMatchObject({ step: "heard", more: false });
    expect(namingCalls(env)).toHaveLength(0);
  });

  it("a take shorter than a minute is named once, at Stop", async () => {
    const env = await aiSpace({ pieces: 8, name: answers, words: script(sentences(11)) });
    expect(await answerOf(env, { rec: REC, through: 8 })).toMatchObject({ step: "idle", more: false });
    await finish(env);
    expect(namingCalls(env)).toHaveLength(1);
    expect(record(env).named.early).toBeUndefined();
  });

  it("with no speech in the first minute, it comes once there is some", async () => {
    const env = await aiSpace({ name: answers, words: script(sentences(18), { from: 70 }) });
    expect(await answerOf(env, { rec: REC, through: 10 })).toMatchObject({ step: "heard", more: false });
    expect(namingCalls(env)).toHaveLength(0);
    expect(await answerOf(env, { rec: REC, through: 20 })).toMatchObject({ step: "heard", more: true });
    expect(await answerOf(env, { rec: REC, through: 20 })).toMatchObject({ step: "titled" });
    expect(userOf(namingCalls(env)[0])).toContain("[1:10] s0a");
    expect(record(env).title).toBe(EARLY);
  });

  it("a slip cut with “shroom cut” in the first minute never reaches it", async () => {
    const lines = [...sentences(5), "Click on this link and you will.", "Shroom cut.", "To get your video use this link.", ...sentences(8, 10)];
    const env = await aiSpace({ name: answers, words: script(lines) });
    await firstMinute(env);
    const user = userOf(namingCalls(env)[0]);
    expect(user).toContain("s4h.");
    expect(user).toContain("To get your video use this link.");
    for (const gone of ["Click", "will.", "Shroom", "cut."]) expect(user).not.toContain(gone);
  });

  it("the owner's page keeps asking while the name is early, and takes the full one in place", async () => {
    const env = await aiSpace({ name: answers });
    await firstMinute(env);
    const cookie = cookieFor(M1, OWNER_MAC);
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toEqual({ named: EARLY, titled: false, later: false, early: true });
    expect(await (await call(env, "GET", `/${REC}`, { headers: { cookie } })).text()).toContain("/_shroom/named/");
    await finish(env);
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toEqual({ named: FULL, titled: false, later: false, early: false });
  });

  it("an early title that fails is let go, and holds nothing up", async () => {
    const env = await aiSpace({ name: answers, fail: (model, input) => { if (!model.includes("whisper") && /Return no chapters/.test(input.messages[0].content)) throw new Error("5006: the model said nothing useful"); } });
    expect(await firstMinute(env)).toMatchObject({ step: "titled", more: false });
    expect(record(env).named).toBeUndefined();
    expect(record(env).title).toBe("Untitled — Oct 2");
    expect(env.BUCKET.json(LATER)).toBeNull();
    // Tried once: the next minute is heard, and done names the take as ever.
    expect(await answerOf(env, { rec: REC, through: 20 })).toMatchObject({ step: "heard" });
    await finish(env);
    expect(record(env)).toMatchObject({ title: FULL, named: { title: FULL } });
  });

  it("a used-up free day holds the early title like any step, and the schedule takes it after 00:00 UTC", async () => {
    let capped = true;
    const env = await aiSpace({ name: answers, fail: (model) => { if (capped && !model.includes("whisper")) throw new Error("3036: You have used up your daily free allocation of 10,000 neurons."); } });
    expect(await firstMinute(env)).toEqual({ shroom: 1, ai: true, later: "daily" });
    expect(env.BUCKET.json(LATER)).toMatchObject({ reason: "daily", after: MIDNIGHT, through: 10 });
    capped = false;
    await aiTick(env, MIDNIGHT + MIN);
    expect(record(env)).toMatchObject({ title: EARLY, named: { early: true } });
  });
});

describe("the free day (B207)", () => {
  // What Workers AI throws on an account at its free day's cap (2026-10-18, word for word).
  const usedUp = () => { throw new Error("AiError: 4006: you have used up your daily free allocation of 10,000 neurons, please upgrade to Cloudflare's Workers Paid plan if you would like to continue usage."); };

  it("knows a used-up day from a busy model and from anything else", () => {
    expect(aiRefusal(new Error("AiError: 3036: You have used up your daily free allocation of 10,000 neurons."))).toBe("daily");
    // The code an account at its cap answered with on 2026-10-18 (the voice-cut tester's), however it's worded.
    expect(aiRefusal(new Error("AiError: 4006: Account limited"))).toBe("daily");
    // As a capped account answered on 2026-10-18 (the test account, on the free plan then), word for word.
    expect(aiRefusal(new Error("AiError: 4006: you have used up your daily free allocation of 10,000 neurons, please upgrade to Cloudflare's Workers Paid plan if you would like to continue usage."))).toBe("daily");
    expect(aiRefusal(new Error("3040: Capacity temporarily exceeded, please try again."))).toBe("busy");
    expect(aiRefusal(new Error("3007: Request timeout"))).toBe("busy");
    expect(aiRefusal(new Error("5007: No such model"))).toBe("error");
  });

  it("a used-up day is no error: the take waits for 00:00 UTC and the owner's page says so", async () => {
    let refuse = true;
    const env = await aiSpace({ fail: (model) => { if (refuse && model === AI_MODELS.hear) usedUp(); } });
    expect(await answerOf(env, { rec: REC, through: 10 })).toEqual({ shroom: 1, ai: true, later: "daily" });
    expect(await answerOf(env, { rec: REC, done: true })).toEqual({ shroom: 1, ai: true, later: "daily" });
    const mark = env.BUCKET.json(LATER);
    expect(mark).toMatchObject({ after: MIDNIGHT, done: true, through: 10, reason: "daily" });
    expect(env.BUCKET.customMetadata(LATER)).toEqual({ since: String(NOW), after: String(MIDNIGHT) });
    expect(record(env).naming).toEqual({ later: "daily", at: new Date(NOW).toISOString() });
    expect(record(env).title).toBe("Untitled — Oct 2");
    const page = await call(env, "GET", `/${REC}`, { headers: { cookie: cookieFor(M1, OWNER_MAC) } });
    expect(textOf(await page.text())).toContain(COPY.namingLater);

    // Before midnight the schedule leaves it be; after, it carries on, a step a minute, until the take is named.
    const before = env.AI.calls.length;
    await aiTick(env, MIDNIGHT - MIN);
    expect(env.AI.calls.length).toBe(before);
    refuse = false;
    for (let i = 0; i < 6 && env.BUCKET.json(LATER); i++) await aiTick(env, MIDNIGHT + (i + 1) * MIN);
    expect(env.BUCKET.json(LATER)).toBeNull();
    expect(record(env)).toMatchObject({ title: "Planning the spring launch", named: { by: "ai" } });
    expect(record(env).naming).toBeUndefined();
    const named = await call(env, "GET", `/${REC}`, { headers: { cookie: cookieFor(M1, OWNER_MAC) } });
    const text = textOf(await named.text());
    expect(text).toContain(COPY.named);
    expect(text).not.toContain(COPY.namingLater);
  });

  it("a busy model is tried again in five minutes, with nothing said", async () => {
    let busy = true;
    const env = await aiSpace({ fail: (model) => { if (busy && model === AI_MODELS.hear) throw new Error("3040: Capacity temporarily exceeded, please try again."); } });
    expect(await answerOf(env, { rec: REC, through: 10 })).toEqual({ shroom: 1, ai: true, later: "busy" });
    expect(env.BUCKET.json(LATER)).toMatchObject({ after: NOW + 5 * MIN, reason: "busy", done: false, through: 10 });
    expect(record(env).naming).toBeUndefined();
    busy = false;
    await aiTick(env, NOW + 6 * MIN);
    expect(env.BUCKET.json(TRANSCRIPT)).toMatchObject({ windows: 1 });
    // Then the early title, a step of its own (B205 f).
    await aiTick(env, NOW + 7 * MIN);
    expect(record(env).named).toMatchObject({ early: true });
    // Nothing more to hear until the Mac asks again: the mark goes.
    await aiTick(env, NOW + 8 * MIN);
    expect(env.BUCKET.json(LATER)).toBeNull();
  });

  it("the schedule takes the oldest waiting take first, one step a tick, and skips one another step holds", async () => {
    const env = await aiSpace();
    const other = "OtherRecording00";
    for (const [k, v] of Object.entries(takeObjects(other, 12))) await env.BUCKET.put(k, v);
    await env.BUCKET.put(metaKey(other), metaOf(other, { durationSec: 72 }));
    await ledger(env, other, M1);
    const mark = (id, since, after = 0) => env.BUCKET.put(`.shroom/ai/later/${id}`, JSON.stringify({ v: 1, since, after, done: true, through: 0, rename: false, reason: "more", tries: 0, k: -1 }), { customMetadata: { since: String(since), after: String(after) } });
    await mark(REC, NOW - 2 * MIN);
    await mark(other, NOW - 5 * MIN);
    await aiTick(env, NOW);
    expect(env.AI.calls).toHaveLength(1);
    expect(env.BUCKET.json(`rec/${other}/ai/transcript.json`)).toMatchObject({ windows: 1 });
    expect(env.BUCKET.json(TRANSCRIPT)).toBeNull();
    // A step holds the other take now (its lease): the next tick moves to the next oldest.
    await mark(other, NOW - 5 * MIN, NOW + MIN);
    await aiTick(env, NOW + 10 * SEC);
    expect(env.BUCKET.json(TRANSCRIPT)).toMatchObject({ windows: 1 });
  });

  it("forgets a deleted take's mark", async () => {
    const env = await aiSpace();
    await env.BUCKET.put(LATER, JSON.stringify({ v: 1, since: NOW, after: 0, done: true, through: 0, rename: false, reason: "more", tries: 0, k: -1 }), { customMetadata: { since: String(NOW), after: "0" } });
    await env.BUCKET.put(goneKey(REC), "{}");
    await aiTick(env, NOW);
    expect(env.AI.calls).toEqual([]);
    expect(env.BUCKET.json(LATER)).toBeNull();
  });

  it("runs from the Worker's schedule, and not at all without the AI", async () => {
    const env = await aiSpace();
    await env.BUCKET.put(LATER, JSON.stringify({ v: 1, since: NOW, after: 0, done: true, through: 0, rename: false, reason: "more", tries: 0, k: -1 }), { customMetadata: { since: String(NOW), after: "0" } });
    const waits = [];
    await worker.scheduled({ scheduledTime: NOW, cron: "* * * * *" }, env, { waitUntil: (p) => waits.push(p) });
    await Promise.all(waits);
    expect(whisperCalls(env)).toHaveLength(1);
    const off = await aiSpace({ ai: false });
    off.BUCKET.ops.length = 0;
    const w2 = [];
    await worker.scheduled({ scheduledTime: NOW, cron: "* * * * *" }, off, { waitUntil: (p) => w2.push(p) });
    await Promise.all(w2);
    expect(off.BUCKET.ops).toEqual([]);
  });
});

/** A named take: SPOKEN's words in its transcript and, by default, three chapters in its
 * record, snapped to 0, 42.3 s ("The budget") and 99.3 s ("Next steps"). */
async function namedSpace(meta = {}, chapters = [["0:00", "Where we are"], ["0:42", "The budget"], ["1:40", "Next steps"]]) {
  const env = await aiSpace({ name: () => chat({ title: "Planning the spring launch", chapters: chapters.map(([start, title]) => ({ start, title })) }) });
  await finish(env);
  const m = record(env);
  await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...m, state: "live", ...meta }));
  env.AI.calls.length = 0;
  env.BUCKET.ops.length = 0;
  return env;
}
const edits = (cuts, rev = 2, over = {}) => ({ version: 1, source_duration: 150, in: 0, out: 150, cuts: cuts.map(([start, end]) => ({ start, end, kind: "manual", applied: true })), rev, ...over });
const vtt = async (env, query = "") => { const res = await call(env, "GET", `/_shroom/cc/${REC}${query}`); return { res, text: await res.text() }; };
/** WebVTT cues as [start, end, text]. */
const cuesOf = (text) => [...text.matchAll(/(\d\d):(\d\d):(\d\d)\.(\d\d\d) --> (\d\d):(\d\d):(\d\d)\.(\d\d\d)\n([\s\S]*?)(?:\n\n|\n?$)/g)].map((m) => [
  Number(m[1]) * 3600 + Number(m[2]) * 60 + Number(m[3]) + Number(m[4]) / 1000, Number(m[5]) * 3600 + Number(m[6]) * 60 + Number(m[7]) + Number(m[8]) / 1000, m[9]]);

describe("captions (B205 e, B208 b, B209 b)", () => {
  it("leaves lone hesitations out of the captions, which the words keep for voice cuts", () => {
    const words = [["So", 0, 0.3], ["um,", 0.4, 0.6], ["we", 0.7, 0.9], ["start", 1.0, 1.2], ["uh.", 1.3, 1.5], ["Then", 2.6, 2.8], ["Erm", 2.9, 3.0], ["umbrellas.", 3.1, 3.6]];
    const cues = aiCaptions(words, null, false).split("\n\n").slice(1).map((c) => c.split("\n").slice(2).join(" "));
    expect(cues).toEqual(["So we start.", "Then umbrellas."]);
  });

  it("serves the words as WebVTT, each word once, in order", async () => {
    const env = await namedSpace();
    const { res, text } = await vtt(env);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("text/vtt; charset=utf-8");
    expect(text.startsWith("WEBVTT\n\n")).toBe(true);
    const cues = cuesOf(text);
    expect(cues.map((c) => c[2].replace(/\n/g, " ")).join(" ").split(" ")).toEqual(SPOKEN.map((w) => w.w));
    for (const [s, e] of cues) expect(e - s).toBeLessThanOrEqual(6.5);
    expect((await call(env, "HEAD", `/_shroom/cc/${REC}`)).status).toBe(200);
  });

  it("has none for a take without words, an unknown one or a deleted one", async () => {
    const env = await namedSpace({ captions: null });
    expect((await vtt(env)).res.status).toBe(404);
    expect((await call(env, "GET", "/_shroom/cc/NoSuchRecording0")).status).toBe(404);
    const gone = await namedSpace();
    await gone.BUCKET.put(goneKey(REC), "{}");
    expect((await vtt(gone)).res.status).toBe(404);
  });

  it("follows the saved edit: no word that's cut, no cue across a cut, on the clock of what plays", async () => {
    const env = await namedSpace({ edits: edits([[30, 40]]) });
    const cut = SPOKEN.filter((w) => (w.s + w.e) / 2 >= 30 && (w.s + w.e) / 2 < 40).map((w) => w.w);
    expect(cut.length).toBeGreaterThan(10);
    const original = cuesOf((await vtt(env)).text);
    const said = original.map((c) => c[2].replace(/\n/g, " ")).join(" ").split(" ");
    expect(said).toEqual(SPOKEN.map((w) => w.w).filter((w) => !cut.includes(w)));
    for (const [s, e] of original) expect(e <= 30.01 || s >= 39.99, `a cue spans the cut: ${s}–${e}`).toBe(true);
    // The editor's own copy plays the edited clock: everything after the cut ten seconds sooner.
    const copy = cuesOf((await vtt(env, "?e=1")).text);
    expect(copy.map((c) => c[2])).toEqual(original.map((c) => c[2]));
    for (let i = 0; i < copy.length; i++) expect(copy[i][0]).toBeCloseTo(original[i][0] >= 40 ? original[i][0] - 10 : original[i][0], 2);
  });

  it("writes a cue's words as text, never markup", () => {
    const text = aiCaptions([["<b>bold</b>", 0, 0.5], ["&", 0.6, 0.8], ["-->", 0.9, 1]], null, false);
    expect(text).toContain("&lt;b&gt;bold&lt;/b&gt; &amp; →");
  });

  it("ends a cue at a sentence's end, a pause, six seconds or two lines, breaking the line near its middle", () => {
    const w = (text, s) => [text, s, s + 0.3];
    const cues = (words) => cuesOf(aiCaptions(words, null, false)).map((c) => c[2].replace(/\n/g, " "));
    expect(cues([w("One.", 0), w("Two", 0.4)])).toEqual(["One.", "Two"]);
    expect(cues([w("One", 0), w("two", 1.5)])).toEqual(["One", "two"]);
    expect(cues(Array.from({ length: 20 }, (_, i) => w(`w${i}`, i * 0.4)))).toEqual(["w0 w1 w2 w3 w4 w5 w6 w7 w8 w9 w10 w11 w12 w13 w14", "w15 w16 w17 w18 w19"]);
    const long = cuesOf(aiCaptions(Array.from({ length: 12 }, (_, i) => w(`abcdefgh${i}`, i * 0.3)), null, false)).map((c) => c[2]);
    expect(long[0].split("\n")).toEqual(["abcdefgh0 abcdefgh1 abcdefgh2 abcdefgh3", "abcdefgh4 abcdefgh5 abcdefgh6 abcdefgh7"]);
    for (const c of long) for (const line of c.split("\n")) expect(line.length).toBeLessThanOrEqual(42);
  });
});

describe("edits (B208)", () => {
  // The editor's vectors (crates/shroom-edit/tests/keep-vectors.json) once that crate is in the
  // tree; until then the same ten cases, inline.
  const file = new URL("../../crates/shroom-edit/tests/keep-vectors.json", import.meta.url);
  const vectors = existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : [
    { name: "untouched", list: { in: 0, out: 272.4, source_duration: 272.4, cuts: [] }, keep: [[0, 272.4]] },
    { name: "trimmed both ends", list: { in: 8, out: 241, source_duration: 272.4, cuts: [] }, keep: [[8, 241]] },
    { name: "one hand cut", list: { in: 0, out: 60, source_duration: 60, cuts: [{ start: 10, end: 22, applied: true }] }, keep: [[0, 10], [22, 60]] },
    { name: "overlapping cuts merge", list: { in: 0, out: 60, source_duration: 60, cuts: [{ start: 10, end: 20, applied: true }, { start: 19.6, end: 25, applied: true }] }, keep: [[0, 10], [25, 60]] },
    { name: "an unapplied suggestion plays", list: { in: 0, out: 60, source_duration: 60, cuts: [{ start: 10, end: 20, applied: false }] }, keep: [[0, 60]] },
    { name: "a cut at each end", list: { in: 0, out: 30, source_duration: 30, cuts: [{ start: 0, end: 1.5, applied: true }, { start: 28.2, end: 30, applied: true }] }, keep: [[1.5, 28.2]] },
    { name: "cuts outside the trim are clamped", list: { in: 5, out: 50, source_duration: 60, cuts: [{ start: 0, end: 8, applied: true }, { start: 45, end: 58, applied: true }, { start: 20, end: 21, applied: true }] }, keep: [[8, 20], [21, 45]] },
    { name: "cuts out of order", list: { in: 0, out: 100, source_duration: 100, cuts: [{ start: 70, end: 80, applied: true }, { start: 10, end: 20, applied: true }, { start: 40, end: 45, applied: true }] }, keep: [[0, 10], [20, 40], [45, 70], [80, 100]] },
    { name: "a trim that leaves nothing is put back", list: { in: 6, out: 6.2, source_duration: 10, cuts: [] }, keep: [[0, 10]] },
    { name: "bad numbers are dropped", list: { in: 0, out: 10, source_duration: 10, cuts: [{ start: -5, end: 2, applied: true }, { start: 9, end: 99, applied: true }, { start: 4, end: 4, applied: true }] }, keep: [[2, 9]] },
  ];

  it.each(vectors)("reads the edit as the editor does: $name", (v) => {
    const want = v.expect?.keep ?? v.keep;
    // null: it all plays, as recorded (the editor's identity).
    const got = aiKeep({ edits: v.list });
    const keep = got ?? [[0, v.list.source_duration]];
    expect(got === null).toBe(v.expect ? v.expect.identity : want.length === 1 && want[0][0] === 0 && want[0][1] === v.list.source_duration);
    expect(keep).toHaveLength(want.length);
    keep.forEach(([s, e], i) => { expect(s).toBeCloseTo(want[i][0], 6); expect(e).toBeCloseTo(want[i][1], 6); });
    if (!v.expect) return;
    v.at_source.forEach((t, i) => {
      const x = aiToEdited(keep, t), y = v.expect.to_edited[i];
      if (y === null) expect(x, `to_edited(${t})`).toBeNull();
      else expect(x, `to_edited(${t})`).toBeCloseTo(y, 6);
    });
    v.at_edited.forEach((e, i) => expect(aiToSource(keep, e), `to_source(${e})`).toBeCloseTo(v.expect.to_source[i], 6));
  });

  it("moves a chapter that starts in a cut to where the cut ends, and drops one cut whole", () => {
    const meta = (over) => ({ durationSec: 180, named: { by: "ai", chapters: [[0, "A"], [60, "B"], [120, "C"]] }, ...over });
    expect(aiChapters(meta({}))).toEqual([[0, "A"], [60, "B"], [120, "C"]]);
    expect(aiChapters(meta({ edits: { ...edits([[50, 70]]), source_duration: 180, out: 180 } }))).toEqual([[0, "A"], [50, "B"], [100, "C"]]);
    expect(aiChapters(meta({ edits: { ...edits([[60, 120]]), source_duration: 180, out: 180 } }))).toEqual([[0, "A"], [60, "C"]]);
    // Fewer than 90 s left: none. A chapter left under 20 s folds into the one before.
    expect(aiChapters(meta({ edits: { ...edits([]), source_duration: 180, in: 0, out: 85 } }))).toEqual([]);
    expect(aiChapters(meta({ edits: { ...edits([[62, 110]]), source_duration: 180, out: 180 } }))).toEqual([[0, "A"], [72, "C"]]);
  });

  it("makes the name again after an edit that cuts a fifth of what was said, in the background, once", async () => {
    const env = await namedSpace();
    // Sentences 0–11 (96 of 312 words) go.
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), edits: edits([[0, 46]]) }));
    env.AI.calls.length = 0;
    const page = await call(env, "GET", `/${REC}`);
    expect(page.status).toBe(200);
    expect(env.BUCKET.json(LATER)).toMatchObject({ rename: true });
    expect(env.AI.calls).toEqual([]);
    await aiTick(env, NOW + MIN);
    expect(namingCalls(env)).toHaveLength(1);
    const user = namingCalls(env)[0].input.messages.find((m) => m.role === "user").content;
    expect(user).not.toContain("s0a");
    expect(user).toContain("[0:00] s12a"); // the first sentence after the cut, on the edited clock
    expect(record(env).named.rev).toBe("0:2");
    expect(env.BUCKET.json(LATER)).toBeNull();
    // The same revision is never weighed twice.
    await call(env, "GET", `/${REC}`);
    expect(env.BUCKET.json(LATER)).toBeNull();
  });

  it("keeps the name after a small edit, and stamps the revision it saw", async () => {
    const env = await namedSpace();
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), edits: edits([[140, 150]], 3) }));
    await call(env, "GET", `/${REC}`);
    expect(env.BUCKET.json(LATER)).toBeNull();
    expect(record(env).named.rev).toBe("0:3");
    expect(record(env).title).toBe("Planning the spring launch");
  });

  it("makes the name again when an edit cuts a whole chapter, though most of what was said stays", async () => {
    const env = await namedSpace({}, [["0:00", "Where we are"], ["0:42", "The budget"], ["1:05", "Costs"], ["1:40", "Next steps"]]);
    expect(record(env).named.chapters.map((c) => c[0])).toEqual([0, 42.3, 65.1, 99.3]);
    // "The budget" (42.3 s … 65.1 s) goes whole: 49 of 312 words.
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), edits: edits([[42, 65.5]]) }));
    await call(env, "GET", `/${REC}`);
    expect(env.BUCKET.json(LATER)).toMatchObject({ rename: true });
  });

  it("never renames over a typed title", async () => {
    const env = await namedSpace({ title: "Mine", titled: true });
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), edits: edits([[0, 75]]) }));
    await call(env, "GET", `/${REC}`);
    await aiTick(env, NOW + MIN);
    expect(record(env)).toMatchObject({ title: "Mine", titled: true });
  });

  it("carries the chapters, captions and words across a baked edit, by the record's bakedFrom", async () => {
    const env = await namedSpace();
    const before = record(env);
    expect(before.named.chapters.map((c) => c[0])).toEqual([0, 42.3, 99.3]);
    // The editor's bake (N63): the copy becomes the video, `edits` goes with it, and the bake
    // says what it kept — here 30 s … 50 s went, sentences 8–12. (Which file plays is the
    // editor's; the times are this one's.)
    const AT = "2026-10-04T10:00:00Z";
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...before, durationSec: 130, bakedFrom: [{ at: AT, keep: [[0, 30], [50, 150]] }] }));
    const html = await (await call(env, "GET", `/${REC}`)).text();
    // "The budget" began at 42.3 s, inside the cut: it begins where the cut ends, on the copy's clock.
    const shown = JSON.parse(html.match(/shroomChapters\(p,(\[\[.*?\]\])\)/)[1]);
    expect(shown.map((c) => c[1])).toEqual(["Where we are", "The budget", "Next steps"]);
    [0, 30, 79.3].forEach((t, i) => expect(shown[i][0]).toBeCloseTo(t, 6));
    // The captions play on the copy's clock and say no word that was cut.
    const { res, text } = await vtt(env, "?e=1");
    expect(res.status).toBe(200);
    const cues = cuesOf(text);
    const said = cues.map((c) => c[2].replace(/\n/g, " ")).join(" ");
    for (const w of ["s8a", "s10d", "s12h"]) expect(said).not.toContain(w);
    for (const w of ["s0a", "s14a", "s38h."]) expect(said).toContain(w);
    const at14 = cues.find((c) => c[2].includes("s14a"));
    expect(at14[0]).toBeGreaterThan(29.5);
    expect(at14[0]).toBeLessThan(33.8); // 53.7 s on the recording's clock
    for (const [, e] of cues) expect(e).toBeLessThanOrEqual(130);
    // After the view the stored words and the chapters are on the copy's clock, once; under a
    // fifth went, so the name stays and the bake is stamped.
    expect(env.BUCKET.json(TRANSCRIPT).clock).toBe(AT);
    expect(env.BUCKET.json(TRANSCRIPT).words.some(([w]) => w === "s8a")).toBe(false);
    expect(record(env).named).toMatchObject({ rev: "1:0", clock: AT, chapters: [[0, "Where we are"], [30, "The budget"], [79.3, "Next steps"]] });
    expect(env.BUCKET.json(LATER)).toBeNull();
    // A lone bake, not in a list, reads the same.
    expect(aiChapters({ durationSec: 130, named: before.named, bakedFrom: { at: AT, keep: [[0, 30], [50, 150]] } }).map((c) => c[1])).toEqual(["Where we are", "The budget", "Next steps"]);
  });

  it("words and chapters on a clock the bake list no longer holds aren't shown, rather than shown wrong", async () => {
    const env = await namedSpace();
    const AT = "2026-10-04T10:00:00Z";
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), durationSec: 130, bakedFrom: [{ at: AT, keep: [[0, 30], [50, 150]] }] }));
    await call(env, "GET", `/${REC}`);
    expect(record(env).named.clock).toBe(AT);
    // Twenty more bakes later the editor's list has dropped this one (it keeps the last 20).
    const later = Array.from({ length: 20 }, (_, i) => ({ at: `2026-10-05T10:${String(i).padStart(2, "0")}:00Z`, keep: [[0, 130]] }));
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), bakedFrom: later }));
    const transcript = JSON.stringify(env.BUCKET.json(TRANSCRIPT));
    const html = await (await call(env, "GET", `/${REC}`)).text();
    expect(html).not.toMatch(/shroomChapters\(p,\[/);
    expect((await vtt(env, "?e=1")).res.status).toBe(404);
    // The view weighs nothing and rewrites nothing.
    expect(JSON.stringify(env.BUCKET.json(TRANSCRIPT))).toBe(transcript);
    expect(env.BUCKET.json(LATER)).toBeNull();
  });

  it("a bake that cuts a fifth of what was said names the video again", async () => {
    const env = await namedSpace();
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), durationSec: 120, bakedFrom: [{ at: "2026-10-04T10:00:00Z", keep: [[0, 30], [60, 150]] }] }));
    await call(env, "GET", `/${REC}`);
    expect(env.BUCKET.json(LATER)).toMatchObject({ rename: true });
  });
});

describe("the page (B209)", () => {
  const live = async (env, cookie) => { const res = await call(env, "GET", `/${REC}`, cookie ? { headers: { cookie } } : {}); expect(res.status).toBe(200); return res.text(); };

  it("draws the chapters in the seek bar: they come with the page, script-safe", async () => {
    const env = await namedSpace();
    const m = record(env);
    m.named.chapters[1][1] = "</script><b>x</b>";
    await env.BUCKET.put(metaKey(REC), JSON.stringify(m));
    const html = await live(env);
    expect(html).toContain("shroomChapters(");
    expect(html).toContain(".chs{");
    expect(html).not.toContain("</script><b>x</b>");
    const island = html.match(/shroomChapters\(p,(\[\[.*?\]\])\)/)?.[1];
    expect(JSON.parse(island)).toEqual([[0, "Where we are"], [42.3, "</script><b>x</b>"], [99.3, "Next steps"]]);
  });

  it("puts a Captions button in the player row, off until pressed, and the track it turns on", async () => {
    const env = await namedSpace();
    const html = await live(env);
    expect(html).toMatch(new RegExp(`<track kind="captions" src="/_shroom/cc/${REC}\\?r=0%3A0" srclang="en" label="${COPY.captions}">`));
    expect(html).toContain(`<button type="button" class="cc" aria-label="${COPY.captions}" aria-pressed="false">`);
    expect(html).toContain("shroomCaptions(p)");
  });

  it("looks exactly as it did for a video the space never named", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { durationSec: 150 }) } });
    const html = await live(env);
    for (const s of ["shroomChapters", "shroomCaptions", "<track", 'class="cc"', ".chs{", "/_shroom/named/"]) expect(html).not.toContain(s);
  });

  it("has no chapters for a video under 90 s", async () => {
    const env = await namedSpace({ durationSec: 80, edits: edits([], 2, { out: 80 }) });
    expect(await live(env)).not.toMatch(/shroomChapters\(p,\[/);
  });

  it("the owner's page shows the named title in its field and says where it came from, until it's typed over", async () => {
    const env = await namedSpace();
    const cookie = cookieFor(M1, OWNER_MAC);
    let html = await live(env, cookie);
    expect(html).toContain(`placeholder="Planning the spring launch"`);
    expect(textOf(html)).toContain(COPY.named);
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), title: "Mine", titled: true }));
    html = await live(env, cookie);
    expect(textOf(html)).not.toContain(COPY.named);
  });

  it("the owner's page waits for a name on its way, and asks only its own space", async () => {
    const env = await aiSpace();
    const cookie = cookieFor(M1, OWNER_MAC);
    const m = JSON.parse(metaOf(REC, { durationSec: 150, createdAt: new Date(NOW - 2 * MIN).toISOString() }));
    await env.BUCKET.put(metaKey(REC), JSON.stringify(m));
    const html = await live(env, cookie);
    expect(html).toContain("/_shroom/named/");
    const res = await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ named: "", titled: false, later: false, early: false });
    expect((await call(env, "GET", `/_shroom/named/${REC}`)).status).toBe(403);
    expect((await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie: cookieFor(MHER, MHER_MAC) } })).status).toBe(403);
    await finish(env);
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toEqual({ named: "Planning the spring launch", titled: false, later: false, early: false });
  });

  it("a space without the AI never asks", async () => {
    const env = await aiSpace({ ai: false });
    expect(await live(env, cookieFor(M1, OWNER_MAC))).not.toContain("/_shroom/named/");
  });
});

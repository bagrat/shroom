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
/** The answer without its note for the Mac's log, which "what each step tells the Mac's log" holds. */
const answerOf = async (env, body, opts) => { const res = await ask(env, body, opts); expect(res.status, JSON.stringify(await bodyOf(res))).toBe(200); const { note: _, ...rest } = await bodyOf(res); return rest; };
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
    expect(fieldOf(html)).toMatchObject({ value: EARLY, autofocus: false });
    // With no line under it (Bagrat, 2026-10-25: removed).
    expect(textOf(html)).not.toContain(NAMED_LINE);
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
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toEqual({ named: EARLY, titled: false, later: false, early: true, done: false, chapters: [], captions: null, copy: null });
    expect(await (await call(env, "GET", `/${REC}`, { headers: { cookie } })).text()).toContain("/_shroom/named/");
    await finish(env);
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toEqual({ named: FULL, titled: false, later: false, early: false, done: true, ...(await reloadShows(env, cookie)), copy: null });
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
    expect(record(env).naming).toEqual({ later: "daily", at: new Date(NOW).toISOString(), code: "4006" });
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
    expect(text).not.toContain(COPY.namingLater);
    expect(text).not.toContain(NAMED_LINE);
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
    // No AI step at all; the schedule's other job, auto-cleanup (N75), only reads that it's off.
    expect(off.BUCKET.ops.filter((o) => !o.key.startsWith(".shroom/autoclean"))).toEqual([]);
    expect(off.BUCKET.written()).toEqual([]);
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

describe("what each step tells the Mac's log (diagnostics)", () => {
  // Bagrat, 2026-10-18 (relayed): a report from a Mac should show what happened in the space
  // too. Each answer carries one short line for the Mac's log, and the record keeps how the
  // naming went; neither ever holds a word said, a title, a key or a link.
  const words = /\bs\d+[a-h]\b|spring|launch|budget|where we are|no such model|allocation/i;
  async function steps(env, asks, opts) {
    const notes = [];
    for (const body of asks) {
      for (let i = 0; i < 12; i++) {
        const res = await ask(env, body, opts);
        expect(res.status).toBe(200);
        const b = await bodyOf(res);
        if (b.note) notes.push(b.note);
        if (!b.more) break;
      }
    }
    for (const n of notes) expect(n).not.toMatch(words);
    return notes;
  }

  it("a take's steps, one line each: the window heard, the early title, the name with its model, time, chapters and tokens", async () => {
    const env = await aiSpace();
    const notes = await steps(env, [{ rec: REC, through: 10 }, { rec: REC, through: 20 }, { rec: REC, done: true }]);
    expect(notes[0]).toMatch(/^heard window 1 in \d+ ms, \d+ words so far$/);
    expect(notes[1]).toMatch(/^early title by gpt-oss-120b in \d+ ms$/);
    expect(notes[2]).toMatch(/^heard window 2 in \d+ ms, \d+ words so far$/);
    expect(notes.at(-1)).toMatch(/^named by gpt-oss-120b in \d+ ms: \d+ chapters from \d+ words, 900\+80 tokens$/);
    expect(record(env).named).toMatchObject({ ms: expect.any(Number), tokens: [900, 80] });
    expect(record(env).named.fallback).toBeUndefined();
  });

  it("the second model says which one failed, and its code", async () => {
    const env = await aiSpace({
      fail: (model) => { if (model === AI_MODELS.name) throw new Error("5007: No such model"); },
      name: () => responseOf({ title: "the spring launch, planned.", chapters: [] }),
    });
    const notes = await steps(env, [{ rec: REC, done: true }]);
    expect(notes.at(-1)).toMatch(/^named by gemma-4-26b-a4b-it in \d+ ms: 0 chapters from \d+ words \(gpt-oss-120b failed: 5007\)$/);
    expect(record(env).named).toMatchObject({ model: AI_MODELS.fallback, fallback: "5007" });
  });

  it("a wait says why, with Workers AI's code and nothing of its words: the free day, a busy model, a failure tried again, a take let go", async () => {
    const usedUp = await aiSpace({ fail: (model) => { if (model === AI_MODELS.hear) throw new Error("AiError: 4006: you have used up your daily free allocation of 10,000 neurons."); } });
    expect(await steps(usedUp, [{ rec: REC, through: 10 }])).toEqual(["waits for 00:00 UTC: the free day is used up (4006)"]);
    expect(await steps(usedUp, [{ rec: REC, done: true }])).toEqual(["waiting (daily)"]);
    expect(record(usedUp).naming).toMatchObject({ later: "daily", code: "4006" });

    const busy = await aiSpace({ fail: (model) => { if (model === AI_MODELS.hear) throw new Error("3040: Capacity temporarily exceeded, please try again."); } });
    expect(await steps(busy, [{ rec: REC, through: 10 }])).toEqual(["waits 5 min: busy (3040)"]);

    const broken = await aiSpace({ fail: (model) => { if (model === AI_MODELS.hear) throw new Error("5007: No such model"); } });
    const notes = [];
    for (let i = 0; i < 5; i++) notes.push(...(await steps(broken, [{ rec: REC, done: true }], { now: NOW + i * 30 * MIN })));
    expect(notes).toEqual(["waits 5 min: error 5007, try 1 of 5", "waits 10 min: error 5007, try 2 of 5", "waits 15 min: error 5007, try 3 of 5", "waits 20 min: error 5007, try 4 of 5", "let go after 5 tries: error 5007"]);
    expect(record(broken).named).toMatchObject({ failed: true, code: "5007" });
  });
});

describe("captions (B205 e, B208 b, B209 b)", () => {
  it("leaves lone hesitations out of the captions, which the words keep for voice cuts", () => {
    const words = [["So", 0, 0.3], ["um,", 0.4, 0.6], ["we", 0.7, 0.9], ["start", 1.0, 1.2], ["uh.", 1.3, 1.5], ["Then", 2.6, 2.8], ["Erm", 2.9, 3.0], ["umbrellas.", 3.1, 3.6]];
    const cues = aiCaptions(words, null, false).trim().split("\n\n").slice(1).map((c) => c.split("\n").slice(2).join(" "));
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

/** The owner's title field as the page draws it: its value, its placeholder, and whether
 * the page opens in it. */
/** The line that said where the name came from, gone from the owner's page (Bagrat, 2026-10-25). */
const NAMED_LINE = "Named from what you said.";

/** What the owner's page shows of the AI once it's reloaded: the chapters its bar is drawn
 * with and its captions' track — what a page left open must come to, in place. */
async function reloadShows(env, cookie) {
  const html = await (await call(env, "GET", `/${REC}`, { headers: { cookie } })).text();
  const island = html.match(/shroomChapters\(p,(\[\[.*?\]\])\)/)?.[1];
  const track = html.match(/<track kind="captions" src="([^"]*)"(?: srclang="([^"]*)")?/);
  return { chapters: island ? JSON.parse(island) : [], captions: track ? { src: track[1].replace(/&amp;/g, "&"), lang: track[2] ?? "" } : null };
}

function fieldOf(html) {
  const tag = html.match(/<input class="title" id="t"[^>]*>/)?.[0];
  expect(tag, "the title field is on the page").toBeTruthy();
  const attr = (n) => tag.match(new RegExp(`\\s${n}="([^"]*)"`))?.[1] ?? null;
  return { value: attr("value"), placeholder: attr("placeholder"), autofocus: /\sautofocus/.test(tag) };
}

/** The owner page's naming poll, run against a stand-in title field: `answer(j)` plays the
 * next `/_shroom/named/` answer; `type(v)` is the owner typing; `click()` puts the
 * keyboard in the field. The field's own events are recorded, as a browser fires them. */
function namingScript(html, { value = fieldOf(html).value, focused = !!fieldOf(html).autofocus, live = true, playing = "/m/x/master.m3u8" } = {}) {
  const script = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1]).find((s) => s.includes("/_shroom/named/"));
  expect(script, "the naming poll is on the page").toBeTruthy();
  // The page's player functions are its own script's, defined beside the poll's.
  expect(html, "the player's chapters and captions can be wired after the page opened").toMatch(/<script>\s*function shroomChapters\(/);
  const on = {};
  const named = [];
  /** A stand-in player: its video (playing on), its row with the speed button, what was wired. */
  const el = (tag, attrs = {}) => ({ tag, ...attrs, children: [], appendChild(c) { this.children.push(c); c.parentNode = this; return c; } });
  const video = el("video", { src: playing, currentSrc: playing, paused: false, currentTime: 12.5 });
  const sp = el("button", { className: "sp" });
  const row = el("div", { insertBefore(c, at) { this.children.splice(this.children.indexOf(at), 0, c); c.parentNode = this; return c; } });
  row.children.push(sp); sp.parentNode = row;
  const wired = { chapters: [], captions: 0 };
  const player = { video, row, wired, classList: { has: new Set(), contains(k) { return this.has.has(k); }, add(k) { this.has.add(k); } },
    querySelector: (q) => (q === "video" ? video : q === ".sp" ? sp : q === ".cc" ? row.children.find((c) => c.className === "cc") ?? null : null) };
  const shroomChapters = (p, ch) => { wired.chapters.push(ch); p.classList.add("ch"); };
  const shroomCaptions = () => { wired.captions++; };
  const docOn = {};
  const document = {
    title: "", activeElement: null, getElementById: (id) => (id === "t" ? t : id === "ainote" ? note : id === "ns" ? shim : null),
    addEventListener: (type, fn) => (docOn[type] ??= []).push(fn),
    querySelector: (q) => (q === ".player" && live ? player : null),
    createElement: (tag) => Object.defineProperty(el(tag), "innerHTML", {
      set(h) { const m = h.match(/^<button type="button" class="([^"]*)" aria-label="([^"]*)" aria-pressed="([^"]*)">/); this.firstChild = m ? el("button", { className: m[1], label: m[2], pressed: m[3], html: h }) : null; },
    }),
  };
  const t = {
    value: value ?? "", placeholder: fieldOf(html).placeholder,
    addEventListener: (type, fn) => (on[type] ??= []).push(fn),
    dispatchEvent: (e) => { if (e.type === "shroom:named") named.push(e.detail); for (const fn of on[e.type] ?? []) fn(e); return true; },
    blur: () => { if (document.activeElement !== t) return; document.activeElement = null; for (const fn of on.blur ?? []) fn({}); },
    focus: () => { document.activeElement = t; },
    select: () => { t.selected = true; },
  };
  const note = { textContent: "", hidden: true };
  /** The title's shimmer (N82), as the page draws it: there only while the name is on its way. */
  const shim = /<span class="nameshim" id="ns"/.test(html) ? { hidden: true, style: {}, gone: false, remove() { this.gone = true; } } : null;
  if (focused) document.activeElement = t;
  const timers = [];
  let pending = null;
  let asked = 0;
  const fetch = () => { asked++; return new Promise((ok) => { pending = ok; }); };
  const CustomEvent = function (type, init) { return { type, detail: init?.detail }; };
  new Function("document", "fetch", "setTimeout", "CustomEvent", "shroomChapters", "shroomCaptions", script)(document, fetch, (fn) => timers.push(fn), CustomEvent, shroomChapters, shroomCaptions);
  const settled = () => new Promise((ok) => setTimeout(ok, 0));
  return {
    t, note, shim, document, named, player, timers,
    /** The page's line ending (N82): the finishing line faded, for whatever reason. */
    finished() { for (const fn of docOn["shroom:finished"] ?? []) fn({}); },
    /** The status saying the name is in: whether this script took it, and when it's dressed. */
    namedIn() { const d = { taken: false, ready: 0, go() { this.ready++; } }; for (const fn of docOn["shroom:named-in"] ?? []) fn({ detail: d }); return d; },
    goLive() { live = true; },
    async answer(j) { timers.shift()(); pending({ ok: true, json: () => Promise.resolve(j) }); await settled(); await settled(); },
    /** The answer to an ask the page made on its own (not on its timer). */
    async reply(j) { pending({ ok: true, json: () => Promise.resolve(j) }); await settled(); await settled(); },
    /** What now plays changed (a voice cut landed, a strip press): the player's own event. */
    keep(k) { for (const fn of docOn["shroom:keep"] ?? []) fn({ detail: k }); },
    get asked() { return asked; },
    click() { for (const fn of on.pointerdown ?? []) fn({}); document.activeElement = t; for (const fn of on.focus ?? []) fn({}); },
    type(v) { t.value = v; for (const fn of on.input ?? []) fn({}); },
  };
}

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
    // On the head's travel, inset by its radius (Bagrat, 2026-10-25): the sections, the tip
    // under the pointer and the one above a finger all place time as the head does.
    // The first and last sections reach the bar's ends, their outer 7 px padding: the chapters'
    // boundaries land on the head's travel, and nothing is drawn beside them (review of B250).
    expect(html).toContain(".chs{position:absolute;left:0;right:0;");
    expect(html).toContain(".chs i:first-child{padding-left:7px;--a:7px}.chs i:last-child{padding-right:7px;--z:7px}");
    expect(html).toContain(".player.ch .seek::-webkit-slider-runnable-track{background:transparent}");
    expect(html).toContain("function at2x(t,w){var d=len();return 7+(d?t/d:0)*(w-14)}");
    expect(html).toContain("show(len()*Math.max(0,Math.min(1,(x-7)/((r.width-14)||1))),x)");
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

  it("the owner's page shows the named title in its field, with no line under it (Bagrat, 2026-10-25)", async () => {
    const env = await namedSpace();
    const cookie = cookieFor(M1, OWNER_MAC);
    let html = await live(env, cookie);
    // The name is the title (Bagrat's report, 2026-10-24): the field holds it as its value,
    // drawn as plain text until it's clicked, and the page doesn't open in the field.
    expect(fieldOf(html)).toEqual({ value: "Planning the spring launch", placeholder: "Untitled — Oct 2", autofocus: false });
    // Leaving the field untouched saves nothing: the name stays the space's (B209 c).
    expect(html).toContain(`"saved":"Planning the spring launch"`);
    expect(textOf(html)).not.toContain(NAMED_LINE);
    expect(html).not.toContain('id="ainote"');
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), title: "Mine", titled: true }));
    html = await live(env, cookie);
    expect(fieldOf(html).value).toBe("Mine");
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
    expect(await res.json()).toEqual({ named: "", titled: false, later: false, early: false, done: false, chapters: [], captions: null, copy: null });
    expect((await call(env, "GET", `/_shroom/named/${REC}`)).status).toBe(403);
    expect((await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie: cookieFor(MHER, MHER_MAC) } })).status).toBe(403);
    await finish(env);
    // With what a reload would show: the chapters on the bar's clock and the captions' track.
    const shows = await reloadShows(env, cookie);
    expect(shows.chapters.length).toBeGreaterThan(1);
    expect(shows.captions).toEqual({ src: expect.stringMatching(new RegExp(`^/_shroom/cc/${REC}\\?r=`)), lang: "en" });
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toEqual({ named: "Planning the spring launch", titled: false, later: false, early: false, done: true, ...shows, copy: null });
  });

  it("the name's answer is done only once the take's voice cuts are in (review of B241)", async () => {
    // A space that listens, the take live a minute ago: its cues are still being judged.
    const env = await aiSpace({ meta: { publishedAt: new Date(NOW - MIN).toISOString() } });
    await finish(env);
    const cookie = cookieFor(M1, OWNER_MAC);
    const ask = async () => (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json();
    expect(await ask()).toMatchObject({ named: "Planning the spring launch", done: false });
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), edits: { ...edits([[30, 40]], 2), voice: { planned_at: NOW, cues: 1 } } }));
    const now = await ask();
    expect(now).toMatchObject({ done: true, ...(await reloadShows(env, cookie)) });
    expect(now.chapters).toEqual([[0, "Where we are"], [51.3, "The budget"]]);
  });

  it("names the edit's own copy, whose clock the captions take; a baked file's own name isn't one (review of B241)", async () => {
    const env = await namedSpace();
    const cookie = cookieFor(M1, OWNER_MAC);
    const COPY = { rev: 2, file: "edit-2.mp4", duration: 140 };
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), edits: { ...edits([[30, 40]], 2), copy: COPY } }));
    let html = await live(env, cookie);
    expect(html).toMatch(new RegExp(`<track kind="captions" src="/_shroom/cc/${REC}\\?r=[^"&]*&amp;e=1"`));
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toMatchObject({ copy: `/m/${REC}/edit-2.mp4` });
    // Baked (B197): edit-3.mp4 is the recording now; a newer edit, its copy not up yet, plays
    // the file with its cuts skipped, on the file's own clock.
    const BAKED = { hls: null, mp4: `rec/${REC}/edit-3.mp4`, poster: null, width: 1920, height: 1080 };
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), durationSec: 150, media: BAKED, bakedFrom: [{ at: "2026-10-03T10:00:00Z", rev: 3, keep: [[0, 150]] }], edits: edits([[30, 40]], 4) }));
    html = await live(env, cookie);
    expect(html).toContain(`/m/${REC}/edit-3.mp4`);
    expect(html).toMatch(new RegExp(`<track kind="captions" src="/_shroom/cc/${REC}\\?r=[^"]*"`));
    expect(html).not.toMatch(/<track kind="captions" src="[^"]*e=1"/);
    expect(await (await call(env, "GET", `/_shroom/named/${REC}`, { headers: { cookie } })).json()).toMatchObject({ copy: null });
  });

  /** An untitled take's owner page, opened while its name is on its way. */
  const waitingPage = async () => {
    const env = await aiSpace();
    const m = JSON.parse(metaOf(REC, { durationSec: 150, createdAt: new Date(NOW - 2 * MIN).toISOString() }));
    await env.BUCKET.put(metaKey(REC), JSON.stringify(m));
    return live(env, cookieFor(M1, OWNER_MAC));
  };

  it("a name that arrives is the title: it fills the field and the field lets go of the keyboard (Bagrat's report, 2026-10-24)", async () => {
    const html = await waitingPage();
    expect(fieldOf(html)).toMatchObject({ value: "", autofocus: true });
    const p = namingScript(html);
    await p.answer({ named: "Planning the spring launch", titled: false, later: false, early: false });
    // The value, not the placeholder: drawn as plain text, its caret free once it's clicked.
    expect(p.t.value).toBe("Planning the spring launch");
    expect(p.document.title).toBe("Planning the spring launch");
    expect(p.document.activeElement).toBeNull();
    // The page's save hears the name first, so letting go saves nothing.
    expect(p.named).toEqual(["Planning the spring launch"]);
    // Nothing said under it (Bagrat, 2026-10-25: the named-from line removed).
    expect(p.note).toMatchObject({ textContent: "", hidden: true });
  });

  it("the full name takes the early one's place in an untouched field", async () => {
    const p = namingScript(await waitingPage());
    await p.answer({ named: "Spring", titled: false, later: false, early: true });
    expect(p.t.value).toBe("Spring");
    await p.answer({ named: "Planning the spring launch", titled: false, later: false, early: false });
    expect(p.t.value).toBe("Planning the spring launch");
    expect(p.named).toEqual(["Spring", "Planning the spring launch"]);
  });

  it("a name arriving while the owner types leaves the typing be, and the keyboard where it is", async () => {
    const p = namingScript(await waitingPage());
    p.click();
    p.type("My own tit");
    await p.answer({ named: "Planning the spring launch", titled: false, later: false, early: false });
    expect(p.t.value).toBe("My own tit");
    expect(p.document.activeElement).toBe(p.t);
    expect(p.document.title).toBe("");
  });

  it("a name arriving after the owner clicked in, before a key, fills the field ready to retype", async () => {
    const p = namingScript(await waitingPage());
    p.click();
    await p.answer({ named: "Planning the spring launch", titled: false, later: false, early: false });
    expect(p.t.value).toBe("Planning the spring launch");
    expect(p.document.activeElement).toBe(p.t);
    expect(p.t.selected).toBe(true);
  });

  it("the page's save takes the name as what's saved, so an untouched field writes nothing", async () => {
    const html = await waitingPage();
    expect(html).toMatch(/t\.addEventListener\("shroom:named",function\(e\)\{saved=e\.detail\}\)/);
  });

  // Bagrat's hand test, 2026-10-25: the name came in place, the chapters and captions only
  // after a refresh. They come with the full name, and go in where the reload would put them.
  const CHAPTERS = [[0, "Where we are"], [61.3, "The budget"]];
  const CAPTIONS = { src: `/_shroom/cc/${REC}?r=0%3A0`, lang: "en" };
  const full = (over = {}) => ({ named: "Planning the spring launch", titled: false, later: false, early: false, done: true, chapters: CHAPTERS, captions: CAPTIONS, ...over });

  it("the chapters and captions that come with the name go into the player in place, as it plays", async () => {
    const p = namingScript(await waitingPage());
    await p.answer({ named: "Spring", titled: false, later: false, early: true, done: false, chapters: [], captions: null, copy: null });
    expect(p.player.wired).toEqual({ chapters: [], captions: 0 });
    await p.answer(full());
    // The bar cut into its chapters, once.
    expect(p.player.wired.chapters).toEqual([CHAPTERS]);
    // The captions' track on the video, the Captions button before the speed, wired once.
    expect(p.player.video.children).toEqual([expect.objectContaining({ tag: "track", kind: "captions", src: CAPTIONS.src, srclang: "en", label: COPY.captions })]);
    expect(p.player.row.children.map((c) => c.className)).toEqual(["cc", "sp"]);
    expect(p.player.row.children[0]).toMatchObject({ label: COPY.captions, pressed: "false" });
    expect(p.player.wired.captions).toBe(1);
    // Playback isn't touched: the same source, still playing, where it was.
    expect(p.player.video).toMatchObject({ src: "/m/x/master.m3u8", paused: false, currentTime: 12.5 });
    expect(p.t.value).toBe("Planning the spring launch");
    // Done: nothing more is asked.
    expect(p.timers).toHaveLength(0);
  });

  it("the captions follow the clock of what plays: an edit's own copy takes the edited one", async () => {
    const p = namingScript(await waitingPage(), { playing: `https://v.example.com/m/${REC}/edit-3.mp4` });
    await p.answer(full({ copy: `/m/${REC}/edit-3.mp4` }));
    expect(p.player.video.children[0].src).toBe(`${CAPTIONS.src}&e=1`);
  });

  it("a baked recording's own file plays on its own clock, whatever its name (review of B241)", async () => {
    // A baked video plays `edit-3.mp4` as the recording, a newer edit's cuts skipped as it plays.
    const p = namingScript(await waitingPage(), { playing: `https://v.example.com/m/${REC}/edit-3.mp4` });
    await p.answer(full({ copy: null }));
    expect(p.player.video.children[0].src).toBe(CAPTIONS.src);
  });

  // Review of B241: a voice cut lands after the name (the Mac plans its cuts from the finished
  // transcript the naming step read). The bar must come to what a reload draws.
  it("chapters that change after they were drawn are drawn again; the asking goes on while the cuts are coming", async () => {
    const p = namingScript(await waitingPage());
    await p.answer(full({ done: false }));
    expect(p.player.wired.chapters).toEqual([CHAPTERS]);
    expect(p.timers).toHaveLength(1);
    const CUT = [[0, "Where we are"], [51.3, "The budget"]];
    await p.answer(full({ chapters: CUT }));
    expect(p.player.wired.chapters).toEqual([CHAPTERS, CUT]);
    // Captions are wired once; their track runs on the recording's clock either way.
    expect(p.player.wired.captions).toBe(1);
    expect(p.timers).toHaveLength(0);
  });

  it("when what plays changes after the asking stopped, it asks once more and redraws the bar", async () => {
    const p = namingScript(await waitingPage());
    await p.answer(full());
    expect(p.timers).toHaveLength(0);
    const before = p.asked;
    p.keep([[0, 30], [40, 150]]);
    expect(p.asked).toBe(before + 1);
    const CUT = [[0, "Where we are"], [51.3, "The budget"]];
    await p.reply(full({ chapters: CUT }));
    expect(p.player.wired.chapters).toEqual([CHAPTERS, CUT]);
    // The same chapters again draw nothing new.
    p.keep([[0, 30], [40, 150]]);
    await p.reply(full({ chapters: CUT }));
    expect(p.player.wired.chapters).toEqual([CHAPTERS, CUT]);
    expect(p.timers).toHaveLength(0);
  });

  it("a title typed while the name is on its way still gets the chapters and captions", async () => {
    const p = namingScript(await waitingPage());
    p.click();
    p.type("Mine");
    p.t.dispatchEvent({ type: "change" });
    await p.answer({ named: "", titled: true, later: false, early: false, done: false, chapters: [], captions: null });
    expect(p.timers).toHaveLength(1);
    await p.answer(full({ named: "", titled: true }));
    expect(p.t.value).toBe("Mine");
    expect(p.player.wired.chapters).toEqual([CHAPTERS]);
    expect(p.player.wired.captions).toBe(1);
  });

  it("a name that lands before the player does is dressed once the player is there", async () => {
    const p = namingScript(await waitingPage(), { live: false });
    await p.answer(full());
    expect(p.t.value).toBe("Planning the spring launch");
    expect(p.timers).toHaveLength(1);
    p.goLive();
    await p.answer(full());
    expect(p.player.wired.chapters).toEqual([CHAPTERS]);
    expect(p.timers).toHaveLength(0);
  });

  it("a name with neither chapters nor captions ends the asking, touching nothing", async () => {
    const p = namingScript(await waitingPage());
    await p.answer(full({ chapters: [], captions: null }));
    expect(p.player.wired).toEqual({ chapters: [], captions: 0 });
    expect(p.player.video.children).toEqual([]);
    expect(p.timers).toHaveLength(0);
  });

  it("a space without the AI never asks", async () => {
    const env = await aiSpace({ ai: false });
    expect(await live(env, cookieFor(M1, OWNER_MAC))).not.toContain("/_shroom/named/");
  });
});

// Bagrat's re-check of 0.1.33-early.1, 2026-10-25: the chapters and captions came after the
// finishing spinner had gone. N82: the line lasts until the name is in, and the empty title
// shimmers while it's on its way; a typed title still takes the chapters and captions.
describe("the name on its way (N82)", () => {
  const live = async (env, cookie) => { const res = await call(env, "GET", `/${REC}`, cookie ? { headers: { cookie } } : {}); expect(res.status).toBe(200); return res.text(); };
  /** A take that went live a minute ago in a space that listens, its cues planned (none), not yet named. */
  const fresh = (over = {}) => ({ durationSec: 150, createdAt: new Date(NOW - 3 * MIN).toISOString(), publishedAt: new Date(NOW - MIN).toISOString(), edits: { ...edits([], 1), voice: { planned_at: NOW - 30 * SEC, cues: 0 } }, ...over });
  const freshPage = async (over = {}) => { const env = await aiSpace({ meta: fresh(over) }); return { env, html: await live(env, cookieFor(M1, OWNER_MAC)) }; };
  const statusOf = async (env) => (await call(env, "GET", `/_shroom/status/${REC}`)).json();
  const lineOf = (html) => html.match(/<div class="upl" id="ul"[^>]*>[\s\S]*?<\/div>/)?.[0] ?? "";

  it("the status says the name is coming while a space that listens names a fresh take, and stops once it's in", async () => {
    const { env } = await freshPage();
    expect(await statusOf(env)).toMatchObject({ named: "coming" });
    // The early title doesn't end the wait; the full name does, and so does a naming that ended with none.
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), named: { by: "ai", early: true, title: "Spring", chapters: [] } }));
    expect(await statusOf(env)).toMatchObject({ named: "coming" });
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), named: { by: "ai", title: null, chapters: [], words: 4 } }));
    expect((await statusOf(env)).named).toBeUndefined();
    await env.BUCKET.put(metaKey(REC), JSON.stringify({ ...record(env), named: { by: "ai", title: null, chapters: [], failed: true } }));
    expect((await statusOf(env)).named).toBeUndefined();
  });

  it("a typed title doesn't end the wait; a space that doesn't listen, the free day used up or five minutes passed do", async () => {
    expect(await statusOf((await freshPage({ titled: true, title: "Mine" })).env)).toMatchObject({ named: "coming" });
    const quiet = space({ objects: { [metaKey(REC)]: metaOf(REC, fresh()) } });
    expect((await statusOf(quiet)).named).toBeUndefined();
    expect((await statusOf((await freshPage({ naming: { later: "daily", at: new Date(NOW).toISOString() } })).env)).named).toBeUndefined();
    expect((await statusOf((await freshPage({ publishedAt: new Date(NOW - 6 * MIN).toISOString() })).env)).named).toBeUndefined();
  });

  it("the finishing line stays after the cuts are planned while the name is on its way, its spinner turning, counting the five minutes", async () => {
    const { html } = await freshPage();
    const line = lineOf(html);
    expect(line).not.toMatch(/ hidden/);
    expect(line).toContain('<span class="spin" id="b"');
    expect(line).toContain(COPY.finishing);
    expect(html).toMatch(/"named":"coming"/);
    const wait = Number(html.match(/"listenFor":(\d+)/)?.[1]);
    expect(wait).toBeGreaterThan(3 * MIN);
    expect(wait).toBeLessThanOrEqual(4 * MIN);
    // Named: nothing left to say, the line goes.
    const named = await namedSpace(fresh());
    expect(lineOf(await live(named, cookieFor(M1, OWNER_MAC)))).toMatch(/ hidden/);
  });

  it("the empty title shimmers while the name is on its way, and the page doesn't open in the field", async () => {
    const { html } = await freshPage();
    expect(html).toContain('<span class="nameshim" id="ns" aria-hidden="true" hidden></span>');
    expect(html).toContain(".nameshim{");
    expect(fieldOf(html)).toMatchObject({ value: "", autofocus: false });
    // Not on a typed title, an early one in the field, a named take, words waiting for the free day, or past its five minutes.
    for (const over of [{ titled: true, title: "Mine" }, { named: { by: "ai", early: true, title: "Spring", chapters: [] }, title: "Spring" }, { naming: { later: "daily", at: new Date(NOW).toISOString() } }, { publishedAt: new Date(NOW - 6 * MIN).toISOString() }]) {
      expect((await freshPage(over)).html, JSON.stringify(over)).not.toContain('id="ns"');
    }
    expect(await live(await namedSpace(fresh()), cookieFor(M1, OWNER_MAC))).not.toContain('id="ns"');
  });

  it("a viewer's page never shimmers nor waits", async () => {
    const { env } = await freshPage();
    const html = await live(env);
    for (const s of ['id="ns"', ".nameshim{", "/_shroom/named/", 'id="ul"']) expect(html).not.toContain(s);
  });

  it("the shimmer shows once placed, and a click in the field drops it at once and takes typing", async () => {
    const p = namingScript((await freshPage()).html);
    expect(p.shim.gone).toBe(false);
    p.click();
    expect(p.shim.gone).toBe(true);
    p.type("My own");
    await p.answer({ named: "Planning the spring launch", titled: false, later: false, early: false, done: true, chapters: [], captions: null });
    expect(p.t.value).toBe("My own");
  });

  it("the shimmer stops with the line, whether a name came or not", async () => {
    const p = namingScript((await freshPage()).html);
    p.finished();
    expect(p.shim.gone).toBe(true);
    const q = namingScript((await freshPage()).html);
    await q.answer({ named: "Planning the spring launch", titled: false, later: false, early: false, done: true, chapters: [], captions: null });
    expect(q.shim.gone).toBe(true);
  });

  it("when the status says the name is in, the script asks at once and lets the line fade once the player is dressed", async () => {
    const p = namingScript((await freshPage()).html);
    const before = p.asked;
    const d = p.namedIn();
    expect(d.taken).toBe(true);
    expect(p.asked).toBe(before + 1);
    expect(d.ready).toBe(0);
    await p.reply({ named: "Planning the spring launch", titled: false, later: false, early: false, done: true, chapters: [[0, "Where we are"], [61.3, "The budget"]], captions: { src: `/_shroom/cc/${REC}?r=0%3A0`, lang: "en" } });
    expect(p.t.value).toBe("Planning the spring launch");
    expect(p.player.wired.chapters).toEqual([[[0, "Where we are"], [61.3, "The budget"]]]);
    expect(d.ready).toBe(1);
  });

  it("a page whose title was typed before it opened still asks, and takes the chapters and captions in place", async () => {
    const { html } = await freshPage({ titled: true, title: "Mine" });
    expect(html).toContain("/_shroom/named/");
    const p = namingScript(html);
    await p.answer({ named: "", titled: true, later: false, early: false, done: true, chapters: [[0, "Where we are"], [61.3, "The budget"]], captions: { src: `/_shroom/cc/${REC}?r=0%3A0`, lang: "en" } });
    expect(p.t.value).toBe("Mine");
    expect(p.player.wired.chapters).toEqual([[[0, "Where we are"], [61.3, "The budget"]]]);
    expect(p.player.wired.captions).toBe(1);
  });
});

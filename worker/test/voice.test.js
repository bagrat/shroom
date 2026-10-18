// Voice cuts on the owner page (spec N65, the face `owner-page-cuts`; DECISIONS B200, B201):
// where the speaker said “shroom cut”, the Mac lands cuts in the record's edit list (N63)
// once the take is heard; the owner page shows them under the player, in owner-mode only,
// with Put back, Cut again and Cut it too, saved through POST /_shroom/edits/<id> under
// If-Match (B21). Viewers get the cut video and nothing else: never the strip, never the
// words cut.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it } from "vitest";
import { forgetSecret, handle, status } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";
import { MHER, MHER_MAC, OWNER_MAC, ROSTER, ledgerKey, roster } from "./space-fixture.js";

const machine = JSON.parse(readFileSync(new URL("../../spec/machine.json", import.meta.url), "utf8"));
const face = machine.states["owner-page-cuts"];
/** N65's words, by said-line id. */
const say = (id) => {
  const line = face.said.find((l) => l.id === `owner-page-cuts.${id}`);
  if (!line) throw new Error(`no said-line owner-page-cuts.${id}`);
  return line.text;
};
const html = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

const ID = "AbCdEfGhIjKlMnOp";
const HOST = "https://v.example.workers.dev";
const NOW = Date.parse("2026-10-02T12:00:00Z");
const K = "kKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKk";
const sha = (s) => createHash("sha256").update(s).digest("hex");
const SECRET = Buffer.alloc(32, 7).toString("base64url");

/** A take's edit list as the Mac lands it: two retakes cut, a cue with nothing said again
 * after it (its own words cut, the line before it offered), and a hand cut from the editor.
 * `voice` is the editor's mark that the cues were planned (N63's `Voice`); `rev` its save count. */
const EDITS = {
  version: 1, source_duration: 272, in: 0, out: 272, rev: 2,
  voice: { planned_at: NOW - 60_000, cues: 3 },
  cuts: [
    { id: "v1", start: 4.1, end: 7.5, kind: "retake", confidence: "high", applied: true, removed: "Hello, that is shroom. Shroom cut.", resumes: "Hello, this is shroom." },
    { id: "v2", start: 72.0, end: 76.1, kind: "retake", confidence: "high", applied: true, removed: "Then you pick the, the window you, um, shroom cut.", resumes: "So, then you pick the window" },
    { id: "v3", start: 165.0, end: 167.0, kind: "cue-only", confidence: "high", applied: true, removed: "Shroom cut.", resumes: "Let's move on to the Library" },
    { id: "v4", start: 160.4, end: 167.0, kind: "line-before-cue", confidence: "low", applied: false, removed: "The second one is, um, I forget what it's called. Shroom cut.", resumes: "Let's move on to the Library" },
    { id: "m1", start: 100.0, end: 106.0, kind: "manual", confidence: "high", applied: true, removed: "" },
  ],
};
const metaOf = (over = {}) => JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: "2026-10-02T11:00:00Z", publishedAt: "2026-10-02T11:05:00Z", title: "Setting up shroom",
  media: { hls: `rec/${ID}/master.m3u8`, mp4: null, width: 1920, height: 1080 }, owner: "m1", edits: EDITS, ...over });
const grant = (over = {}) => JSON.stringify({ member: "m1", role: "owner", device: OWNER_MAC.id, exp: new Date(NOW + 600_000).toISOString(), ...over });
const memberGrant = () => grant({ member: MHER, role: "member", device: MHER_MAC.id });

const req = (env, path, init = {}) => handle(new Request(`${HOST}${path}`, init), env, NOW);
/** A space that listens (N66) has Cloudflare's AI bound to its page. */
const space = (extra = {}, listens = false) => ({ ...(listens ? { AI: {} } : {}), BUCKET: fakeBucket({ ".shroom/cookie.key": SECRET, [ROSTER]: JSON.stringify(roster()), [`meta/${ID}.json`]: metaOf(), ...extra }) });
async function cookieFor(env, g = grant()) {
  const k = `${K.slice(0, 42)}Q`;
  env.BUCKET.store.set(`.shroom/keys/${sha(k)}`, new TextEncoder().encode(g));
  const res = await req(env, `/${ID}?k=${k}`);
  const set = res.headers.get("set-cookie");
  return set ? set.split(";")[0] : null;
}
const post = (env, path, body, cookie, origin = HOST) =>
  req(env, path, { method: "POST", headers: { "content-type": "application/json", origin, ...(cookie ? { cookie } : {}) }, body: JSON.stringify(body) });
const stored = (env) => JSON.parse(new TextDecoder().decode(env.BUCKET.store.get(`meta/${ID}.json`)));
const page = async (env, cookie) => (await req(env, `/${ID}`, cookie ? { headers: { cookie } } : {})).text();
const WORDS_CUT = EDITS.cuts.map((c) => c.removed).filter(Boolean);

beforeEach(() => forgetSecret());

describe("the owner page shows the voice cuts under the player (owner-page-cuts)", () => {
  it("to an owner-class browser: the heading, the note, and a row for each cut", async () => {
    const env = space();
    const body = await page(env, await cookieFor(env));
    expect(body).toContain(html(say("cut-where-you-said")));
    expect(body).toContain(html(say("only-you-see-this")));
    // The strip sits under the player, before the upload line.
    expect(body.indexOf('class="cuts"')).toBeGreaterThan(body.indexOf('class="player"'));
    // Each retake is struck, with Put back; times are on the video (m:ss).
    for (const [id, at, words] of [["v1", "0:04", "Hello, that is shroom. Shroom cut."], ["v2", "1:12", "Then you pick the, the window you, um, shroom cut."]]) {
      const row = body.match(new RegExp(`<li[^>]*data-cut="${id}"[^>]*>[\\s\\S]*?</li>`))?.[0] ?? "";
      expect(row, id).toContain('class="cut on"');
      expect(row).toContain(at);
      expect(row).toContain(html(`“${words}”`));
      expect(row).toContain(`>${html(say("put-back"))}</button>`);
    }
  });

  it("offers the line before a cue with nothing said again after it: its words not struck, the note, and Cut it too", async () => {
    const env = space();
    const body = await page(env, await cookieFor(env));
    const offer = body.match(/<li[^>]*data-cut="v4"[^>]*>[\s\S]*?<\/li>/)?.[0] ?? "";
    expect(offer).toContain('class="cut offer"');
    expect(offer).toContain(html(say("only-the-words-went")));
    expect(offer).toContain(`>${html(say("cut-it-too"))}</button>`);
    // The offered line is shown without the cue's own words, which are already gone.
    expect(offer).toContain(html("“The second one is, um, I forget what it's called.”"));
    // The cue's own cut rides on its offer: no row of its own.
    expect(body).not.toMatch(/data-cut="v3"/);
  });

  it("gives a row that was put back Cut again, its words no longer struck", async () => {
    const edits = { ...EDITS, cuts: EDITS.cuts.map((c) => (c.id === "v2" ? { ...c, applied: false } : c)) };
    const env = space({ [`meta/${ID}.json`]: metaOf({ edits }) });
    const body = await page(env, await cookieFor(env));
    const row = body.match(/<li[^>]*data-cut="v2"[^>]*>[\s\S]*?<\/li>/)?.[0] ?? "";
    expect(row).toContain('class="cut"');
    expect(row).toContain(`>${html(say("cut-again"))}</button>`);
  });

  it("leaves hand cuts to the editor: no row for them", async () => {
    const env = space();
    const body = await page(env, await cookieFor(env));
    expect(body).not.toMatch(/data-cut="m1"/);
  });

  it("shows no strip when the take has no voice cuts, or no edit list", async () => {
    for (const edits of [undefined, { ...EDITS, voice: { planned_at: NOW, cues: 0 }, cuts: [EDITS.cuts[4]] }]) {
      const env = space({ [`meta/${ID}.json`]: metaOf({ edits }) });
      const body = await page(env, await cookieFor(env));
      expect(body).not.toContain('class="cuts"');
      expect(body).not.toContain(html(say("cut-where-you-said")));
    }
  });

  it("gives the maker the strip on their own video, as the title (N40)", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ owner: MHER }) });
    env.BUCKET.touch(ledgerKey(ID), JSON.stringify({ v: 1, member: MHER, device: MHER_MAC.id, at: "2026-10-02T11:00:00Z" }), { m: MHER });
    const body = await page(env, await cookieFor(env, memberGrant()));
    expect(body).toContain(html(say("cut-where-you-said")));
  });
});

describe("viewers get the cut video, and nothing else", () => {
  it("a recipient's page carries neither the strip nor any word that was cut", async () => {
    const body = await page(space(), null);
    expect(body).not.toContain('class="cuts"');
    expect(body).not.toContain(html(say("cut-where-you-said")));
    for (const words of WORDS_CUT) expect(body).not.toContain(html(words));
    expect(body).not.toContain("resumes");
  });

  it("nor does a member's page for a video they didn't make, nor the embed", async () => {
    const env = space();
    const member = await page(env, await cookieFor(env, memberGrant()));
    const embed = await (await req(env, `/${ID}?embed=1`)).text();
    for (const body of [member, embed]) {
      expect(body).not.toContain('class="cuts"');
      for (const words of WORDS_CUT) expect(body).not.toContain(html(words));
    }
  });

  it("the page's status never names a word that was cut", () => {
    const s = JSON.stringify(status(JSON.parse(metaOf()), NOW));
    for (const words of WORDS_CUT) expect(s).not.toContain(words);
  });
});

describe("a press saves at once (POST /_shroom/edits/<id>)", () => {
  it("Put back flips that cut alone, keeps the rest of the record, and answers with what now plays", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const res = await post(env, `/_shroom/edits/${ID}`, { cut: "v1", applied: false }, cookie);
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toContain("no-store");
    const answer = await res.json();
    expect(answer).toMatchObject({ cut: "v1", applied: false });
    // What plays: the trim minus the applied cuts, overlaps merged (the edit list's keep()).
    expect(answer.keep).toEqual([[0, 72], [76.1, 100], [106, 165], [167, 272]]);
    const meta = stored(env);
    expect(meta.edits.cuts.find((c) => c.id === "v1").applied).toBe(false);
    expect(meta.edits.cuts.filter((c) => c.id !== "v1")).toEqual(EDITS.cuts.filter((c) => c.id !== "v1"));
    // A press is a save: the revision moves on, so a copy cut from the list before reads as stale.
    expect(meta.edits.rev).toBe(EDITS.rev + 1);
    expect(meta.edits.voice).toEqual(EDITS.voice);
    expect(meta).toMatchObject({ title: "Setting up shroom", state: "live", owner: "m1" });
  });

  it("Cut it too applies the offered line, and Cut again puts a cut back in", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect((await post(env, `/_shroom/edits/${ID}`, { cut: "v4", applied: true }, cookie)).status).toBe(200);
    expect(stored(env).edits.cuts.find((c) => c.id === "v4").applied).toBe(true);
    expect((await post(env, `/_shroom/edits/${ID}`, { cut: "v1", applied: false }, cookie)).status).toBe(200);
    expect((await post(env, `/_shroom/edits/${ID}`, { cut: "v1", applied: true }, cookie)).status).toBe(200);
    expect(stored(env).edits.cuts.find((c) => c.id === "v1").applied).toBe(true);
  });

  it("retries from a fresh read when the Mac rewrote the record meanwhile — never clobbers it", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const realGet = env.BUCKET.get.bind(env.BUCKET);
    let raced = false;
    env.BUCKET.get = async (key, o) => {
      const r = await realGet(key, o);
      if (key === `meta/${ID}.json` && !raced) {
        raced = true; // the Mac's finish lands video.mp4 between the Worker's read and its write
        env.BUCKET.touch(key, metaOf({ media: { hls: `rec/${ID}/master.m3u8`, mp4: `rec/${ID}/video.mp4`, width: 1920, height: 1080 } }));
      }
      return r;
    };
    expect((await post(env, `/_shroom/edits/${ID}`, { cut: "v2", applied: false }, cookie)).status).toBe(200);
    const meta = stored(env);
    expect(meta.media.mp4).toBe(`rec/${ID}/video.mp4`);
    expect(meta.edits.cuts.find((c) => c.id === "v2").applied).toBe(false);
  });

  it("refuses a stranger, another site, a member's press on someone else's video, a hand cut, an unknown cut and a removed video", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const press = (body, c = cookie, origin) => post(env, `/_shroom/edits/${ID}`, body, c, origin);
    expect((await press({ cut: "v1", applied: false }, null)).status).toBe(403);
    expect((await press({ cut: "v1", applied: false }, cookie, "https://evil.example")).status).toBe(403);
    expect((await press({ cut: "v1", applied: false }, await cookieFor(env, memberGrant()))).status).toBe(403);
    // The strip only ever presses voice cuts; hand cuts are the editor's.
    expect((await press({ cut: "m1", applied: false })).status).toBe(403);
    expect((await press({ cut: "v9", applied: false })).status).toBe(404);
    expect((await press({ cut: "v1", applied: "no" })).status).toBe(400);
    expect((await press({ applied: false })).status).toBe(400);
    expect(stored(env).edits).toEqual(EDITS);
    env.BUCKET.touch(`meta/${ID}.json`, JSON.stringify({ schema: 1, id: ID, state: "deleted" }));
    expect((await press({ cut: "v1", applied: false })).status).toBe(409);
  });

  it("finds nothing to press on a take with no edit list", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ edits: undefined }) });
    expect((await post(env, `/_shroom/edits/${ID}`, { cut: "v1", applied: false }, await cookieFor(env))).status).toBe(404);
  });
});

describe("a page already open when the cuts land (the edge owner-page → owner-page-cuts)", () => {
  it("the status says cuts are coming while a space that listens hears a fresh take, and names the plan once it lands", () => {
    const fresh = (over = {}) => JSON.parse(metaOf({ publishedAt: new Date(NOW - 10_000).toISOString(), edits: undefined, ...over }));
    expect(status(fresh(), NOW, true)).toMatchObject({ cuts: "coming" });
    // An edit saved before the plan lands doesn't end the wait; the plan's mark does.
    expect(status(fresh({ edits: { ...EDITS, voice: undefined, cuts: [EDITS.cuts[4]] } }), NOW, true)).toMatchObject({ cuts: "coming" });
    // A space that doesn't listen never makes a page wait.
    expect(status(fresh(), NOW, false).cuts).toBeUndefined();
    expect(status(fresh(), NOW).cuts).toBeUndefined();
    // A Mac that went quiet stops the wait five minutes after the go-live.
    expect(status(fresh({ publishedAt: new Date(NOW - 6 * 60_000).toISOString() }), NOW, true).cuts).toBeUndefined();
    // Words left for tomorrow's free day (N66) don't keep a page polling.
    expect(status(fresh({ naming: { later: "daily", at: new Date(NOW).toISOString() } }), NOW, true).cuts).toBeUndefined();
    // A plan with no cues ends the wait and shows nothing.
    expect(status(fresh({ edits: { ...EDITS, voice: { planned_at: NOW, cues: 0 }, cuts: [] } }), NOW, true).cuts).toBeUndefined();
    const planned = status(JSON.parse(metaOf()), NOW, true);
    expect(status(JSON.parse(metaOf()), NOW).cuts).toBe(planned.cuts);
    expect(planned.cuts).toMatch(/^[0-9a-z]+$/);
    // Any press changes it, so the page knows to fetch the strip again.
    const pressed = JSON.parse(metaOf());
    pressed.edits.cuts[0].applied = false;
    expect(status(pressed, NOW).cuts).not.toBe(planned.cuts);
  });

  it("the owner fetches the strip and what plays (GET /_shroom/edits/<id>); nobody else does", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const res = await req(env, `/_shroom/edits/${ID}`, { headers: { cookie } });
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toContain("no-store");
    const got = await res.json();
    expect(got.strip).toContain(html(say("cut-where-you-said")));
    expect(got.strip).toContain('data-cut="v1"');
    expect(got.keep).toEqual([[0, 4.1], [7.5, 72], [76.1, 100], [106, 165], [167, 272]]);
    expect((await req(env, `/_shroom/edits/${ID}`)).status).toBe(403);
    expect((await req(env, `/_shroom/edits/${ID}`, { headers: { cookie: await cookieFor(env, memberGrant()) } })).status).toBe(403);
  });

  it("the page polls on while cuts are coming, and swaps the strip in when they land", async () => {
    const fresh = metaOf({ publishedAt: new Date(NOW - 5_000).toISOString(), edits: undefined });
    const env = space({ [`meta/${ID}.json`]: fresh }, true);
    const body = await page(env, await cookieFor(env));
    // An empty strip waits under the player for the page's poll to fill it.
    expect(body).toContain('<section class="cuts" id="cuts" hidden');
    expect(body).toContain("/_shroom/edits/");
    // The status route gives the page the same word.
    expect(await (await req(env, `/_shroom/status/${ID}`)).json()).toMatchObject({ cuts: "coming" });
    // Without the space listening, the page doesn't wait.
    const quiet = space({ [`meta/${ID}.json`]: fresh });
    expect(await page(quiet, await cookieFor(quiet))).not.toContain('id="cuts"');
  });
});

// The edit the link plays (N63, DECISIONS B195): the Mac's editor saves an edit list in the
// video's record (`meta.edits`) and the page, the embed and the owner page apply it as they
// play — held to the trim, every cut skipped, the edited length shown — until the Mac's
// permanent copy of the edit is up; then they play the copy as it is, Download hands it
// over, and the original is out of a viewer's reach. Only the ranges that play and the
// copy's own names ever leave the Worker: never the transcript text a voice cut carries.
//
// `editsOf` and the player's own arithmetic are held to crates/shroom-edit's vectors, the
// truth the editor's webview is held to as well. What a browser does with all this is
// driven for real by `npm run drive:page` (an edited record in Chromium and WebKit).
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { COPY, EDIT_CUTS_MAX, PLAYER_JS, editsOf, forgetSecret, handle, status } from "../src/worker.js";
import { M1, NOW, ORIGIN, OWNER_AIR, REC, cookieFor, metaKey, metaOf, space } from "./space-fixture.js";

const VECTORS = JSON.parse(readFileSync(new URL("../../crates/shroom-edit/tests/keep-vectors.json", import.meta.url), "utf8"));
const { shroomLength, shroomAt, shroomToSource, shroomGain } = new Function(`${PLAYER_JS};return { shroomLength, shroomAt, shroomToSource, shroomGain }`)();
/** The vectors' rounding (crates/shroom-edit/tests/vectors.rs). */
const r6 = (x) => Math.round(x * 1e6) / 1e6;
const ranges = (keep) => keep.map(([s, e]) => [r6(s), r6(e)]);

const get = (env, path, init) => handle(new Request(`${ORIGIN}${path}`, init), env, NOW);
const text = async (env, path, init) => (await get(env, path, init)).text();
const owner = { headers: { cookie: cookieFor(M1, OWNER_AIR) } };

// The transcript text a later voice cut carries: it must never reach a viewer.
const SAID = ["THE-REMOVED-WORDS", "THE-WORDS-THAT-RESUME", "THE-PLANNER-SAYS-WHY", "THE-SUGGESTION-WORDS"];
const LIST = {
  version: 1, source_duration: 272.4, in: 8, out: 241,
  cuts: [
    { start: 150, end: 162, kind: "retake", confidence: "high", applied: true, removed: SAID[0], resumes: SAID[1], why: SAID[2] },
    { start: 20, end: 30, kind: "line-before-cue", confidence: "low", applied: false, removed: SAID[3] },
  ],
};
/** What plays of LIST: 8–150 and 162–241, 221 s (3:41). */
const KEEP = [[8, 150], [162, 241]];
const COPY3 = { rev: 3, file: "edit-3.mp4", duration: 220.9, poster: "edit-3-poster.jpg", teaser: "edit-3-teaser.mp4" };
const MEDIA = { hls: `rec/${REC}/master.m3u8`, mp4: `rec/${REC}/video.mp4`, poster: `rec/${REC}/poster.jpg`, teaser: `rec/${REC}/teaser.mp4`, width: 1920, height: 1080 };
const ONE_MAP = "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:6.0,\nseg_00000.m4s\n#EXT-X-ENDLIST\n";
/** Every file a recording and its copy have in the bucket, each its own name as its bytes. */
const FILES = ["master.m3u8", "v0/stream.m3u8", "v0/init.mp4", "v0/seg_00000.m4s", "video.mp4", "poster.jpg", "teaser.mp4", "thumb-0123456789abcdef.gif",
  "captions.vtt", "edit-3.mp4", "edit-3-poster.jpg", "edit-3-teaser.mp4", "edit-2.mp4"];
const files = () => Object.fromEntries(FILES.map((f) => [`rec/${REC}/${f}`, f === "v0/stream.m3u8" ? ONE_MAP : f]));
const record = (edits, over = {}) => metaOf(REC, { title: "Where the 502s come from", titled: true, durationSec: 272.4, media: MEDIA, ...(edits === undefined ? {} : { edits }), ...over });
const withEdit = (edits, over) => space({ objects: { [metaKey(REC)]: record(edits, over), ...files() } });
const edited = (over) => withEdit({ ...LIST, rev: 3 }, over);
const copied = (copy = COPY3, over) => withEdit({ ...LIST, rev: 3, copy }, over);
const stale = () => withEdit({ ...LIST, rev: 4, copy: COPY3 });
/** The player's last argument on a page: what it applies. */
const playerArg = (body) => JSON.parse(body.match(/shroomPlayer\(document\.querySelector\("\.player"\),"[^"]*",\{[^}]*\},(\{.*?\})\)/)?.[1] ?? "null");
const playerSrc = (body) => body.match(/shroomPlayer\(document\.querySelector\("\.player"\),"([^"]*)"/)?.[1] ?? body.match(/src="([^"]*)",tb=/)?.[1] ?? null;

describe("editsOf: the record's edit, as the editor's own list plays it (crates/shroom-edit keep-vectors.json)", () => {
  for (const v of VECTORS) {
    it(v.name, () => {
      const got = editsOf({ durationSec: v.list.source_duration, edits: { ...v.list, rev: 1 } }, REC);
      if (v.expect.identity) {
        expect(got).toBeNull();
        return;
      }
      expect(ranges(got.keep)).toEqual(v.expect.keep);
      expect(r6(got.dur)).toBe(v.expect.edited_duration);
      expect(got.copy).toBeNull();
    });
  }

  it("trusts the record's length over the list's", () => {
    // The list says 300 s; the recording is 200: the trim's out and a cut past it are held to 200.
    const got = editsOf({ durationSec: 200, edits: { version: 1, source_duration: 300, in: 10, out: 290, cuts: [{ start: 190, end: 250, applied: true }], rev: 1 } }, REC);
    expect(got.keep).toEqual([[10, 190]]);
    // A record with no length of its own falls back to the list's.
    expect(editsOf({ durationSec: null, edits: { source_duration: 60, in: 5, out: 60, cuts: [] } }, REC).keep).toEqual([[5, 60]]);
    expect(editsOf({ edits: { in: 5, out: 60, cuts: [] } }, REC)).toBeNull();
  });
});

describe("the player's own arithmetic, held to the same vectors", () => {
  for (const v of VECTORS) {
    it(v.name, () => {
      const K = v.expect.keep;
      expect(r6(shroomLength(K))).toBe(v.expect.edited_duration);
      v.at_source.forEach((t, i) => {
        const want = v.expect.to_edited[i];
        // Inside a range, the edited moment; in a cut or outside the trim, where the next
        // range starts (the player is never shown there: it seeks on).
        const next = K.find(([s]) => s > t);
        expect(r6(shroomAt(K, t)), `to_edited(${t})`).toBe(want ?? r6(next ? shroomAt(K, next[0]) : shroomLength(K)));
      });
      v.at_edited.forEach((e, i) => expect(r6(shroomToSource(K, e)), `to_source(${e})`).toBe(v.expect.to_source[i]));
    });
  }
});

describe("editsOf takes nothing on trust", () => {
  const base = { durationSec: 60 };
  const of = (edits) => editsOf({ ...base, edits }, REC);

  it("reads no edit from what isn't one", () => {
    for (const edits of [undefined, null, "cut it", 42, true, [], {}, { cuts: "all of it" }, { in: 0, out: 60, cuts: [] }]) expect(of(edits), JSON.stringify(edits)).toBeNull();
    expect(editsOf(null)).toBeNull();
    expect(editsOf({ edits: { in: 0, out: 1, cuts: [] } }, REC)).toBeNull();
  });

  it("reads a value that isn't a number as one that isn't finite: the trim falls back to the whole", () => {
    for (const bad of ["8", null, {}, [], true, NaN, Infinity, -Infinity]) {
      expect(of({ in: bad, out: 50, cuts: [] })?.keep, String(bad)).toEqual([[0, 50]]);
      expect(of({ in: 10, out: bad, cuts: [] })?.keep, String(bad)).toEqual([[10, 60]]);
    }
  });

  it("drops a cut that isn't applied, isn't an object, or whose ends aren't numbers", () => {
    const cuts = [null, 7, "x", [], { start: 5, end: 9 }, { start: 5, end: 9, applied: "true" }, { start: 5, end: 9, applied: 1 },
      { start: "5", end: 9, applied: true }, { start: 5, end: NaN, applied: true }, { start: 9, end: 5, applied: true }, { start: 20, end: 25, applied: true }];
    expect(of({ in: 0, out: 60, cuts }).keep).toEqual([[0, 20], [25, 60]]);
  });

  it(`reads at most ${EDIT_CUTS_MAX} applied cuts, whatever the record holds — suggestions never count`, () => {
    const many = Array.from({ length: 100_000 }, (_, i) => ({ start: i * 0.0005, end: i * 0.0005 + 0.0001, applied: true }));
    const t0 = performance.now();
    const got = of({ in: 0, out: 60, cuts: many });
    expect(performance.now() - t0).toBeLessThan(200);
    expect(got.keep.length).toBeLessThanOrEqual(EDIT_CUTS_MAX + 1);
    // Ten thousand suggestions ahead of the three applied cuts don't push them out.
    const offered = Array.from({ length: 10_000 }, (_, i) => ({ start: i * 0.005, end: i * 0.005 + 0.004, applied: false }));
    expect(of({ in: 0, out: 60, cuts: [...offered, { start: 10, end: 20, applied: true }, { start: 30, end: 31, applied: true }, { start: 50, end: 55, applied: true }] }).keep)
      .toEqual([[0, 10], [20, 30], [31, 50], [55, 60]]);
  });

  it("names the permanent copy only once it's this revision's, under names of its own kind", () => {
    const list = { in: 0, out: 60, cuts: [{ start: 10, end: 20, applied: true }] };
    const copy = (c, rev = 3) => of({ ...list, rev, copy: c })?.copy;
    expect(copy({ rev: 3, file: "edit-3.mp4", duration: 50, poster: "edit-3-poster.jpg", teaser: "edit-3-teaser.mp4" }))
      .toEqual({ src: `/m/${REC}/edit-3.mp4`, dur: 50, poster: `/m/${REC}/edit-3-poster.jpg`, teaser: `/m/${REC}/edit-3-teaser.mp4` });
    // A stale copy, or none: the list plays over the original.
    expect(copy({ rev: 2, file: "edit-2.mp4", duration: 50 })).toBeNull();
    expect(copy({ rev: "3", file: "edit-3.mp4", duration: 50 })).toBeNull();
    expect(copy(undefined)).toBeNull();
    // A name that isn't the copy's kind is never served from the record.
    for (const file of ["video.mp4", "../video.mp4", "edit-3.mp4/../../meta/x.json", ".shroom/cookie.key", "edit-3-poster.jpg", "edit-.mp4", "edit-1234567890.mp4", "EDIT-3.mp4", 3]) {
      expect(copy({ rev: 3, file, duration: 50 }), String(file)).toBeNull();
    }
    expect(copy({ rev: 3, file: "edit-3.mp4", duration: 50, poster: "poster.jpg", teaser: "edit-3.mp4" })).toEqual({ src: `/m/${REC}/edit-3.mp4`, dur: 50 });
    // A copy whose length isn't a number takes the list's.
    expect(copy({ rev: 3, file: "edit-3.mp4", duration: "long" }).dur).toBe(50);
    // A record with no revision is revision 0 (serde's default).
    expect(of({ ...list, copy: { rev: 0, file: "edit-0.mp4", duration: 50 } }).copy?.src).toBe(`/m/${REC}/edit-0.mp4`);
    // No copy without the recording's own id to put it under.
    expect(editsOf({ ...base, edits: { ...list, rev: 3, copy: { rev: 3, file: "edit-3.mp4", duration: 50 } } }, "../x").copy).toBeNull();
  });

  it("hands out nothing but numbers and the copy's addresses", () => {
    const got = editsOf(JSON.parse(record({ ...LIST, rev: 3, copy: COPY3 })), REC);
    expect(Object.keys(got).sort()).toEqual(["copy", "dur", "keep"]);
    for (const s of SAID) expect(JSON.stringify(got)).not.toContain(s);
  });
});

describe("the page applies the edit until the copy is up (N63)", () => {
  it("plays the original held to the trim, every cut skipped, and shows the edited length", async () => {
    const body = await text(edited(), `/${REC}`);
    expect(playerArg(body)).toEqual({ keep: KEEP, len: 272.4, st: `/_shroom/status/${REC}`, copy: false });
    expect(body).toContain(`class="player" data-dur="221"`);
    expect(body).toMatch(/<input type="range" class="seek" min="0" max="221"/);
    expect(body).toContain(`<span class="time">0:00 / 3:41</span>`);
    // Over the single-file copy of the original (B177) as over the stream.
    expect(playerSrc(body)).toBe(`/m/${REC}/video.mp4`);
    expect(playerArg(await text(edited({ media: { ...MEDIA, mp4: null } }), `/${REC}`)).keep).toEqual(KEEP);
  });

  it("is the same on the embed and the owner page", async () => {
    const embed = await text(edited(), `/${REC}?embed=1`);
    expect(playerArg(embed)).toEqual({ keep: KEEP, len: 272.4, st: `/_shroom/status/${REC}`, copy: false });
    expect(embed).toContain(`<span class="time">0:00 / 3:41</span>`);
    const mine = await text(edited(), `/${REC}`, owner);
    expect(mine).toContain(`class="title"`);
    expect(mine).toContain(`shroomPlayer(document.querySelector(".player"),src,c.labels,{"keep":[[8,150],[162,241]],"len":272.4,"st":"/_shroom/status/${REC}","copy":false})`);
    expect(mine).toContain(`<span class="time">0:00 / 3:41</span>`);
    // Copy thumbnail's picture carries the edited length.
    expect(mine).toContain(`"len":"3:41"`);
  });

  it("offers no Download while the copy is on its way: a cut part never leaves as a file", async () => {
    const env = edited();
    expect(await text(env, `/${REC}`)).not.toContain(`href="/d/${REC}"`);
    const mine = await text(env, `/${REC}`, owner);
    expect(mine).toContain(`<span id="dl" data-tip="${COPY.download}"></span>`);
    expect(mine).toContain(`"mp4":false`);
    expect((await get(env, `/d/${REC}`)).status).toBe(404);
  });

  it("says the edited length in the unfurl and the status", async () => {
    const body = await text(edited(), `/${REC}`);
    expect(body).toContain(`<meta property="og:video:duration" content="221">`);
    expect(body).toContain(`<meta property="og:image" content="${ORIGIN}/m/${REC}/poster.jpg">`);
    expect(body).toContain(`<meta property="og:video" content="${ORIGIN}/m/${REC}/teaser.mp4">`);
    expect(JSON.parse(await text(edited(), `/_shroom/status/${REC}`))).toEqual({ state: "live", poster: true, mp4: false, teaser: true, thumb: null, dur: 221, drained: true, keep: KEEP });
  });

  it("leaves the original reachable at its address until the copy is up", async () => {
    for (const f of ["master.m3u8", "v0/seg_00000.m4s", "video.mp4", "teaser.mp4", "poster.jpg"]) expect((await get(edited(), `/m/${REC}/${f}`)).status, f).toBe(200);
  });

  it("an edit that leaves everything in is no edit", async () => {
    const env = withEdit({ ...LIST, in: 0, out: 272.4, cuts: [{ ...LIST.cuts[0], applied: false }], rev: 2, copy: { ...COPY3, rev: 2 } });
    const body = await text(env, `/${REC}`);
    expect(playerArg(body).keep).toBeNull();
    expect(body).toContain(`<span class="time">0:00 / 4:32</span>`);
    expect(body).toContain(`href="/d/${REC}"`);
    expect((await get(env, `/m/${REC}/master.m3u8`)).status).toBe(200);
  });

  it("a record whose edit makes no sense plays as recorded", async () => {
    for (const edits of ["garbage", { in: "x", out: null, cuts: "nope" }, [1, 2, 3], { in: 0, out: 272.4, cuts: [{ start: "a", end: {}, applied: true }] }]) {
      const res = await get(withEdit(edits), `/${REC}`);
      expect(res.status).toBe(200);
      const body = await res.text();
      expect(playerArg(body).keep, JSON.stringify(edits)).toBeNull();
      expect(body).toContain(`<span class="time">0:00 / 4:32</span>`);
    }
  });

  it("a record holding a huge list still renders, carrying no more than the cap", async () => {
    const cuts = Array.from({ length: 20_000 }, (_, i) => ({ start: 1 + i * 0.01, end: 1 + i * 0.01 + 0.005, kind: "manual", confidence: "high", applied: true }));
    const res = await get(withEdit({ ...LIST, in: 0, out: 272.4, cuts, rev: 1 }), `/${REC}`);
    expect(res.status).toBe(200);
    expect(playerArg(await res.text()).keep.length).toBeLessThanOrEqual(EDIT_CUTS_MAX + 1);
  });

  it("costs no read of its own: the edit rides the record the page already read", async () => {
    const plain = space({ objects: { [metaKey(REC)]: record(undefined), ...files() } });
    for (const env of [edited(), copied()]) {
      for (const [path, init] of [[`/${REC}`], [`/${REC}?embed=1`], [`/${REC}`, owner], [`/_shroom/status/${REC}`], [`/oembed?url=${encodeURIComponent(`${ORIGIN}/${REC}`)}`]]) {
        await get(plain, path, init);
        await get(env, path, init);
        expect(env.BUCKET.read(), path).toEqual(plain.BUCKET.read());
        plain.BUCKET.ops.length = 0;
        env.BUCKET.ops.length = 0;
      }
    }
  });
});

describe("once the permanent copy is up, the page plays it as it is (N63)", () => {
  it("plays the copy with nothing to skip, its own first frame and its own length", async () => {
    const body = await text(copied(), `/${REC}`);
    expect(playerSrc(body)).toBe(`/m/${REC}/edit-3.mp4`);
    expect(playerArg(body)).toEqual({ keep: null, len: 272.4, st: `/_shroom/status/${REC}`, copy: true });
    expect(body.match(/<video[^>]*>/)?.[0]).toContain(`poster="/m/${REC}/edit-3-poster.jpg"`);
    expect(body).toContain(`class="player" data-dur="220.9"`);
    expect(body).toContain(`<span class="time">0:00 / 3:41</span>`);
    const embed = await text(copied(), `/${REC}?embed=1`);
    expect(playerSrc(embed)).toBe(`/m/${REC}/edit-3.mp4`);
    expect(embed.match(/<video[^>]*>/)?.[0]).toContain(`poster="${ORIGIN}/m/${REC}/edit-3-poster.jpg"`);
    const mine = await text(copied(), `/${REC}`, owner);
    expect(mine).toContain(`src="/m/${REC}/edit-3.mp4"`);
    expect(mine).toContain(`shroomPlayer(document.querySelector(".player"),src,c.labels,{"keep":null,"len":272.4,"st":"/_shroom/status/${REC}","copy":true})`);
    // Copy thumbnail draws from the copy's own pictures.
    expect(mine).toContain(`"psrc":"/m/${REC}/edit-3-poster.jpg","tsrc":"/m/${REC}/edit-3-teaser.mp4"`);
  });

  it("offers Download, which hands over the copy named after the title", async () => {
    const env = copied();
    expect(await text(env, `/${REC}`)).toContain(`<a href="/d/${REC}">${COPY.download}</a>`);
    expect(await text(env, `/${REC}`, owner)).toContain(`"mp4":true`);
    const res = await get(env, `/d/${REC}`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-disposition")).toContain(`filename*=UTF-8''Where%20the%20502s%20come%20from.mp4`);
    expect(await res.text()).toBe("edit-3.mp4");
    const part = await get(env, `/d/${REC}`, { headers: { range: "bytes=0-3" } });
    expect(part.status).toBe(206);
    expect(await part.text()).toBe("edit");
  });

  it("unfurls with the copy's own pictures and length", async () => {
    const body = await text(copied(), `/${REC}`);
    expect(body).toContain(`<meta property="og:image" content="${ORIGIN}/m/${REC}/edit-3-poster.jpg">`);
    expect(body).toContain(`<meta property="og:video" content="${ORIGIN}/m/${REC}/edit-3-teaser.mp4">`);
    expect(body).toContain(`<meta property="og:video:duration" content="221">`);
    expect(body).toContain(`<meta name="twitter:image" content="${ORIGIN}/m/${REC}/edit-3-poster.jpg">`);
    expect(body).toContain(`<meta name="twitter:player:stream" content="${ORIGIN}/m/${REC}/edit-3-teaser.mp4">`);
    expect(body).not.toContain(`/m/${REC}/teaser.mp4`);
    const oembed = JSON.parse(await text(copied(), `/oembed?url=${encodeURIComponent(`${ORIGIN}/${REC}`)}`));
    expect(oembed.thumbnail_url).toBe(`${ORIGIN}/m/${REC}/edit-3-poster.jpg`);
  });

  it("a copy with no pictures of its own shows the original's first frame and no teaser", async () => {
    const env = copied({ rev: 3, file: "edit-3.mp4", duration: 220.9 });
    const body = await text(env, `/${REC}`);
    expect(body.match(/<video[^>]*>/)?.[0]).toContain(`poster="/m/${REC}/poster.jpg"`);
    expect(body).toContain(`<meta property="og:image" content="${ORIGIN}/m/${REC}/poster.jpg">`);
    expect(body).not.toContain("og:video");
    expect(JSON.parse(await text(env, `/_shroom/status/${REC}`))).toMatchObject({ teaser: false, poster: true, copy: { src: `/m/${REC}/edit-3.mp4`, dur: 220.9, poster: null } });
  });

  it("tells the status where the copy is, so an open player can move over to it", async () => {
    expect(JSON.parse(await text(copied(), `/_shroom/status/${REC}`))).toEqual({ state: "live", poster: true, mp4: true, teaser: true, thumb: null, dur: 220.9, drained: true,
      keep: KEEP, copy: { src: `/m/${REC}/edit-3.mp4`, dur: 220.9, poster: `/m/${REC}/edit-3-poster.jpg` } });
    // The player asks it when its media fails, and swaps to the copy at the same edited moment.
    expect(PLAYER_JS).toMatch(/v\.addEventListener\("error",fail\)/);
    // hls.js keeps retrying a piece that answers 404 (not fatal for a while): the 404 itself asks.
    expect(PLAYER_JS).toContain(`h.on(Hls.Events.ERROR,function(e,d){if(d&&(d.fatal||d.response&&d.response.code===404))f()})`);
  });

  it("a stale copy — a newer save came first — is ignored by every reader", async () => {
    const env = stale();
    const body = await text(env, `/${REC}`);
    expect(playerSrc(body)).toBe(`/m/${REC}/video.mp4`);
    expect(playerArg(body).keep).toEqual(KEEP);
    expect(body).not.toContain("edit-3");
    expect((await get(env, `/d/${REC}`)).status).toBe(404);
    expect(JSON.parse(await text(env, `/_shroom/status/${REC}`)).copy).toBeUndefined();
    expect((await get(env, `/m/${REC}/master.m3u8`)).status).toBe(200);
  });
});

describe("step 3: once the copy is up, the cut parts are out of reach (N63)", () => {
  const SERVED = ["poster.jpg", "edit-3.mp4", "edit-3-poster.jpg", "edit-3-teaser.mp4", "thumb-0123456789abcdef.gif"];
  const GONE = ["master.m3u8", "v0/stream.m3u8", "v0/init.mp4", "v0/seg_00000.m4s", "video.mp4", "teaser.mp4", "captions.vtt", "edit-2.mp4"];

  it("serves only the copy's files, the original's first frame and Copy thumbnail's pictures", async () => {
    const env = copied();
    for (const f of SERVED) {
      const res = await get(env, `/m/${REC}/${f}`);
      expect(res.status, f).toBe(200);
      expect(await res.text()).toBe(f);
    }
    for (const f of GONE) {
      for (const init of [{}, { method: "HEAD" }, { headers: { range: "bytes=0-0" } }]) {
        const res = await get(env, `/m/${REC}/${f}`, init);
        expect(res.status, `${init.method ?? "GET"} ${f}`).toBe(404);
        expect(await res.text()).toBe("");
      }
    }
    expect((await get(env, `/m/${REC}/edit-3.mp4`, { headers: { range: "bytes=0-3" } })).status).toBe(206);
  });

  it("opens again once a newer save makes the copy stale, and when the edit goes", async () => {
    expect((await get(stale(), `/m/${REC}/video.mp4`)).status).toBe(200);
    expect((await get(space({ objects: { [metaKey(REC)]: record(undefined), ...files() } }), `/m/${REC}/video.mp4`)).status).toBe(200);
  });

  it("still answers a tombstone and an unreadable record as before", async () => {
    const env = withEdit({ ...LIST, rev: 3, copy: COPY3 }, { state: "deleted" });
    expect((await get(env, `/m/${REC}/edit-3.mp4`)).status).toBe(404);
    const live = copied();
    const real = live.BUCKET.get;
    live.BUCKET.get = (k, o) => (k === metaKey(REC) ? Promise.reject(new Error("storage")) : real(k, o));
    expect((await get(live, `/m/${REC}/edit-3.mp4`)).status).toBe(503);
  });
});

describe("the Worker's own rewrites keep the edit as the Mac wrote it", () => {
  it("a title save leaves meta.edits whole, transcript and all", async () => {
    forgetSecret();
    const env = copied();
    const res = await get(env, `/_shroom/title/${REC}`, { method: "POST", headers: { origin: ORIGIN, cookie: cookieFor(M1, OWNER_AIR), "content-type": "application/json" }, body: JSON.stringify({ title: "A new title" }) });
    expect(res.status).toBe(200);
    expect(env.BUCKET.json(metaKey(REC)).edits).toEqual({ ...LIST, rev: 3, copy: COPY3 });
  });
});

describe("the transcript a voice cut carries never reaches a viewer", () => {
  it("not in the page, the embed, the owner page, the status, oEmbed or a download", async () => {
    for (const make of [edited, copied, stale]) {
      const env = make();
      const seen = [
        await text(env, `/${REC}`), await text(env, `/${REC}?embed=1`), await text(env, `/${REC}`, owner),
        await text(env, `/_shroom/status/${REC}`), await text(env, `/oembed?url=${encodeURIComponent(`${ORIGIN}/${REC}`)}`),
        JSON.stringify(Object.fromEntries((await get(env, `/d/${REC}`)).headers)),
      ].join("\n");
      for (const s of SAID) expect(seen, `${make.name}: ${s}`).not.toContain(s);
      // Nor the list itself: only the ranges that play.
      expect(seen).not.toMatch(/"(removed|resumes|why|cuts|applied|confidence|kind|source_duration)":|line-before-cue|retake/);
    }
    for (const s of SAID) expect(JSON.stringify(status(JSON.parse(record({ ...LIST, rev: 3, copy: COPY3 })), NOW, REC))).not.toContain(s);
  });
});

/** The player against a stand-in browser: a video element whose clock the test moves, the
 * frames the player asks for run by hand, and the status it asks when its media fails. */
function stage(keep, { dur = 20, answer = null, len = dur, copy = false } = {}) {
  const node = () => ({ attrs: {}, on: {}, textContent: "", value: "0", max: "0", style: { setProperty() {} }, classList: { toggle() {}, add() {}, remove() {}, contains: () => false },
    setAttribute(k, v) { this.attrs[k] = String(v); }, getAttribute(k) { return this.attrs[k] ?? null; }, addEventListener(e, f) { (this.on[e] ??= []).push(f); } });
  const emit = (n, e, ev = {}) => (n.on[e] ?? []).forEach((f) => f({ type: e, target: { tagName: "DIV" }, preventDefault() {}, ...ev }));
  const video = { ...node(), currentTime: 0, paused: true, ended: false, duration: dur, buffered: { length: 0 }, playbackRate: 1, muted: false, src: "", poster: "", played: 0,
    canPlayType: () => "", play() { this.played++; if (this.paused) { this.paused = false; emit(this, "play"); } return Promise.resolve(); },
    pause() { if (!this.paused) { this.paused = true; emit(this, "pause"); } } };
  const parts = { video, ".pp": node(), ".seek": node(), ".time": node(), ".fs": node(), ".sp": node(), ".mu": node() };
  const p = { ...node(), querySelector: (q) => parts[q], contains: () => false };
  p.attrs["data-dur"] = String(keep ? keep.reduce((n, [s, e]) => n + e - s, 0) : dur);
  let frame = null;
  const asked = [];
  const docOn = {};
  const document = { addEventListener(e, f) { (docOn[e] ??= []).push(f); }, createElement: () => ({}), head: { appendChild() {} }, activeElement: null, fullscreenElement: null };
  const fetch = (url) => { asked.push(url); return Promise.resolve({ json: () => Promise.resolve(answer) }); };
  const shroomPlayer = new Function("document", "requestAnimationFrame", "fetch", "setTimeout", "clearTimeout", `${PLAYER_JS};return shroomPlayer`)(
    document, (f) => { frame = f; return 1; }, fetch, () => 0, () => {});
  shroomPlayer(p, "/m/x/video.mp4", { play: "Play", pause: "Pause", full: "", exit: "", speed: "", mute: "", unmute: "" }, { keep, len, st: "/_shroom/status/x", copy });
  return {
    /** A `shroom:keep` event on the document, as the voice-cut build sends it (#59). */
    keepEvent: (detail) => (docOn["shroom:keep"] ?? []).forEach((f) => f({ type: "shroom:keep", detail })),
    /** The player has asked for a frame. */
    framing: () => frame !== null,
    video, asked, time: () => parts[".time"].textContent, seek: parts[".seek"],
    /** The clock reaches `t` while playing: the frame the player asked for runs, then `timeupdate`. */
    at(t) { video.currentTime = t; const f = frame; frame = null; if (f) f(); emit(video, "timeupdate"); },
    /** Seeking from elsewhere (a phone's own controls). */
    seekTo(t) { video.currentTime = t; emit(video, "seeking"); },
    click: () => emit(parts[".pp"], "click"),
    key: (key) => emit(p, "keydown", { key }),
    drag(value) { parts[".seek"].value = String(value); emit(parts[".seek"], "input"); emit(parts[".seek"], "change"); },
    emit: (e) => emit(video, e),
  };
}

describe("the player applies the edit as it plays (the skip, the time, the seek bar, the keys)", () => {
  // 2–6, 9–12 and 13.5–18 of a 20 s recording: 11.5 s, a length that reads 0:12.
  const K = [[2, 6], [9, 12], [13.5, 18]];

  it("starts at the trim's in, skips each cut on the next frame, and stops at the out, showing it ended", () => {
    const s = stage(K);
    expect(s.time()).toBe("0:00 / 0:12");
    s.click();
    expect(s.video.paused).toBe(false);
    expect(s.video.currentTime).toBe(2);
    s.at(4);
    expect(s.time()).toBe("0:02 / 0:12");
    s.at(5.98);
    expect(s.video.currentTime).toBe(9);
    s.at(10.5);
    expect(s.time()).toBe("0:05 / 0:12");
    s.at(11.975);
    expect(s.video.currentTime).toBe(13.5);
    s.at(17.98);
    expect(s.video.paused).toBe(true);
    expect(s.time()).toBe("0:12 / 0:12");
    // Play again: from the edit's start.
    s.click();
    expect(s.video.currentTime).toBe(2);
    expect(s.video.paused).toBe(false);
  });

  it("skips on timeupdate too (a background tab runs no frames), and from a play begun elsewhere", () => {
    const s = stage(K);
    s.video.play();
    expect(s.video.currentTime).toBe(2);
    s.video.currentTime = 6.1;
    s.emit("timeupdate");
    expect(s.video.currentTime).toBe(9);
  });

  it("moves on at once past a range shorter than its lead: the last seek's target holds back only itself", () => {
    // Cuts 10–20 and 20.01–30 leave 10 ms between them, less than the skip's 30 ms lead:
    // landing there, the range after it is already due (the review's case held at 20.03).
    const s = stage([[0, 10], [20, 20.01], [30, 40]], { dur: 40 });
    s.click();
    s.at(9.98);
    expect(s.video.currentTime).toBe(30);
    // A playhead just short of the start it was sent to is left to land there: no seek loop.
    const t = stage([[0, 10], [20, 30]], { dur: 40 });
    t.click();
    t.at(9.98);
    expect(t.video.currentTime).toBe(20);
    t.at(19.995);
    expect(t.video.currentTime).toBe(19.995);
  });

  it("puts a moment a seek lands in a cut at the next range's start, and one past the out at the end", () => {
    const s = stage(K);
    s.click();
    s.seekTo(7.2);
    expect(s.video.currentTime).toBe(9);
    s.seekTo(19);
    expect(s.video.paused).toBe(true);
    expect(s.time()).toBe("0:12 / 0:12");
  });

  it("counts the seek bar and ←/→, Home and End in edited time", () => {
    const s = stage(K);
    expect(s.seek.max).toBe("11.5");
    s.drag(6);
    expect(s.video.currentTime).toBe(11);
    expect(s.time()).toBe("0:06 / 0:12");
    s.key("ArrowRight");
    expect(s.video.currentTime).toBe(17.5);
    s.key("ArrowLeft");
    expect(s.video.currentTime).toBe(11);
    s.key("Home");
    expect(s.video.currentTime).toBe(2);
    s.key("ArrowLeft");
    expect(s.video.currentTime).toBe(2);
    s.key("End");
    expect(s.video.currentTime).toBe(18);
    expect(s.time()).toBe("0:12 / 0:12");
  });

  it("plays a recording with no edit as it always has", () => {
    const s = stage(null);
    expect(s.time()).toBe("0:00 / 0:20");
    s.click();
    expect(s.video.currentTime).toBe(0);
    s.at(7.2);
    expect(s.video.currentTime).toBe(7.2);
    s.key("ArrowRight");
    expect(s.video.currentTime).toBe(12.2);
  });

  it("moves over to the copy at the same edited moment when the original goes out of reach", async () => {
    const copy = { src: "/m/x/edit-3.mp4", dur: 11.4, poster: "/m/x/edit-3-poster.jpg" };
    const s = stage(K, { answer: { state: "live", copy, keep: K } });
    s.click();
    s.at(10.5);
    s.emit("error");
    await new Promise((ok) => setImmediate(ok));
    expect(s.asked).toEqual(["/_shroom/status/x"]);
    expect(s.video.src).toBe(copy.src);
    expect(s.video.poster).toBe(copy.poster);
    s.video.duration = 11.4;
    const played = s.video.played;
    s.emit("loadedmetadata");
    expect(s.video.currentTime).toBe(5.5);
    expect(s.video.played).toBe(played + 1);
    // The copy is the edit: nothing left to skip.
    s.at(9.5);
    expect(s.video.currentTime).toBe(9.5);
    expect(s.time()).toBe("0:09 / 0:11");
  });

  it("moving from a copy to a newer one keeps its moment: the status's ranges count on the recording's clock, not the copy's", async () => {
    // Playing revision 3's copy, 60 s in; revision 4's copy lands and 3's goes.
    const keep4 = [[8, 150], [162, 231]];
    const s = stage(null, { dur: 220, copy: true, answer: { state: "live", keep: keep4, copy: { src: "/m/x/edit-4.mp4", dur: 211 } } });
    s.click();
    s.at(60);
    s.emit("error");
    await new Promise((ok) => setImmediate(ok));
    expect(s.video.src).toBe("/m/x/edit-4.mp4");
    s.emit("loadedmetadata");
    expect(s.video.currentTime).toBe(60);
    // A player on the recording itself is on that clock: its moment maps through the ranges.
    const o = stage(null, { answer: { state: "live", keep: [[2, 6], [9, 12]], copy: { src: "/m/x/edit-3.mp4", dur: 7 } } });
    o.click();
    o.at(10);
    o.emit("error");
    await new Promise((ok) => setImmediate(ok));
    o.emit("loadedmetadata");
    expect(o.video.currentTime).toBe(5);
  });

  it("stays put when the status names no copy", async () => {
    const s = stage(K, { answer: { state: "live", keep: K } });
    s.click();
    s.at(4);
    s.emit("error");
    await new Promise((ok) => setImmediate(ok));
    expect(s.asked).toEqual(["/_shroom/status/x"]);
    expect(s.video.src).toBe("/m/x/video.mp4");
  });
});

describe("the player takes new ranges from the page (shroom:keep, agreed with the voice-cut build #59)", () => {
  const { shroomKeep } = new Function(`${PLAYER_JS};return { shroomKeep }`)();

  it("makes the ranges safe as editsOf makes an edit: numbers only, held to the recording, merged, at most 500", () => {
    expect(shroomKeep([[15, 30], ["x", 3], [5, 4], [1, 3], [2.5, 6], [NaN, 1], null, [Infinity, 2], [-4, 0.5], { 0: 7, 1: 8 }], 20)).toEqual([[0, 0.5], [1, 6], [15, 20]]);
    expect(shroomKeep([[5, 8], [0, 5]], 20)).toEqual([[0, 8]]);
    for (const bad of ["nope", null, undefined, 3, {}, [[]], [["1", "2"]]]) expect(shroomKeep(bad, 20), JSON.stringify(bad)).toEqual([]);
    const many = Array.from({ length: 2000 }, (_, i) => [i * 0.04, i * 0.04 + 0.02]);
    expect(shroomKeep(many, 100)).toHaveLength(500);
    // A recording whose length isn't known yet isn't held at the top.
    expect(shroomKeep([[1, 50]], 0)).toEqual([[1, 50]]);
  });

  it("swaps them in: the length, the seek bar and the playhead follow", () => {
    const s = stage(null);
    expect(s.time()).toBe("0:00 / 0:20");
    s.keepEvent([[2, 6], [9, 12]]);
    expect(s.time()).toBe("0:00 / 0:07");
    expect(s.seek.max).toBe("7");
    expect(s.video.currentTime).toBe(2);
    s.click();
    // Playing, the frames run the skip from here on.
    expect(s.framing()).toBe(true);
    s.at(5.98);
    expect(s.video.currentTime).toBe(9);
    s.at(11.98);
    expect(s.video.paused).toBe(true);
    expect(s.time()).toBe("0:07 / 0:07");
  });

  it("moves a playhead a new cut covers on to the next range's start; past the last, it ends there", () => {
    const s = stage([[2, 6], [9, 12], [13.5, 18]]);
    s.click();
    s.at(10);
    s.keepEvent([[0, 9.5], [11, 20]]);
    expect(s.video.currentTime).toBe(11);
    expect(s.time()).toBe("0:09 / 0:19");
    s.keepEvent([[0, 5]]);
    expect(s.video.paused).toBe(true);
    expect(s.time()).toBe("0:05 / 0:05");
  });

  it("starts the frames when ranges arrive mid-play", () => {
    const s = stage(null);
    s.click();
    s.at(1);
    expect(s.framing()).toBe(false);
    s.keepEvent([[0, 3], [6, 20]]);
    expect(s.framing()).toBe(true);
    s.at(2.98);
    expect(s.video.currentTime).toBe(6);
  });

  it("ranges that leave everything in put the recording back as it was", () => {
    const s = stage([[2, 6], [9, 12]]);
    s.keepEvent([[0, 20]]);
    expect(s.time()).toBe("0:00 / 0:20");
    s.click();
    s.at(7);
    expect(s.video.currentTime).toBe(7);
  });

  it("a detail that holds nothing to play changes nothing", () => {
    const s = stage([[2, 6], [9, 12]]);
    for (const detail of ["nope", [], [[5, 5]], [[30, 40]], null]) s.keepEvent(detail);
    expect(s.time()).toBe("0:00 / 0:07");
  });

  it("does nothing while the permanent copy plays: it is the edit already", async () => {
    const s = stage(null, { copy: true });
    s.keepEvent([[2, 6]]);
    expect(s.time()).toBe("0:00 / 0:20");
    expect(s.video.currentTime).toBe(0);
    // Nor once a player has moved over to the copy.
    const moved = stage([[2, 6], [9, 12], [13.5, 18]], { answer: { state: "live", copy: { src: "/m/x/edit-3.mp4", dur: 11.4 } } });
    moved.click();
    moved.at(4);
    moved.emit("error");
    await new Promise((ok) => setImmediate(ok));
    expect(moved.video.src).toBe("/m/x/edit-3.mp4");
    moved.video.duration = 11.4;
    moved.emit("loadedmetadata");
    moved.keepEvent([[0, 1]]);
    moved.at(5);
    expect(moved.video.currentTime).toBe(5);
    expect(moved.time()).toBe("0:05 / 0:11");
  });

  it("hands every player the recording's own length and whether it plays the copy", async () => {
    expect(playerArg(await text(edited(), `/${REC}`))).toMatchObject({ len: 272.4, copy: false });
    expect(playerArg(await text(copied(), `/${REC}?embed=1`))).toMatchObject({ len: 272.4, copy: true });
    expect(await text(space({ objects: { [metaKey(REC)]: uploadingRecord() } }), `/${REC}`, owner)).toContain(`st:"/_shroom/status/"+c.id,copy:!!cp}`);
  });
});

/** A record still uploading: its owner page swaps the player in at the go-live. */
const uploadingRecord = () => metaOf(REC, { state: "uploading", media: null, progress: { done: 1, total: 4, updatedAt: new Date(NOW).toISOString() } });

// B197: after an edit, the original goes and the edit becomes the video — the bake points
// `media` at the copy's own files (`hls: null`), sets `durationSec` to the copy's length and
// drops `edits`. The page, Download and `/m/` follow `media`'s keys from then on.
describe("a baked recording: the edit became the video (B197)", () => {
  const BAKED_MEDIA = { hls: null, mp4: `rec/${REC}/edit-3.mp4`, poster: `rec/${REC}/edit-3-poster.jpg`, teaser: `rec/${REC}/edit-3-teaser.mp4`, width: 1920, height: 1080 };
  const BAKED_FROM = [{ at: "2026-10-18T09:00:00.123Z", keep: [[8.125, 150.375], [162, 241]] }];
  const bakedRecord = (over = {}) => metaOf(REC, { title: "Where the 502s come from", titled: true, durationSec: 220.9, media: BAKED_MEDIA, bakedFrom: BAKED_FROM, ...over });
  const EDIT5 = { [`rec/${REC}/edit-5.mp4`]: "edit-5.mp4", [`rec/${REC}/edit-5-poster.jpg`]: "edit-5-poster.jpg", [`rec/${REC}/edit-5-teaser.mp4`]: "edit-5-teaser.mp4" };
  const baked = (over) => space({ objects: { [metaKey(REC)]: bakedRecord(over), ...files(), ...EDIT5 } });
  /** Edited again, over its own clock: 2–50 and 60–200 of its 220.9 s play — 188 s, 3:08. */
  const AGAIN = { version: 1, source_duration: 220.9, in: 2, out: 200, cuts: [{ id: "m1", start: 50, end: 60, kind: "manual", confidence: "high", applied: true, removed: SAID[0] }], rev: 5 };
  const COPY5 = { rev: 5, file: "edit-5.mp4", duration: 187.9, poster: "edit-5-poster.jpg", teaser: "edit-5-teaser.mp4" };

  it("is a live video that plays its one file, with its own first frame, teaser and length", async () => {
    const env = baked();
    const body = await text(env, `/${REC}`);
    expect(playerSrc(body)).toBe(`/m/${REC}/edit-3.mp4`);
    expect(playerArg(body)).toEqual({ keep: null, len: 220.9, st: `/_shroom/status/${REC}`, copy: false });
    expect(body.match(/<video[^>]*>/)?.[0]).toContain(`poster="/m/${REC}/edit-3-poster.jpg"`);
    expect(body).toContain(`class="player" data-dur="220.9"`);
    expect(body).toContain(`<span class="time">0:00 / 3:41</span>`);
    expect(body).toContain(`<a href="/d/${REC}">${COPY.download}</a>`);
    const embed = await text(env, `/${REC}?embed=1`);
    expect(playerSrc(embed)).toBe(`/m/${REC}/edit-3.mp4`);
    expect(embed.match(/<video[^>]*>/)?.[0]).toContain(`poster="${ORIGIN}/m/${REC}/edit-3-poster.jpg"`);
    const mine = await text(env, `/${REC}`, owner);
    expect(mine).toContain(`class="title"`);
    expect(mine).toContain(`src="/m/${REC}/edit-3.mp4"`);
    expect(mine).toContain(`"mp4":true`);
    expect(mine).toContain(`"len":"3:41","psrc":"/m/${REC}/edit-3-poster.jpg","tsrc":"/m/${REC}/edit-3-teaser.mp4"`);
    expect(JSON.parse(await text(env, `/_shroom/status/${REC}`))).toEqual({ state: "live", poster: true, mp4: true, teaser: true, thumb: null, dur: 220.9, drained: true });
  });

  it("unfurls with its own pictures and length", async () => {
    const body = await text(baked(), `/${REC}`);
    expect(body).toContain(`<meta property="og:image" content="${ORIGIN}/m/${REC}/edit-3-poster.jpg">`);
    expect(body).toContain(`<meta property="og:video" content="${ORIGIN}/m/${REC}/edit-3-teaser.mp4">`);
    expect(body).toContain(`<meta property="og:video:duration" content="221">`);
    const oembed = JSON.parse(await text(baked(), `/oembed?url=${encodeURIComponent(`${ORIGIN}/${REC}`)}`));
    expect(oembed.thumbnail_url).toBe(`${ORIGIN}/m/${REC}/edit-3-poster.jpg`);
    // No teaser, no poster of its own: none shown, and no original's in their place.
    const bare = await text(baked({ media: { ...BAKED_MEDIA, poster: null, teaser: undefined } }), `/${REC}`);
    expect(bare).not.toContain("og:image");
    expect(bare).not.toContain("og:video");
    expect(bare.match(/<video[^>]*>/)?.[0]).not.toContain("poster=");
  });

  it("hands its file over as the download, named after the title", async () => {
    const res = await get(baked(), `/d/${REC}`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-disposition")).toContain(`filename*=UTF-8''Where%20the%20502s%20come%20from.mp4`);
    expect(await res.text()).toBe("edit-3.mp4");
    expect((await get(baked(), `/d/${REC}`, { headers: { range: "bytes=0-3" } })).status).toBe(206);
  });

  it("serves exactly the files its record names, and Copy thumbnail's pictures", async () => {
    const env = baked();
    for (const f of ["edit-3.mp4", "edit-3-poster.jpg", "edit-3-teaser.mp4", "thumb-0123456789abcdef.gif"]) expect((await get(env, `/m/${REC}/${f}`)).status, f).toBe(200);
    // What a bake left behind, or an earlier one's file: none of it is the video now.
    for (const f of ["master.m3u8", "v0/stream.m3u8", "v0/seg_00000.m4s", "video.mp4", "poster.jpg", "teaser.mp4", "edit-2.mp4", "edit-5.mp4", "captions.vtt"]) {
      for (const init of [{}, { method: "HEAD" }, { headers: { range: "bytes=0-0" } }]) {
        const res = await get(env, `/m/${REC}/${f}`, init);
        expect(res.status, `${init.method ?? "GET"} ${f}`).toBe(404);
        await res.arrayBuffer();
      }
    }
  });

  it("never waits on a stream it doesn't have: the playlist's answer, there or not, changes nothing", async () => {
    for (const playlist of [null, ONE_MAP, "#EXTM3U\n#EXT-X-MAP:URI=\"a\"\n#EXT-X-MAP:URI=\"b\"\n"]) {
      const objects = { [metaKey(REC)]: bakedRecord(), [`rec/${REC}/edit-3.mp4`]: "x" };
      if (playlist) objects[`rec/${REC}/v0/stream.m3u8`] = playlist;
      const env = space({ objects });
      expect(playerSrc(await text(env, `/${REC}`))).toBe(`/m/${REC}/edit-3.mp4`);
      expect(playerSrc(await text(env, `/${REC}?embed=1`))).toBe(`/m/${REC}/edit-3.mp4`);
    }
    const env = baked();
    const failing = env.BUCKET.get.bind(env.BUCKET);
    env.BUCKET.get = (key, opts) => key.endsWith("stream.m3u8") ? Promise.reject(new Error("get: internal error (10001)")) : failing(key, opts);
    expect(playerSrc(await text(env, `/${REC}`))).toBe(`/m/${REC}/edit-3.mp4`);
    // Nothing but the page's own read beside the record ever asks for it.
    const quiet = baked();
    for (const path of [`/d/${REC}`, `/_shroom/status/${REC}`, `/oembed?url=${encodeURIComponent(`${ORIGIN}/${REC}`)}`, `/m/${REC}/edit-3.mp4`]) await get(quiet, path);
    expect(quiet.BUCKET.read().filter((k) => k.endsWith("stream.m3u8"))).toEqual([]);
  });

  it("takes no key at its word: a key that isn't the recording's own file of its slot's kind is no file", async () => {
    for (const mp4 of [`rec/AAAAAAAAAAAAAAAA/edit-3.mp4`, `rec/${REC}/v0/edit-3.mp4`, `rec/${REC}/../${REC}/edit-3.mp4`, ".shroom/cookie.key", `rec/${REC}/edit-3.mp4.json`,
      `rec/${REC}/edit-3-poster.jpg`, `meta/${REC}.json`, `rec/${REC}/edit-.mp4`, 42, ["x"], {}]) {
      const env = baked({ media: { ...BAKED_MEDIA, mp4 } });
      expect(await text(env, `/${REC}`), JSON.stringify(mp4)).toContain(COPY.unavailable);
      expect(JSON.parse(await text(env, `/_shroom/status/${REC}`))).toEqual({ state: "unavailable" });
      expect((await get(env, `/d/${REC}`)).status).toBe(404);
      expect(env.BUCKET.read().filter((k) => !k.startsWith(`rec/${REC}/`) && ![metaKey(REC), `.shroom/gone/${REC}`, ".shroom/mark.json"].includes(k))).toEqual([]);
    }
    const wrongKinds = await text(baked({ media: { ...BAKED_MEDIA, poster: `rec/${REC}/edit-3.mp4`, teaser: `rec/${REC}/edit-3-poster.jpg` } }), `/${REC}`);
    expect(wrongKinds.match(/<video[^>]*>/)?.[0]).not.toContain("poster=");
    expect(wrongKinds).not.toContain("og:video");
  });

  it("keeps bakedFrom to itself: none of it reaches a viewer", async () => {
    const env = baked();
    const seen = [await text(env, `/${REC}`), await text(env, `/${REC}?embed=1`), await text(env, `/${REC}`, owner), await text(env, `/_shroom/status/${REC}`),
      await text(env, `/oembed?url=${encodeURIComponent(`${ORIGIN}/${REC}`)}`)].join("\n");
    for (const s of ["bakedFrom", "150.375", "8.125", BAKED_FROM[0].at]) expect(seen).not.toContain(s);
  });

  it("can be edited again, over its own clock: the list plays over its file until a newer copy is up", async () => {
    const env = baked({ edits: AGAIN });
    const body = await text(env, `/${REC}`);
    expect(playerSrc(body)).toBe(`/m/${REC}/edit-3.mp4`);
    expect(playerArg(body)).toEqual({ keep: [[2, 50], [60, 200]], len: 220.9, st: `/_shroom/status/${REC}`, copy: false });
    expect(body).toContain(`<span class="time">0:00 / 3:08</span>`);
    expect(body).not.toContain(`href="/d/${REC}"`);
    expect((await get(env, `/d/${REC}`)).status).toBe(404);
    for (const f of ["edit-3.mp4", "edit-3-poster.jpg", "edit-3-teaser.mp4"]) expect((await get(env, `/m/${REC}/${f}`)).status, f).toBe(200);
    for (const f of ["edit-5.mp4", "video.mp4", "master.m3u8"]) expect((await get(env, `/m/${REC}/${f}`)).status, f).toBe(404);
    for (const s of SAID) expect(body).not.toContain(s);
  });

  it("once the newer copy is up: plays it, hands it over, and the earlier video is out of reach", async () => {
    const env = baked({ edits: { ...AGAIN, copy: COPY5 } });
    const body = await text(env, `/${REC}`);
    expect(playerSrc(body)).toBe(`/m/${REC}/edit-5.mp4`);
    expect(playerArg(body)).toEqual({ keep: null, len: 220.9, st: `/_shroom/status/${REC}`, copy: true });
    expect(body.match(/<video[^>]*>/)?.[0]).toContain(`poster="/m/${REC}/edit-5-poster.jpg"`);
    expect(body).toContain(`<span class="time">0:00 / 3:08</span>`);
    expect(await (await get(env, `/d/${REC}`)).text()).toBe("edit-5.mp4");
    // The copy's files, the recording's own first frame (the Library shows it) and the
    // pictures Copy thumbnail made; the earlier video and its teaser are out of reach.
    for (const f of ["edit-5.mp4", "edit-5-poster.jpg", "edit-5-teaser.mp4", "edit-3-poster.jpg", "thumb-0123456789abcdef.gif"]) expect((await get(env, `/m/${REC}/${f}`)).status, f).toBe(200);
    for (const f of ["edit-3.mp4", "edit-3-teaser.mp4", "poster.jpg", "video.mp4", "master.m3u8"]) expect((await get(env, `/m/${REC}/${f}`)).status, f).toBe(404);
    // A stale copy: the list over the recording's own file again.
    const stale5 = baked({ edits: { ...AGAIN, rev: 6, copy: COPY5 } });
    expect(playerSrc(await text(stale5, `/${REC}`))).toBe(`/m/${REC}/edit-3.mp4`);
    expect((await get(stale5, `/m/${REC}/edit-3.mp4`)).status).toBe(200);
    expect((await get(stale5, `/m/${REC}/edit-5.mp4`)).status).toBe(404);
  });

  it("leaves a record that still has its stream as it was: a set slot is the original's fixed file", async () => {
    // The Worker never took those keys at their word, and doesn't now.
    const env = space({ objects: { [metaKey(REC)]: record(undefined, { media: { ...MEDIA, mp4: ".shroom/cookie.key", poster: "p", teaser: "t" } }), ...files() } });
    const body = await text(env, `/${REC}`);
    expect(body.match(/<video[^>]*>/)?.[0]).toContain(`poster="/m/${REC}/poster.jpg"`);
    expect(body).toContain(`<meta property="og:video" content="${ORIGIN}/m/${REC}/teaser.mp4">`);
    expect(await (await get(env, `/d/${REC}`)).text()).toBe("video.mp4");
    expect((await get(env, `/m/${REC}/v0/seg_00000.m4s`)).status).toBe(200);
  });
});

describe("the sound dips at each skip instead of clicking (N63; the voice-cut build's joins, PR #59)", () => {
  const K = [[1, 10], [20, 30], [40, 50]];
  const F = 0.05;
  it("plays at full level away from every join", () => {
    for (const t of [1, 5, 9.8, 25, 45, 49.99]) expect(shroomGain(K, t, F), `${t}`).toBe(1);
  });
  it("eases out over the last 50 ms before a jump, and in over the first 50 ms after it", () => {
    // The jump to 20 s comes at 10 − 0.03 s.
    expect(shroomGain(K, 9.97 - 0.05, F)).toBeCloseTo(1, 6);
    expect(shroomGain(K, 9.97 - 0.025, F)).toBeCloseTo(0.5, 6);
    expect(shroomGain(K, 9.969, F)).toBeLessThan(0.05);
    expect(shroomGain(K, 20, F)).toBe(0);
    expect(shroomGain(K, 20.025, F)).toBeCloseTo(0.5, 6);
    expect(shroomGain(K, 20.05, F)).toBeCloseTo(1, 6);
  });
  it("leaves the trim's own ends alone: playback starts and ends there, with nothing joined", () => {
    expect(shroomGain(K, 1, F)).toBe(1);
    expect(shroomGain(K, 49.99, F)).toBe(1);
    expect(shroomGain(K, 60, F)).toBe(1);
  });
  it("takes the shorter ease on a range too short for both", () => {
    const k = [[0, 5], [6, 6.06], [8, 9]];
    for (let t = 6; t < 6.03; t += 0.005) expect(shroomGain(k, t, F)).toBeLessThanOrEqual(1);
    expect(shroomGain(k, 6.015, F)).toBeCloseTo(0.3, 6);
  });
});

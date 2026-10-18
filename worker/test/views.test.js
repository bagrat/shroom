// Plays and the videos suggested to clean up (POC, dev/proposals/cleanup-suggestions.md):
// the page's beacons are counted in .shroom/views/ with nothing that names a viewer, the
// owner's own page never counts, and POST /_shroom/api/cleanup scores each video under
// one of four formulas, with the reason and the facts its line needs.
import { describe, expect, it } from "vitest";
import { BYTES_PER_SEC, FORMULAS, VIEW_JS, applyView, cleanupFacts, cleanupScore, medianWatched, monthlySaving } from "../src/worker.js";
import {
  DAY, HOST, M1, MHER, MHER_MAC, NOW, OWNER_MAC, bodyOf, call, cookieFor, iso, ledger, metaKey, metaOf, recId, space, textOf,
} from "./space-fixture.js";

const VID = recId(1);
const viewsKey = (id) => `.shroom/views/${id}.json`;
const live = (id, over = {}) => metaOf(id, { durationSec: 300, title: `Video ${id.slice(-2)}`, ...over });
const beacon = (env, id, body, { now = NOW, ip = "203.0.113.7", ua = "Safari", origin = `https://${HOST}` } = {}) =>
  call(env, "POST", `/_shroom/view/${id}`, { body, now, headers: { "content-type": "text/plain", "cf-connecting-ip": ip, "user-agent": ua, ...(origin ? { origin } : {}) } });

describe("counting plays", () => {
  it("a play and its end land in the video's record, with no address, browser or token kept past the day", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    expect((await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" })).status).toBe(204);
    expect((await beacon(env, VID, { e: "end", t: "tok_aaaaaaaa", f: 0.83 })).status).toBe(204);
    const rec = env.BUCKET.json(viewsKey(VID));
    expect(rec).toMatchObject({ v: 1, plays: 1, viewers: 1, ends: 1, lastAt: iso(NOW), days: { "2026-10-02": 1 } });
    expect(rec.watch[8]).toBe(1);
    expect(rec.watchedSec).toBeCloseTo(255);
    const raw = env.BUCKET.text(viewsKey(VID));
    expect(raw).not.toContain("203.0.113.7");
    expect(raw).not.toContain("Safari");
    expect(rec.today.h[0]).toMatch(/^[0-9a-f]{8}$/);
    // The next day's play keeps the counts and forgets the day before's hashes and tokens.
    await beacon(env, VID, { e: "play", t: "tok_bbbbbbbb" }, { now: NOW + DAY });
    const next = env.BUCKET.json(viewsKey(VID));
    expect(next).toMatchObject({ plays: 2, viewers: 2, days: { "2026-10-02": 1, "2026-10-03": 1 } });
    expect(next.today.h).toHaveLength(1);
    expect(Object.keys(next.today.t)).toEqual(["tok_bbbbbbbb"]);
  });

  it("the day's salt is replaced when the day turns, so one viewer reads as new the next day", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" });
    const salt1 = env.BUCKET.json(".shroom/views/salt.json");
    await beacon(env, VID, { e: "play", t: "tok_bbbbbbbb" }, { now: NOW + DAY });
    const salt2 = env.BUCKET.json(".shroom/views/salt.json");
    expect(salt2.d).toBe("2026-10-03");
    expect(salt2.salt).not.toBe(salt1.salt);
  });

  it("one viewer replaying the same day is one viewer; a second browser is another", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" });
    await beacon(env, VID, { e: "play", t: "tok_bbbbbbbb" });
    await beacon(env, VID, { e: "play", t: "tok_cccccccc" }, { ua: "Firefox" });
    expect(env.BUCKET.json(viewsKey(VID))).toMatchObject({ plays: 3, viewers: 2 });
  });

  it("a page load's play counts once, and its latest end replaces the one before", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" });
    await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" });
    await beacon(env, VID, { e: "end", t: "tok_aaaaaaaa", f: 0.2 });
    await beacon(env, VID, { e: "end", t: "tok_aaaaaaaa", f: 1 });
    const rec = env.BUCKET.json(viewsKey(VID));
    expect(rec).toMatchObject({ plays: 1, ends: 1 });
    expect(rec.watch).toEqual([0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    expect(rec.watchedSec).toBeCloseTo(285);
  });

  it("a video that isn't live, an unknown id, or a space with counting off gets no record", async () => {
    const gone = recId(2), up = recId(3);
    const env = space({ objects: { [metaKey(VID)]: live(VID), [metaKey(up)]: live(up, { state: "uploading" }), [`.shroom/gone/${gone}`]: "", [metaKey(gone)]: live(gone) } });
    for (const id of [gone, up, recId(4)]) expect((await beacon(env, id, { e: "play", t: "tok_aaaaaaaa" })).status).toBe(204);
    expect(env.BUCKET.written().filter((k) => k.startsWith(".shroom/views/"))).toEqual([]);
    await env.BUCKET.put(".shroom/views.off", "");
    await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" });
    expect(env.BUCKET.json(viewsKey(VID))).toBeNull();
  });

  it("refuses a cross-site beacon and a malformed one, and writes nothing", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    expect((await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" }, { origin: "https://elsewhere.example.com" })).status).toBe(403);
    expect((await call(env, "POST", `/_shroom/view/${VID}`, { body: { e: "play", t: "tok_aaaaaaaa" }, headers: { "sec-fetch-site": "cross-site" } })).status).toBe(403);
    for (const body of [{ e: "seek", t: "tok_aaaaaaaa" }, { e: "play", t: "x" }, { e: "end", t: "tok_aaaaaaaa", f: 2 }, "not json", "x".repeat(600)]) {
      expect((await beacon(env, VID, body)).status).toBe(400);
    }
    expect(env.BUCKET.json(viewsKey(VID))).toBeNull();
  });

  it("takes WebKit's beacon from a page being left, sent as Origin: null", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    expect((await beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" }, { origin: "null" })).status).toBe(204);
    expect(env.BUCKET.json(viewsKey(VID))).toMatchObject({ plays: 1 });
  });

  it("two plays racing on one record both count (compare-and-swap)", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    await Promise.all([beacon(env, VID, { e: "play", t: "tok_aaaaaaaa" }), beacon(env, VID, { e: "play", t: "tok_bbbbbbbb" }, { ip: "198.51.100.2" })]);
    expect(env.BUCKET.json(viewsKey(VID))).toMatchObject({ plays: 2, viewers: 2 });
  });

  it("the viewer's page carries the beacon; the owner's own page never does", async () => {
    const env = space({ objects: { [metaKey(VID)]: live(VID) } });
    await ledger(env, VID, M1, OWNER_MAC.id);
    const viewer = await (await call(env, "GET", `/${VID}`)).text();
    expect(viewer).toContain("shroomCount(");
    const embed = await (await call(env, "GET", `/${VID}?embed=1`)).text();
    expect(embed).toContain("shroomCount(");
    const mine = await (await call(env, "GET", `/${VID}`, { headers: { cookie: cookieFor(M1, OWNER_MAC) } })).text();
    expect(textOf(mine)).toBeTruthy();
    expect(mine).not.toContain("shroomCount(");
    expect(VIEW_JS).not.toMatch(/cookie|localStorage|sessionStorage/);
  });
});

describe("the record's arithmetic", () => {
  it("median watched reads the tenths' middles", () => {
    expect(medianWatched(null)).toBeNull();
    expect(medianWatched({ watch: [5, 0, 0, 0, 0, 0, 0, 0, 0, 0] })).toBeCloseTo(0.05);
    expect(medianWatched({ watch: [1, 0, 0, 0, 0, 0, 0, 0, 0, 2] })).toBeCloseTo(0.95);
    expect(medianWatched({ watch: [2, 0, 0, 0, 1, 0, 0, 0, 0, 2] })).toBeCloseTo(0.45);
  });

  it("per-day counts keep the last 400 days", () => {
    let r = null;
    for (let d = 0; d < 410; d++) r = applyView(r, { e: "play", t: `tok_${String(d).padStart(8, "0")}`, f: 0, viewer: null }, NOW + d * DAY, 60);
    expect(Object.keys(r.days)).toHaveLength(400);
    expect(r.plays).toBe(410);
  });

  it("what deleting saves is nothing inside the free 10 GB, and the list price past it", () => {
    expect(monthlySaving(8e9, 2e9)).toBe(0);
    expect(monthlySaving(12e9, 1e9)).toBeCloseTo(0.015);
    expect(monthlySaving(11e9, 3e9)).toBeCloseTo(0.015);
    expect(BYTES_PER_SEC * 3600 * 3).toBeCloseTo(10e9, -6);
  });
});

describe("the scores", () => {
  const SINCE = NOW - 120 * DAY;
  const days = (n, ...ago) => Object.fromEntries(ago.map((a) => [iso(NOW - a * DAY).slice(0, 10), n]));
  const facts = (over = {}, rec = null) => cleanupFacts({ state: "live", createdAt: iso(NOW - 200 * DAY), durationSec: 600, ...over }, rec, NOW, SINCE);

  it("never suggests a kept video, a new one, one watched in the last two weeks, or before counting can tell", () => {
    for (const f of FORMULAS) {
      expect(cleanupScore(facts({ keep: true }), f, NOW).reason.code).toBe("kept");
      expect(cleanupScore(facts({ createdAt: iso(NOW - 5 * DAY) }), f, NOW).reason.code).toBe("new");
      expect(cleanupScore(facts({}, { v: 1, plays: 1, lastAt: iso(NOW - 3 * DAY), days: days(1, 3), watch: [] }), f, NOW).reason.code).toBe("watched-lately");
      expect(cleanupScore(cleanupFacts({ createdAt: iso(NOW - 200 * DAY), durationSec: 60 }, null, NOW, null), f, NOW).reason.code).toBe("counting-too-new");
    }
  });

  it("fading-cost: faded plays and a cent a month or more; a past hit and small clutter are left", () => {
    expect(cleanupScore(facts({ durationSec: 1800 }), "fading-cost", NOW)).toMatchObject({ candidate: true, reason: { code: "never-played", heat: 0 } });
    expect(cleanupScore(facts({ durationSec: 300 }), "fading-cost", NOW).candidate).toBe(false); // ~0.28 GB: under a cent
    const hit = { v: 1, plays: 30, lastAt: iso(NOW - 150 * DAY), days: days(10, 150, 151, 152), watch: [0, 0, 0, 0, 0, 0, 0, 0, 0, 30] };
    expect(cleanupScore(facts({ durationSec: 1800 }, hit), "fading-cost", NOW).candidate).toBe(false);
    const faded = { ...hit, plays: 6, days: days(2, 150, 151, 152), watch: [0, 0, 0, 0, 0, 0, 0, 0, 0, 6] };
    const v = cleanupScore(facts({ durationSec: 1800 }, faded), "fading-cost", NOW);
    expect(v).toMatchObject({ candidate: true, reason: { code: "not-lately", plays: 6 } });
    // The dearer of two equally faded videos goes first.
    expect(cleanupScore(facts({ durationSec: 3600 }, faded), "fading-cost", NOW).score).toBeGreaterThan(v.score);
  });

  it("idle: nobody in 30 days if nobody ever, else 60; counting that began late counts from then", () => {
    expect(cleanupScore(facts(), "idle", NOW)).toMatchObject({ candidate: true, reason: { code: "never-played", quietDays: 120 } });
    const late = cleanupFacts({ createdAt: iso(NOW - 400 * DAY), durationSec: 60 }, null, NOW, NOW - 20 * DAY);
    expect(cleanupScore(late, "idle", NOW).candidate).toBe(false);
    const played = { v: 1, plays: 4, lastAt: iso(NOW - 45 * DAY), days: days(4, 45), watch: [0, 0, 0, 0, 0, 0, 0, 0, 1, 3] };
    expect(cleanupScore(facts({}, played), "idle", NOW)).toMatchObject({ candidate: false, reason: { code: "not-lately" } });
    expect(cleanupScore(facts({}, { ...played, lastAt: iso(NOW - 75 * DAY), days: days(4, 75) }), "idle", NOW).candidate).toBe(true);
  });

  it("idle ranks bigger and quieter first", () => {
    const small = cleanupScore(facts({ durationSec: 60 }), "idle", NOW).score;
    const big = cleanupScore(facts({ durationSec: 3600 }), "idle", NOW).score;
    expect(big).toBeGreaterThan(small);
  });

  it("fading: a burst five months ago has faded; a big one hasn't yet; a steady trickle keeps it", () => {
    const burst = { v: 1, plays: 6, lastAt: iso(NOW - 150 * DAY), days: days(2, 150, 151, 152), watch: [0, 0, 0, 0, 0, 0, 0, 0, 0, 6] };
    expect(cleanupScore(facts({}, burst), "fading", NOW).candidate).toBe(true);
    const big = { ...burst, plays: 30, days: days(10, 150, 151, 152), watch: [0, 0, 0, 0, 0, 0, 0, 0, 0, 30] };
    expect(cleanupScore(facts({}, big), "fading", NOW).candidate).toBe(false);
    const trickle = { v: 1, plays: 4, lastAt: iso(NOW - 15 * DAY), days: days(1, 15, 30, 45, 60), watch: [0, 0, 0, 0, 0, 0, 0, 0, 0, 4] };
    expect(cleanupScore(facts({}, trickle), "fading", NOW).candidate).toBe(false);
  });

  it("stopped-early names videos people opened but left within the first quarter", () => {
    const rec = { v: 1, plays: 3, lastAt: iso(NOW - 90 * DAY), days: days(1, 90, 95, 99), watch: [2, 1, 0, 0, 0, 0, 0, 0, 0, 0] };
    expect(cleanupScore(facts({}, rec), "idle", NOW).reason.code).toBe("stopped-early");
  });

  it("cost: only what costs a cent a month or more, and draws under a play a month", () => {
    expect(cleanupScore(facts({ durationSec: 120 }), "cost", NOW).candidate).toBe(false);
    expect(cleanupScore(facts({ durationSec: 1800 }), "cost", NOW)).toMatchObject({ candidate: true, reason: { code: "big-and-quiet" } });
    expect(cleanupScore(facts({ durationSec: 1800, bytes: 4e9 }), "cost", NOW).reason.usdPerMonth).toBeCloseTo(0.06);
  });

  it("gravity: a year-old video with a handful of plays sinks; a month-old popular one doesn't", () => {
    const oldFew = facts({ createdAt: iso(NOW - 365 * DAY) }, { v: 1, plays: 5, lastAt: iso(NOW - 300 * DAY), days: {}, watch: [0, 0, 0, 0, 0, 5, 0, 0, 0, 0] });
    expect(cleanupScore(oldFew, "gravity", NOW).candidate).toBe(true);
    const youngHit = facts({ createdAt: iso(NOW - 35 * DAY) }, { v: 1, plays: 40, lastAt: iso(NOW - 31 * DAY), days: {}, watch: [0, 0, 0, 0, 0, 0, 0, 0, 0, 40] });
    expect(cleanupScore(youngHit, "gravity", NOW).candidate).toBe(false);
  });
});

describe("POST /_shroom/api/cleanup", () => {
  const LATER = NOW + 100 * DAY;
  async function populated() {
    const a = recId(10), b = recId(11), c = recId(12), d = recId(13);
    const env = space({
      objects: {
        [metaKey(a)]: live(a, { createdAt: iso(NOW - 10 * DAY), durationSec: 900 }),
        [metaKey(b)]: live(b, { createdAt: iso(NOW - 10 * DAY) }),
        [metaKey(c)]: live(c, { createdAt: iso(NOW - 10 * DAY), keep: true }),
        [metaKey(d)]: live(d, { createdAt: iso(NOW - 10 * DAY), state: "uploading" }),
      },
    });
    await ledger(env, a, M1, OWNER_MAC.id); await ledger(env, b, MHER, MHER_MAC.id); await ledger(env, c, M1, OWNER_MAC.id); await ledger(env, d, M1, OWNER_MAC.id);
    await beacon(env, b, { e: "play", t: "tok_aaaaaaaa" }); // counting begins at NOW
    await beacon(env, b, { e: "play", t: "tok_bbbbbbbb" }, { now: LATER - 5 * DAY });
    return { env, a, b, c };
  }

  it("owner-class sees everyone's live videos, scored, with when counting began", async () => {
    const { env, a, b, c } = await populated();
    const res = await call(env, "POST", "/_shroom/api/cleanup", { auth: OWNER_MAC.bearer, body: { scope: "everyone" }, now: LATER });
    expect(res.status).toBe(200);
    const out = await bodyOf(res);
    expect(out).toMatchObject({ shroom: 1, countingSince: iso(NOW), formula: "fading-cost", next: null });
    const by = Object.fromEntries(out.items.map((i) => [i.id, i]));
    expect(Object.keys(by).sort()).toEqual([a, b, c].sort());
    expect(by[a]).toMatchObject({ candidate: true, plays: 0, owner: M1, reason: { code: "never-played", quietDays: 100 } });
    expect(by[a].bytes).toBe(Math.round(900 * BYTES_PER_SEC));
    expect(by[b]).toMatchObject({ candidate: false, plays: 2, owner: MHER, reason: { code: "watched-lately" } });
    expect(by[c]).toMatchObject({ candidate: false, reason: { code: "kept" } });
  });

  it("a member sees only their own, and can't ask for everyone's", async () => {
    const { env, b } = await populated();
    const out = await bodyOf(await call(env, "POST", "/_shroom/api/cleanup", { auth: MHER_MAC.bearer, body: { scope: "mine", formula: "fading" }, now: LATER }));
    expect(out.items.map((i) => i.id)).toEqual([b]);
    expect((await call(env, "POST", "/_shroom/api/cleanup", { auth: MHER_MAC.bearer, body: { scope: "everyone" }, now: LATER })).status).toBe(403);
  });

  it("refuses an unknown formula, and a caller without a device key", async () => {
    const { env } = await populated();
    expect((await call(env, "POST", "/_shroom/api/cleanup", { auth: OWNER_MAC.bearer, body: { formula: "vibes" } })).status).toBe(400);
    expect((await call(env, "POST", "/_shroom/api/cleanup", { body: {} })).status).toBe(401);
  });

  it("pages at 40 records a call", async () => {
    const objects = {};
    for (let i = 0; i < 45; i++) objects[metaKey(recId(100 + i))] = live(recId(100 + i));
    const env = space({ objects });
    const first = await bodyOf(await call(env, "POST", "/_shroom/api/cleanup", { auth: OWNER_MAC.bearer, body: { scope: "everyone" } }));
    expect(first.items).toHaveLength(40);
    expect(first.next).toBe(recId(139));
    const second = await bodyOf(await call(env, "POST", "/_shroom/api/cleanup", { auth: OWNER_MAC.bearer, body: { scope: "everyone", after: first.next } }));
    expect(second.items).toHaveLength(5);
    expect(second.next).toBeNull();
  });
});

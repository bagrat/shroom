// Auto-cleanup (spec N75, Bagrat 2026-10-24; a year, N77, 2026-10-25): off by default; Off / 15 / 30 / 60 / 365 days from
// the last counted watch, or from the upload if nobody played it; a pinned (kept) video is
// never deleted; three days' warning; and the space's own page does the deleting on its one
// schedule, so a closed Mac never misses one. The page scans a few records a run and deletes
// at most one video a run, exactly as a Delete does.
import { describe, expect, it } from "vitest";
import worker, { AUTOCLEAN_SCAN, autocleanTick } from "../src/worker.js";
import { DAY, HOUR, M1, MHER, MHER_MAC, NOW, OWNER_MAC, bodyOf, call, goneKey, iso, ledger, metaKey, metaOf, recId, space } from "./space-fixture.js";

const CONFIG = ".shroom/autoclean.json";
const STATE = ".shroom/autoclean/state.json";
const SINCE = ".shroom/views/since.json";
const viewsKey = (id) => `.shroom/views/${id}.json`;
const editingKey = (id) => `.shroom/editing/${id}.json`;
const A = recId(1), B = recId(2), C = recId(3), D = recId(4);

/** A live video made `daysAgo` days before NOW, drained, with its bytes. */
const video = (id, daysAgo, over = {}) => metaOf(id, { createdAt: iso(NOW - daysAgo * DAY), durationSec: 300, bytes: 1_000_000_000, title: `Video ${id.slice(-2)}`, ...over });
const played = (lastAt) => JSON.stringify({ v: 1, plays: 1, viewers: 1, ends: 1, watch: [0, 0, 0, 0, 0, 0, 0, 0, 0, 1], watchedSec: 300, firstAt: lastAt, lastAt, days: {}, today: { d: "x", h: [], t: {} } });

/** A space with counting on since long ago, auto-cleanup at `days` turned on long ago. */
function cleanSpace(objects = {}, { days = 30, since = iso(NOW - 200 * DAY) } = {}) {
  return space({
    objects: {
      [SINCE]: JSON.stringify({ since: iso(NOW - 300 * DAY) }),
      ...(days === null ? {} : { [CONFIG]: JSON.stringify({ v: 1, days, at: since, since }) }),
      ...objects,
    },
  });
}

/** Runs the schedule until the scan has walked the whole listing. */
async function scan(env, now = NOW) {
  for (let i = 0; i < 50; i++) {
    await autocleanTick(env, now);
    const s = env.BUCKET.json(STATE);
    if (s && s.cursor === null && s.scanAt > 0) return s;
  }
  throw new Error("the scan never finished");
}

describe("the auto-cleanup setting", () => {
  it("is off until owner-class turns it on; members read it but can't change it", async () => {
    const env = cleanSpace({}, { days: null });
    expect(await bodyOf(await call(env, "GET", "/_shroom/api/autoclean", { auth: MHER_MAC.bearer }))).toMatchObject({ days: 0, since: null, going: [], last: null });
    expect((await call(env, "POST", "/_shroom/api/autoclean", { auth: MHER_MAC.bearer, body: { days: 30 } })).status).toBe(403);
    const on = await bodyOf(await call(env, "POST", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer, body: { days: 30 } }));
    expect(on).toMatchObject({ days: 30, since: iso(NOW) });
    expect(env.BUCKET.json(CONFIG)).toMatchObject({ v: 1, days: 30, at: iso(NOW), since: iso(NOW) });
    expect(await bodyOf(await call(env, "GET", "/_shroom/api/autoclean", { auth: MHER_MAC.bearer }))).toMatchObject({ days: 30 });
  });

  it("takes only Off, 15, 30 or 60 days, or a year", async () => {
    const env = cleanSpace();
    for (const days of [7, -1, "30", null, 30.5, 90, 364, 366, "365"]) expect((await call(env, "POST", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer, body: { days } })).status).toBe(400);
    expect((await call(env, "POST", "/_shroom/api/autoclean", { body: { days: 30 } })).status).toBe(401);
  });

  it("restarts the warning clock when turned on or shortened, and not when lengthened", async () => {
    const env = cleanSpace({}, { days: null });
    await call(env, "POST", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer, body: { days: 60 }, now: NOW });
    await call(env, "POST", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer, body: { days: 15 }, now: NOW + DAY });
    expect(env.BUCKET.json(CONFIG).since).toBe(iso(NOW + DAY));
    await call(env, "POST", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer, body: { days: 30 }, now: NOW + 2 * DAY });
    expect(env.BUCKET.json(CONFIG)).toMatchObject({ days: 30, since: iso(NOW + DAY) });
  });

  it("a change starts a fresh scan; Off forgets what was going", async () => {
    const env = cleanSpace({ [metaKey(A)]: video(A, 90) });
    await scan(env);
    expect(Object.keys(env.BUCKET.json(STATE).going)).toEqual([A]);
    await call(env, "POST", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer, body: { days: 60 } });
    expect(env.BUCKET.json(STATE)).toMatchObject({ scanAt: 0, cursor: null });
    await call(env, "POST", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer, body: { days: 0 } });
    expect(env.BUCKET.json(STATE).going).toEqual({});
  });
});

describe("what's going, and when", () => {
  it("lists a video due within three days, never sooner than three days from the warning", async () => {
    // A: made 90 days ago, never played → due long ago; it goes three days after the warning.
    // B: played 28 days ago → due in 2 days, warned now, goes in 3.
    // C: played 10 days ago → not due for 20 days, not listed.
    const env = cleanSpace({
      [metaKey(A)]: video(A, 90), [metaKey(B)]: video(B, 90), [viewsKey(B)]: played(iso(NOW - 28 * DAY)),
      [metaKey(C)]: video(C, 90), [viewsKey(C)]: played(iso(NOW - 10 * DAY)),
    });
    const s = await scan(env);
    expect(s.going).toEqual({ [A]: { warnedAt: iso(NOW), deletesAt: iso(NOW + 3 * DAY) }, [B]: { warnedAt: iso(NOW), deletesAt: iso(NOW + 3 * DAY) } });
  });

  it("at a year, counts from the last play however long ago, and never from before counting began", async () => {
    // A: played 363 days ago → due in 2 days. B: made 500 days ago, never played → due.
    // C: played 300 days ago, D: made 200 days ago → not for months.
    const objects = {
      [metaKey(A)]: video(A, 500), [viewsKey(A)]: played(iso(NOW - 363 * DAY)), [metaKey(B)]: video(B, 500),
      [metaKey(C)]: video(C, 500), [viewsKey(C)]: played(iso(NOW - 300 * DAY)), [metaKey(D)]: video(D, 200),
    };
    const env = cleanSpace({ ...objects, [SINCE]: JSON.stringify({ since: iso(NOW - 500 * DAY) }) }, { days: 365 });
    expect(Object.keys((await scan(env)).going).sort()).toEqual([A, B].sort());
    // Counting began 300 days ago: nothing has been quiet a year as far as it can tell.
    expect((await scan(cleanSpace(objects, { days: 365 }))).going).toEqual({});
  });

  it("never lists a pinned video, one still uploading, or one before counting began", async () => {
    const env = space({
      objects: {
        [SINCE]: JSON.stringify({ since: iso(NOW - 5 * DAY) }),
        [CONFIG]: JSON.stringify({ v: 1, days: 30, at: iso(NOW - 200 * DAY), since: iso(NOW - 200 * DAY) }),
        [metaKey(A)]: video(A, 90, { keep: true }),
        [metaKey(B)]: video(B, 90, { state: "uploading" }),
        [metaKey(C)]: video(C, 90),
      },
    });
    const s = await scan(env);
    // Counting began 5 days ago: C has been quiet only 5 days as far as counting can tell.
    expect(s.going).toEqual({});
  });

  it("turned on or shortened today, nothing goes before three days from now", async () => {
    const env = cleanSpace({ [metaKey(A)]: video(A, 90) }, { since: iso(NOW) });
    await scan(env, NOW - HOUR);
    const s = await scan(env, NOW + 2 * DAY);
    expect(s.going[A].deletesAt).toBe(iso(NOW + 3 * DAY));
  });

  it("a watch takes it off the list at the next scan", async () => {
    const env = cleanSpace({ [metaKey(A)]: video(A, 90) });
    await scan(env);
    await env.BUCKET.put(viewsKey(A), played(iso(NOW + HOUR)));
    const s = await scan(env, NOW + DAY + 2 * HOUR);
    expect(s.going).toEqual({});
  });

  it("walks a few records a run, then rests until the next day", async () => {
    const objects = {};
    for (let i = 1; i <= AUTOCLEAN_SCAN * 2 + 1; i++) objects[metaKey(recId(i))] = video(recId(i), 5);
    const env = cleanSpace(objects);
    await autocleanTick(env, NOW);
    expect(env.BUCKET.json(STATE).cursor).not.toBeNull();
    const s = await scan(env);
    expect(s.scanAt).toBe(NOW);
    env.BUCKET.ops.length = 0;
    await autocleanTick(env, NOW + HOUR);
    expect(env.BUCKET.ops.filter((o) => o.op === "list")).toEqual([]);
  });
});

describe("the delete, on the date", () => {
  /** A going after a scan, and the clock past its date. */
  async function due(objects = {}) {
    const env = cleanSpace({ [metaKey(A)]: video(A, 90), [`rec/${A}/master.m3u8`]: "x", [`rec/${A}/v0/seg0.m4s`]: "x", [`rec/${A}/v0/seg1.m4s`]: "x", ...objects });
    await ledger(env, A, M1);
    await scan(env);
    return env;
  }
  const after = NOW + 3 * DAY + HOUR;

  it("goes exactly as a Delete: the tombstone, the bytes, the seal, the plays", async () => {
    const env = await due({ [viewsKey(A)]: played(iso(NOW - 40 * DAY)) });
    await scan(env, after);
    await autocleanTick(env, after);
    expect(env.BUCKET.json(metaKey(A))).toMatchObject({ schema: 1, id: A, state: "deleted", deletedAt: iso(after), by: "auto", owner: M1 });
    expect([...env.BUCKET.store.keys()].filter((k) => k.startsWith(`rec/${A}/`))).toEqual([]);
    expect(env.BUCKET.json(goneKey(A))).toMatchObject({ v: 1, by: "auto", at: iso(after) });
    expect(env.BUCKET.json(viewsKey(A))).toBeNull();
    const s = env.BUCKET.json(STATE);
    expect(s.going).toEqual({});
    expect(s.last).toMatchObject({ at: iso(after), ids: [A], bytes: 1_000_000_000 });
    // The link says what a deleted video's says.
    expect((await call(env, "GET", `/_shroom/status/${A}`, { now: after })).status).toBe(200);
    expect(await bodyOf(await call(env, "GET", `/_shroom/status/${A}`, { now: after }))).toMatchObject({ state: "removed" });
  });

  it("never before its date", async () => {
    const env = await due();
    await autocleanTick(env, NOW + 2 * DAY);
    expect(env.BUCKET.json(metaKey(A)).state).toBe("live");
  });

  it("skips one pinned, watched or no longer live since the warning, and forgets it", async () => {
    for (const change of [
      (env) => env.BUCKET.put(metaKey(A), video(A, 90, { keep: true })),
      (env) => env.BUCKET.put(viewsKey(A), played(iso(NOW + DAY))),
      (env) => env.BUCKET.put(metaKey(A), video(A, 90, { state: "uploading" })),
      (env) => env.BUCKET.put(metaKey(A), video(A, 90, { progress: { done: 3, total: 9, updatedAt: iso(NOW) } })),
    ]) {
      const env = await due();
      await change(env);
      await scan(env, after);
      await autocleanTick(env, after);
      const m = env.BUCKET.json(metaKey(A));
      expect(m.state === "deleted").toBe(false);
      expect(env.BUCKET.json(STATE).going[A]).toBeUndefined();
    }
  });

  it("waits while an editor holds it, and goes once the mark runs out", async () => {
    const env = await due({ [editingKey(A)]: JSON.stringify({ until: iso(after + 10 * 60 * 1000), by: OWNER_MAC.id }) });
    await scan(env, after);
    await autocleanTick(env, after);
    expect(env.BUCKET.json(metaKey(A)).state).toBe("live");
    expect(env.BUCKET.json(STATE).going[A]).toBeDefined();
    await autocleanTick(env, after + 20 * 60 * 1000);
    expect(env.BUCKET.json(metaKey(A)).state).toBe("deleted");
  });

  it("deletes one video a run, the earliest first, and a day's round adds up", async () => {
    const env = cleanSpace({ [metaKey(A)]: video(A, 90), [metaKey(B)]: video(B, 80, { bytes: 500_000_000 }) });
    await scan(env);
    await scan(env, after);
    await autocleanTick(env, after);
    expect([env.BUCKET.json(metaKey(A)).state, env.BUCKET.json(metaKey(B)).state].filter((s) => s === "deleted")).toHaveLength(1);
    await autocleanTick(env, after + 60_000);
    expect(env.BUCKET.json(STATE).last).toMatchObject({ at: iso(after), bytes: 1_500_000_000 });
    expect(env.BUCKET.json(STATE).last.ids.sort()).toEqual([A, B].sort());
  });

  it("does nothing when off, or while counting is off", async () => {
    for (const objects of [{ [CONFIG]: JSON.stringify({ v: 1, days: 0, at: iso(NOW), since: null }) }, { ".shroom/plays.json": JSON.stringify({ on: false }) }]) {
      const env = cleanSpace({ [metaKey(A)]: video(A, 90), ...objects });
      for (let i = 0; i < 5; i++) await autocleanTick(env, after + i * 60_000);
      expect(env.BUCKET.json(metaKey(A)).state).toBe("live");
    }
  });

  it("runs from the page's schedule, with or without the AI", async () => {
    const env = await due();
    await scan(env, after);
    const waits = [];
    await worker.scheduled({ scheduledTime: after, cron: "* * * * *" }, env, { waitUntil: (p) => waits.push(p) });
    await Promise.all(waits);
    expect(env.BUCKET.json(metaKey(A)).state).toBe("deleted");
  });
});

describe("what the Macs read", () => {
  it("the cleanup answer carries each video's date and the setting; a member sees their own", async () => {
    const env = cleanSpace({ [metaKey(A)]: video(A, 90), [metaKey(B)]: video(B, 90, { owner: MHER }) });
    await ledger(env, A, M1);
    await ledger(env, B, MHER);
    await scan(env);
    const all = await bodyOf(await call(env, "POST", "/_shroom/api/cleanup", { auth: OWNER_MAC.bearer, body: { scope: "everyone" } }));
    expect(all.autoclean).toMatchObject({ days: 30, last: null });
    expect(all.items.find((i) => i.id === A).deletesAt).toBe(iso(NOW + 3 * DAY));
    const mine = await bodyOf(await call(env, "GET", "/_shroom/api/autoclean", { auth: MHER_MAC.bearer }));
    expect(mine.going).toEqual([{ id: B, deletesAt: iso(NOW + 3 * DAY) }]);
    const owner = await bodyOf(await call(env, "GET", "/_shroom/api/autoclean", { auth: OWNER_MAC.bearer }));
    expect(owner.going.map((g) => g.id).sort()).toEqual([A, B].sort());
  });

  it("a member's last round is only theirs", async () => {
    const env = cleanSpace({ [metaKey(A)]: video(A, 90), [metaKey(B)]: video(B, 90, { owner: MHER, bytes: 200 }) });
    await ledger(env, A, M1);
    await ledger(env, B, MHER);
    await scan(env);
    const after = NOW + 3 * DAY + HOUR;
    await scan(env, after);
    await autocleanTick(env, after);
    await autocleanTick(env, after + 60_000);
    const mine = await bodyOf(await call(env, "GET", "/_shroom/api/autoclean", { auth: MHER_MAC.bearer, now: after + 120_000 }));
    expect(mine.last).toMatchObject({ ids: [B], bytes: 200 });
  });
});

describe("the editor's mark", () => {
  it("the maker or owner-class marks a video as being edited, for fifteen minutes", async () => {
    const env = cleanSpace({ [metaKey(A)]: video(A, 1), [metaKey(B)]: video(B, 1) });
    await ledger(env, A, MHER);
    await ledger(env, B, M1);
    expect((await call(env, "POST", "/_shroom/api/editing", { auth: MHER_MAC.bearer, body: { rec: A } })).status).toBe(204);
    expect(env.BUCKET.json(editingKey(A))).toEqual({ until: iso(NOW + 15 * 60 * 1000), by: MHER_MAC.id });
    expect((await call(env, "POST", "/_shroom/api/editing", { auth: MHER_MAC.bearer, body: { rec: B } })).status).toBe(403);
    expect((await call(env, "POST", "/_shroom/api/editing", { auth: OWNER_MAC.bearer, body: { rec: A } })).status).toBe(204);
    expect((await call(env, "POST", "/_shroom/api/editing", { auth: OWNER_MAC.bearer, body: { rec: "nope" } })).status).toBe(400);
  });
});

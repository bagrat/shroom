// The page's last two faces (ARCHITECTURE §16 #44; DECISIONS B94): an id with no record
// reaches “…stalled” on a clock of its own, and a storage failure under the page's read is
// the unavailable face — never Cloudflare's error page, never a guess that the video is on
// its way.
import { describe, expect, it } from "vitest";
import { COPY, STALL_SECONDS, STORAGE_TRANSIENT, handle, storageCode } from "../src/worker.js";
import { M1, NOW, ORIGIN, OWNER_AIR, REC, cookieFor, metaKey, metaOf, space } from "./space-fixture.js";

const get = (env, path, init) => handle(new Request(`${ORIGIN}${path}`, init), env, NOW);
const UNKNOWN = "NoSuchVideo00000";

/** Runs a page's stall-clock script against a stand-in document, at `now`, with `store` as
 * the tab's session storage; returns what the line and the bar show afterwards. */
function runClock(body, store, now, path = `/${UNKNOWN}`) {
  const script = [...body.matchAll(/<script[^>]*>([\s\S]*?)<\/script>/g)].map((m) => m[1]).find((s) => s.includes("shroom-seen:"));
  expect(script, "the stall clock is on the page").toBeTruthy();
  const line = { textContent: body.includes(COPY.onItsWay) ? COPY.onItsWay : "" };
  const classes = new Set(["bar", "go"]);
  const bar = { classList: { remove: (c) => classes.delete(c) } };
  const document = { querySelector: (q) => (q.includes("line") ? line : q === ".bar" ? bar : null) };
  const sessionStorage = store === null ? null : { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) };
  const DateStub = { now: () => now };
  new Function("document", "sessionStorage", "location", "Date", script)(document, sessionStorage, { pathname: path }, DateStub);
  return { line: line.textContent, resting: !classes.has("go") };
}

describe("an id with no record reaches “…stalled” (N40: “a mistyped link lands here too”)", () => {
  for (const [face, path] of [["the page", `/${UNKNOWN}`], ["the embed", `/${UNKNOWN}?embed=1`]]) {
    it(`${face}: on its way at first, stalled once the tab has watched it for STALL_SECONDS`, async () => {
      const body = await (await get(space(), path)).text();
      expect(body).toContain(COPY.onItsWay);
      const store = new Map();
      const t0 = 1_000_000;
      expect(runClock(body, store, t0)).toEqual({ line: COPY.onItsWay, resting: false });
      // The page refreshes itself every 5 s: the clock is the tab's, not the load's.
      expect(runClock(body, store, t0 + 60_000)).toEqual({ line: COPY.onItsWay, resting: false });
      expect(runClock(body, store, t0 + STALL_SECONDS * 1000)).toEqual({ line: COPY.onItsWay, resting: false });
      expect(runClock(body, store, t0 + STALL_SECONDS * 1000 + 1)).toEqual({ line: COPY.stalled, resting: true });
      // Another address in the same tab starts its own clock.
      expect(runClock(body, store, t0 + STALL_SECONDS * 1000 + 1, "/AnotherOne000000")).toEqual({ line: COPY.onItsWay, resting: false });
    });
  }

  it("without session storage (a blocked frame) the page simply stays on its way", async () => {
    const body = await (await get(space(), `/${UNKNOWN}`)).text();
    expect(runClock(body, null, 5_000_000)).toEqual({ line: COPY.onItsWay, resting: false });
  });

  it("a recording with a record keeps the server's own clock (its progress), never the tab's", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { state: "uploading", media: null, progress: { done: 1, total: 4, updatedAt: new Date(NOW).toISOString() } }) } });
    expect(await (await get(env, `/${REC}`)).text()).not.toContain("shroom-seen:");
    expect(await (await get(env, `/${REC}?embed=1`)).text()).not.toContain("shroom-seen:");
  });

  it("the stalled words are the ratified ones, carried safely into the script", async () => {
    const body = await (await get(space(), `/${UNKNOWN}`)).text();
    expect(body).toContain(JSON.stringify(COPY.stalled).replace(/[<>&]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`));
  });
});

/** A space whose bucket throws R2's own binding error for the record's read — or for the
 * reads of `keys`; `null` is every read. */
function failing(code, keys = [metaKey(REC)]) {
  const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
  const real = env.BUCKET;
  env.BUCKET = new Proxy(real, {
    get(target, prop) {
      if (prop === "get") return async (key, ...rest) => {
        if (!keys || keys.includes(key)) throw new Error(code === null ? "get: network connection lost" : `get: We encountered an internal error. Please try again. (${code})`);
        return target.get(key, ...rest);
      };
      const v = Reflect.get(target, prop);
      return typeof v === "function" ? v.bind(target) : v;
    },
  });
  return env;
}

describe("which storage failures mean unavailable (r4, N27): every one under the record's read", () => {
  it("reads R2's code from the binding's message — Cloudflare's shape, and the local workerd's", () => {
    expect(storageCode(new Error("get: We encountered an internal error. Please try again. (10001)"))).toBe(10001);
    expect(storageCode(new Error("get: The specified object name is not valid. (10020)"))).toBe(10020);
    expect(storageCode(new Error("put: Your metadata headers exceed the maximum allowed metadata size. (10012)"))).toBe(10012);
    expect(storageCode(new Error("network connection lost"))).toBe(null);
    expect(storageCode(undefined)).toBe(null);
    expect(STORAGE_TRANSIENT).toEqual([10001, 10043, 10054, 10058]);
  });

  for (const code of [...STORAGE_TRANSIENT, 10020, null]) {
    it(`a record read failing with ${code ?? "no code"} shows the unavailable face on the page, the embed and the poll`, async () => {
      const env = failing(code);
      for (const [path, init] of [[`/${REC}`, undefined], [`/${REC}`, { headers: { cookie: cookieFor(M1, OWNER_AIR) } }], [`/${REC}?embed=1`, undefined]]) {
        const res = await get(env, path, init);
        expect(res.status, path).toBe(200);
        expect(res.headers.get("x-shroom-unavailable")).toBe(code === null ? "storage" : `storage-${code}`);
        const body = await res.text();
        expect(body).toContain(COPY.unavailable);
        expect(body).toContain(`<meta http-equiv="refresh" content="30">`);
        expect(body).not.toContain(COPY.onItsWay);
        expect(body).not.toContain("internal error");
      }
      expect(await (await get(env, `/_shroom/status/${REC}`)).json()).toEqual({ state: "unavailable" });
      expect((await get(env, `/d/${REC}`)).status).toBe(503);
    });
  }

  it("a mark that can't be read never takes the page down: the record decides the face", async () => {
    const env = failing(10043, [".shroom/mark.json"]);
    const body = await (await get(env, `/${REC}`)).text();
    expect(body).not.toContain(COPY.unavailable);
    expect(body).toContain(`class="player"`);
    expect(await (await get(env, `/${REC}?embed=1`)).text()).toContain(`class="player"`);
  });

  // The switch governs every face of a video page but this one (DECISIONS B108): when the
  // space can't be read, its setting is part of what couldn't be — whether the read that
  // failed was the record's alone, with the switch itself still readable, or every read.
  it("the page shown when the space can't be read carries the mark whatever the switch says", async () => {
    for (const [down, keys] of [["the record's read alone", [metaKey(REC)]], ["every read", null]]) {
      for (const on of [true, false]) {
        const env = failing(10043, keys);
        await env.BUCKET.put(".shroom/mark.json", JSON.stringify({ on, at: "2026-10-02T11:00:00Z" }));
        for (const [who, init] of [["a recipient", undefined], ["an owner-class browser", { headers: { cookie: cookieFor(M1, OWNER_AIR) } }]]) {
          const what = `${who} — ${down} failing, the switch ${on ? "on" : "off"}`;
          const res = await get(env, `/${REC}`, init);
          expect(res.headers.get("x-shroom-unavailable"), what).toBe("storage-10043");
          const body = await res.text();
          expect(body, what).toContain(COPY.unavailable);
          const logo = body.match(/<div class="foot">[\s\S]*?<\/div>/)?.[0].match(/<a class="mark"[^>]*>/)?.[0];
          expect(logo ?? "no mark", what).toContain(`title="${COPY.mark}"`);
          expect(logo, what).not.toMatch(/\shidden/);
        }
      }
    }
  });
});

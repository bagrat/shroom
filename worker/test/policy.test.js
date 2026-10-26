// Every HTML page the Worker serves carries a script policy: a fresh nonce per response,
// named in its Content-Security-Policy and carried by each inline <script> the Worker
// writes, with the player's own script (/_shroom/hls.js) allowed by origin. Plugins, a
// <base> and form posts are off; framing stays as it was (only the embed may be framed).
import { describe, expect, it } from "vitest";
import { COPY } from "../src/worker.js";
import { CODE, M1, OWNER_AIR, REC, call, cookieFor, ledger, metaKey, metaOf, space } from "./space-fixture.js";

/** The policy's directives, by name. @param {string | null} csp */
function directives(csp) {
  /** @type {Record<string, string[]>} */
  const out = {};
  for (const part of String(csp ?? "").split(";")) {
    const [name, ...values] = part.trim().split(/\s+/);
    if (name) out[name] = values;
  }
  return out;
}

/** A space whose every storage read throws: the unavailable faces. */
function failing() {
  const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
  const real = env.BUCKET;
  env.BUCKET = new Proxy(real, {
    get(target, prop) {
      if (prop === "get") return async () => { throw new Error("get: We encountered an internal error. Please try again. (10001)"); };
      const v = Reflect.get(target, prop);
      return typeof v === "function" ? v.bind(target) : v;
    },
  });
  return env;
}

const owner = { headers: { cookie: cookieFor(M1, OWNER_AIR) } };
const live = (over = {}) => metaOf(REC, { durationSec: 180, captions: { language: "en" }, named: { by: "ai", title: "Spring plan", chapters: [[0, "Start"], [60, "Middle"], [120, "End"]] }, ...over });

/** [what, env, path, init, frameable, status, has scripts] for every face the Worker renders. */
async function faces() {
  const mine = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
  await ledger(mine, REC, M1);
  mine.AI = {}; // a space with its AI on, the name still on its way: the owner page's naming script
  const pending = space();
  await ledger(pending, REC, M1);
  return [
    ["the viewer's page", space({ objects: { [metaKey(REC)]: live() } }), `/${REC}`, undefined, false, 200, true],
    ["the owner page", mine, `/${REC}`, owner, false, 200, true],
    ["the owner page before its record", pending, `/${REC}`, owner, false, 200, true],
    ["the page on its way", space({ objects: { [metaKey(REC)]: metaOf(REC, { state: "uploading", media: null }) } }), `/${REC}`, undefined, false, 200, false],
    ["an unknown id", space(), `/NoSuchVideo00000`, undefined, false, 200, true],
    ["the removed page", space({ objects: { [metaKey(REC)]: metaOf(REC, { state: "deleted" }) } }), `/${REC}`, undefined, false, 404, false],
    ["the unavailable page", failing(), `/${REC}`, undefined, false, 200, false],
    ["the embed", space({ objects: { [metaKey(REC)]: live() } }), `/${REC}?embed=1`, undefined, true, 200, true],
    ["the embed of an unknown id", space(), `/NoSuchVideo00000?embed=1`, undefined, true, 200, true],
    ["the unavailable embed", failing(), `/${REC}?embed=1`, undefined, true, 200, false],
    ["the join page", space(), `/join/${CODE.ani}`, undefined, false, 200, true],
    ["the join page's line", space(), `/join/${CODE.unknown}`, undefined, false, 200, false],
  ];
}

describe("an HTML page's script policy", () => {
  it("names a fresh nonce that every inline script on the page carries, and nothing else runs inline", async () => {
    for (const [what, env, path, init, frameable, code, scripted] of await faces()) {
      const res = await call(env, "GET", path, init);
      expect(res.status, what).toBe(code);
      expect(res.headers.get("content-type"), what).toMatch(/^text\/html/);
      const d = directives(res.headers.get("content-security-policy"));
      const nonces = (d["script-src"] ?? []).filter((v) => /^'nonce-/.test(v));
      expect(nonces, `${what}: one nonce`).toHaveLength(1);
      const nonce = nonces[0].slice("'nonce-".length, -1);
      expect(nonce, `${what}: a nonce of at least 128 bits`).toMatch(/^[A-Za-z0-9+/_-]{22,}={0,2}$/);
      expect(d["script-src"], what).toEqual([`'nonce-${nonce}'`, "'self'"]);
      const body = await res.text();
      const tags = [...body.matchAll(/<script\b[^>]*>/gi)].map((m) => m[0]);
      if (scripted) expect(tags.length, `${what}: has its scripts`).toBeGreaterThan(0);
      for (const tag of tags) expect(tag, what).toBe(`<script nonce="${nonce}">`);
      // Outside its scripts, the markup runs nothing: no handler attributes, no script URLs.
      const markup = body.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, "");
      expect(markup, `${what}: no inline handlers`).not.toMatch(/<[a-z][^>]*\son[a-z]+\s*=/i);
      expect(markup, `${what}: no script URLs`).not.toMatch(/javascript:/i);
    }
  });

  it("turns off plugins, <base> and form posts, keeps framing as it was, and lets the player's worker start", async () => {
    for (const [what, env, path, init, frameable] of await faces()) {
      const res = await call(env, "GET", path, init);
      const d = directives(res.headers.get("content-security-policy"));
      expect(d["object-src"], what).toEqual(["'none'"]);
      expect(d["base-uri"], what).toEqual(["'none'"]);
      expect(d["form-action"], what).toEqual(["'none'"]);
      expect(d["frame-ancestors"], what).toEqual([frameable ? "*" : "'none'"]);
      // hls.js starts its transmuxer from a blob: URL; without this the script policy would refuse it.
      expect(d["worker-src"], what).toEqual(["'self'", "blob:"]);
      expect(res.headers.get("x-content-type-options"), what).toBe("nosniff");
    }
  });

  it("gives every response a nonce of its own", async () => {
    const env = space({ objects: { [metaKey(REC)]: live() } });
    const seen = new Set();
    for (let i = 0; i < 5; i++) seen.add(directives((await call(env, "GET", `/${REC}`)).headers.get("content-security-policy"))["script-src"][0]);
    expect(seen.size).toBe(5);
  });

  it("covers the owner page's naming script and the player's chapters and captions", async () => {
    const [, mine, , init] = (await faces())[1];
    const body = await (await call(mine, "GET", `/${REC}`, init)).text();
    expect(body).toContain("/_shroom/named/");
    const viewer = await (await call(space({ objects: { [metaKey(REC)]: live() } }), "GET", `/${REC}`)).text();
    expect(viewer).toContain(`/_shroom/cc/${REC}`);
    expect(viewer).toContain("shroomChapters");
    expect(viewer).toContain(COPY.captions);
  });
});

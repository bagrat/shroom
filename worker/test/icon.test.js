// The tab icon (DECISIONS B109; Bagrat, on the preview: “the player pages do not have the
// favicon”). Every page the Worker renders names the icon in its head the way the landing
// does, and the Worker answers the icon's three addresses itself with the landing's art,
// byte for byte, carried in worker.js — no storage read, no cookie, and never taken for a
// video's id. The icon is there whatever the “made with shroom” switch says: it isn't the
// mark, and without one a browser draws a blank.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { COPY, STALL_SECONDS, handle } from "../src/worker.js";
import { CODE, M1, MHER, MHER_MAC, NOW, ORIGIN, OWNER_AIR, REC, UA, cookieFor, ledger, metaKey, metaOf, space, textOf } from "./space-fixture.js";

const site = (name) => readFileSync(new URL(`../../site/${name}`, import.meta.url));
const SVG = site("favicon.svg");
const PNG = site("favicon.png");
/** Browsers ask for /favicon.ico unprompted; it answers with the PNG. */
const ROUTES = [
  ["/favicon.svg", "image/svg+xml", SVG, "favicon.svg"],
  ["/favicon.png", "image/png", PNG, "favicon.png"],
  ["/favicon.ico", "image/png", PNG, "favicon.png"],
];
const MONTH_SECONDS = 30 * 24 * 3600;

/** The landing's three lines, their addresses rooted: the landing lives at `/`, a page at
 * `/<id>` or `/join/<code>`. */
const LINKS = (site("index.html").toString("utf8").match(/<link rel="(?:icon|apple-touch-icon)"[^>]*>/g) ?? [])
  .map((l) => l.replace(`href="favicon.`, `href="/favicon.`));

const get = (env, path, init) => handle(new Request(`${ORIGIN}${path}`, init), env, NOW);
const ownerCookie = () => cookieFor(M1, OWNER_AIR);
const uploading = (ageSec) => metaOf(REC, { state: "uploading", media: null, progress: { done: 1, total: 4, updatedAt: new Date(NOW - ageSec * 1000).toISOString() } });
const withRecord = (meta, more = {}) => space({ objects: { [metaKey(REC)]: meta, ...more } });

/** A bucket no call to which ever answers: an icon that read storage would fail here. */
const noStorage = () => ({ BUCKET: new Proxy({}, { get: (_, op) => () => { throw new Error(`the icon read storage: ${String(op)}`); } }) });

/** The page names the icon once, in its head, the landing's three lines in the landing's
 * order — and nothing in its policy could refuse an image of its own. */
async function expectIcon(res, what, code = 200) {
  expect(res.status, what).toBe(code);
  expect(res.headers.get("content-type"), what).toMatch(/^text\/html/);
  expect(res.headers.get("content-security-policy") ?? "", what).not.toMatch(/(^|;)\s*(default-src|img-src)\b/);
  const html = await res.text();
  const head = html.slice(html.indexOf("<head>"), html.indexOf("</head>"));
  let at = -1;
  for (const link of LINKS) {
    expect(head.split(link).length - 1, `${what}: ${link}`).toBe(1);
    expect(head.indexOf(link), `${what}: the landing's order`).toBeGreaterThan(at);
    at = head.indexOf(link);
  }
  expect((html.match(/<link rel="[^"]*icon[^"]*"/g) ?? []).length, `${what}: no other icon`).toBe(LINKS.length);
  return html;
}

describe("the icon's three addresses", () => {
  for (const [path, type, art, file] of ROUTES) {
    it(`${path} is the landing's art, byte for byte`, async () => {
      const res = await get(space(), path);
      expect(res.status).toBe(200);
      expect(res.headers.get("content-type")).toBe(type);
      expect(res.headers.get("content-length")).toBe(String(art.length));
      const body = Buffer.from(await res.arrayBuffer());
      expect(body.equals(art), `${path} is not site/${file}: worker.js carries its own copy of the landing's art ` +
        `(the SVG as text, the PNG as base64 — \`base64 -b 120 -i site/favicon.png\`), and the two must stay the same bytes`).toBe(true);
    });

    it(`${path} is kept by the browser for a long time, and is no page for a search engine`, async () => {
      const res = await get(space(), path);
      const cache = res.headers.get("cache-control") ?? "";
      expect(cache).toMatch(/\bpublic\b/);
      expect(Number(cache.match(/max-age=(\d+)/)?.[1] ?? 0)).toBeGreaterThanOrEqual(MONTH_SECONDS);
      expect(res.headers.get("x-robots-tag")).toBe("noindex");
      expect(res.headers.get("set-cookie")).toBeNull();
    });

    it(`${path} answers HEAD with the same headers and no body`, async () => {
      const res = await get(space(), path, { method: "HEAD" });
      expect(res.status).toBe(200);
      expect(res.headers.get("content-type")).toBe(type);
      expect(res.headers.get("content-length")).toBe(String(art.length));
      expect(res.headers.get("cache-control")).toBe((await get(space(), path)).headers.get("cache-control"));
      expect((await res.arrayBuffer()).byteLength).toBe(0);
    });

    it(`${path} reads no storage and needs no cookie — with one, without one, on a space that can't answer`, async () => {
      const env = space();
      for (const init of [undefined, { headers: { cookie: ownerCookie() } }, { headers: { cookie: "shroom_owner=anything" } }, { method: "HEAD" }]) {
        const res = await get(env, path, init);
        expect(res.status).toBe(200);
        expect(res.headers.get("content-type")).toBe(type);
      }
      expect(env.BUCKET.ops).toEqual([]);
      const res = await get(noStorage(), path);
      expect(res.status).toBe(200);
      expect(Buffer.from(await res.arrayBuffer()).equals(art)).toBe(true);
    });

    it(`${path} is the icon whatever rides the address — never a key to redeem, never the embed face`, async () => {
      for (const query of ["?k=kKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKk", "?embed=1", "?v=2"]) {
        const res = await get(noStorage(), `${path}${query}`);
        expect(res.status, query).toBe(200);
        expect(res.headers.get("content-type"), query).toBe(type);
        expect(res.headers.get("location"), query).toBeNull();
      }
    });

    it(`${path} takes no writes`, async () => {
      for (const method of ["POST", "PUT", "DELETE"]) expect((await get(space(), path, { method })).status, method).toBe(405);
    });
  }
});

describe("the watch page names its icon, on every face", () => {
  const failing = () => {
    const env = withRecord(metaOf(REC));
    const real = env.BUCKET.get.bind(env.BUCKET);
    env.BUCKET.get = async (key, ...rest) => {
      if (key === metaKey(REC)) throw new Error("get: We encountered an internal error. Please try again. (10001)");
      return real(key, ...rest);
    };
    return env;
  };
  const off = { ".shroom/mark.json": JSON.stringify({ on: false, at: "2026-10-02T11:00:00Z" }) };
  /** [the face, its space, the request's headers, what shows it is that face, what must be absent] */
  const faces = [
    ["the recipient's live page", () => withRecord(metaOf(REC)), {}, [`class="player"`], [`class="title"`]],
    ["the owner's live page", () => withRecord(metaOf(REC)), { cookie: ownerCookie() }, [`class="title"`, `id="mk"`, `class="player"`], []],
    ["the owner's page while it uploads", () => withRecord(uploading(10)), { cookie: ownerCookie() }, [`class="title"`, COPY.finishing], []],
    ["on its way", () => withRecord(uploading(10)), {}, [COPY.onItsWay], [`class="title"`]],
    ["stalled", () => withRecord(uploading(STALL_SECONDS + 1)), {}, [COPY.stalled], [COPY.onItsWay]],
    ["an id with no record", () => space(), {}, [COPY.onItsWay], [`<h1>`]],
    ["removed", () => withRecord(JSON.stringify({ schema: 1, id: REC, state: "deleted" })), {}, [COPY.notFound], [`class="player"`]],
    ["removed, to its owner", () => withRecord(JSON.stringify({ schema: 1, id: REC, state: "deleted" })), { cookie: ownerCookie() }, [COPY.notFound], [`class="title"`]],
    ["unavailable", () => withRecord(metaOf(REC, { media: null })), {}, [COPY.unavailable], [`class="player"`]],
    ["unavailable, the record unreadable", failing, {}, [COPY.unavailable], [COPY.onItsWay]],
    ["the recipient's live page, the mark switched off", () => withRecord(metaOf(REC), off), {}, [`class="player"`], [`class="foot"`]],
    ["removed, the mark switched off", () => withRecord(JSON.stringify({ schema: 1, id: REC, state: "deleted" }), off), {}, [COPY.notFound], [`class="foot"`]],
    ["the owner's live page, the mark switched off", () => withRecord(metaOf(REC), off), { cookie: ownerCookie() }, [`class="title"`, `id="mk"`], []],
  ];
  for (const [face, env, headers, shows, lacks] of faces) {
    it(face, async () => {
      // A removed video is a plain not found (B141): 404, with the page's icon all the same.
      const html = await expectIcon(await get(env(), `/${REC}`, { headers }), face, face.startsWith("removed") ? 404 : 200);
      for (const s of shows) expect(html, face).toContain(s);
      for (const s of lacks) expect(html, face).not.toContain(s);
    });
  }

  it("the owner's page before its record lands", async () => {
    const env = space();
    await ledger(env, REC, M1);
    const html = await expectIcon(await get(env, `/${REC}`, { headers: { cookie: ownerCookie() } }), "pending");
    expect(html).toContain(`class="title"`);
  });

  it("a member's own page", async () => {
    const env = withRecord(metaOf(REC));
    await ledger(env, REC, MHER, MHER_MAC.id);
    const html = await expectIcon(await get(env, `/${REC}`, { headers: { cookie: cookieFor(MHER, MHER_MAC) } }), "a member's page");
    expect(html).toContain(`class="title"`);
    expect(html).not.toContain(`id="mk"`);
  });

  it("after the tags a link preview reads — those stay first in the head", async () => {
    const html = await expectIcon(await get(withRecord(metaOf(REC)), `/${REC}`), "live");
    expect(html.indexOf("og:type")).toBeGreaterThan(-1);
    expect(html.indexOf("og:type")).toBeLessThan(html.indexOf(LINKS[0]));
    expect(html.lastIndexOf("application/json+oembed")).toBeLessThan(html.indexOf(LINKS[0]));
    expect(html.indexOf(LINKS[2])).toBeLessThan(html.indexOf("<style>"));
  });
});

describe("the join page names its icon, on every face", () => {
  const join = (env, code, { ua = UA.mac, cookie } = {}) =>
    get(env, `/join/${code}`, { headers: { "user-agent": ua, ...(cookie ? { cookie } : {}) } });
  /** [the face, the code, the request, a line only that face says] */
  const faces = [
    ["an invite, on a Mac", CODE.ani, {}, COPY.getShroom],
    ["an invite, on a phone", CODE.ani, { ua: UA.iphone }, COPY.elsewhere],
    ["an invite, as its owner sees it", CODE.ani, { cookie: ownerCookie() }, "doesn't use it up"],
    ["an own-Mac link", CODE.own, {}, COPY.ownHero],
    ["an invite past its seven days", CODE.old, {}, "This invite has expired"],
    ["an own-Mac link past its hour", CODE.oldOwn, {}, COPY.ownExpired],
    ["a cancelled invite", CODE.cancelled, {}, "This invite has expired"],
    ["an invite already used", CODE.used, {}, "This invite was already used"],
    ["an invite already used, to the one it let in", CODE.used, { cookie: cookieFor(MHER, MHER_MAC) }, COPY.youreIn],
    ["a code nobody made", CODE.unknown, {}, COPY.inviteUnknown],
    ["a code that is no code", "favicon.ico", {}, COPY.inviteUnknown],
  ];
  for (const [face, code, opts, line] of faces) {
    it(face, async () => {
      const html = await expectIcon(await join(space(), code, opts), face);
      expect(textOf(html), face).toContain(line);
    });
  }

  it("a space whose members can't be read right now", async () => {
    const html = await expectIcon(await join(space({ roster: null }), CODE.ani), "transient");
    expect(textOf(html)).toContain(COPY.inviteTransient);
  });
});

describe("nothing else about the addresses changed", () => {
  it("the page declares what the landing declares", () => {
    expect(LINKS).toEqual([
      `<link rel="icon" type="image/svg+xml" href="/favicon.svg">`,
      `<link rel="icon" type="image/png" sizes="180x180" href="/favicon.png">`,
      `<link rel="apple-touch-icon" href="/favicon.png">`,
    ]);
  });

  it("a name that only looks like the icon's is an unknown video, on its way like any other", async () => {
    const unknown = await (await get(space(), "/zzzzzzzzzzzzzzzz")).text();
    expect(unknown).toContain(COPY.onItsWay);
    for (const path of ["/favicon", "/favicon.gif", "/favicons.svg", "/favicon.svg.bak", "/FAVICON.ICO"]) {
      const res = await get(space(), path);
      expect(res.status, path).toBe(200);
      expect(res.headers.get("content-type"), path).toMatch(/^text\/html/);
      expect(await res.text(), path).toBe(unknown);
    }
  });

  it("deeper addresses still answer as they did: an empty 404, the media route's own, the join page", async () => {
    const env = withRecord(metaOf(REC));
    for (const path of ["/x/favicon.svg", "/favicon.svg/x", "/favicon.ico/", `/m/${REC}/favicon.png`, `/d/favicon.ico`]) {
      const res = await get(env, path);
      expect(res.status, path).toBe(404);
      expect(res.headers.get("content-type"), path).toBe("text/plain; charset=utf-8");
      expect(await res.text(), path).toBe("");
    }
    const invite = await get(env, "/join/favicon.svg");
    expect(invite.headers.get("content-type")).toMatch(/^text\/html/);
    expect(textOf(await invite.text())).toContain(COPY.inviteUnknown);
  });

  it("a real video's page, its bytes and the probe are untouched by an icon request beside them", async () => {
    const env = withRecord(metaOf(REC), { [`rec/${REC}/master.m3u8`]: "#EXTM3U\n" });
    for (const [path] of ROUTES) await get(env, path);
    expect(await (await get(env, `/${REC}`)).text()).toContain(`/m/${REC}/master.m3u8`);
    expect(await (await get(env, `/m/${REC}/master.m3u8`)).text()).toBe("#EXTM3U\n");
    expect(await (await get(env, "/_shroom/ping")).json()).toMatchObject({ shroom: true });
  });
});

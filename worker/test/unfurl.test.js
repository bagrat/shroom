// Unfurls, oEmbed and the embed face (R5.2–R5.3, §5.4): the tags crawlers read from the
// raw HTML, og:video as the small teaser (never HLS), the oEmbed iframe, and the one
// face that may be framed.
import { describe, expect, it } from "vitest";
import { COPY, handle } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";

const ID = "AbCdEfGhIjKlMnOp";
const ORIGIN = "https://v.example.workers.dev";
const NOW = Date.parse("2026-10-02T12:00:00Z");
const get = (env, path, headers = {}) => handle(new Request(`${ORIGIN}${path}`, { headers }), env, NOW);
const metaOf = (over = {}, media = {}) => JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: "2026-10-02T11:00:00Z", title: "Q3 plan & “next”",
  durationSec: 272.4, media: { hls: `rec/${ID}/master.m3u8`, mp4: null, poster: `rec/${ID}/poster.jpg`, teaser: `rec/${ID}/teaser.mp4`, width: 1920, height: 1080, ...media }, ...over });
const env = (meta = metaOf()) => ({ BUCKET: fakeBucket({ [`meta/${ID}.json`]: meta }) });
const BOTS = ["Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots)", "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)", "Twitterbot/1.0", "facebookexternalhit/1.1"];

describe("unfurl tags", () => {
  it("carry the title, the poster and the teaser — the same for every crawler", async () => {
    for (const ua of BOTS) {
      const body = await (await get(env(), `/${ID}`, { "user-agent": ua })).text();
      expect(body).toContain(`<meta property="og:type" content="video.other">`);
      expect(body).toContain(`<meta property="og:title" content="Q3 plan &amp; “next”">`);
      expect(body).toContain(`<meta property="og:url" content="${ORIGIN}/${ID}">`);
      expect(body).toContain(`<meta property="og:image" content="${ORIGIN}/m/${ID}/poster.jpg"><meta property="og:image:width" content="1200"><meta property="og:image:height" content="675">`);
      expect(body).toContain(`<meta property="og:video" content="${ORIGIN}/m/${ID}/teaser.mp4">`);
      expect(body).toContain(`<meta property="og:video:type" content="video/mp4"><meta property="og:video:width" content="640"><meta property="og:video:height" content="360"><meta property="og:video:duration" content="272">`);
      expect(body).toContain(`<meta name="twitter:card" content="summary_large_image">`);
      expect(body).toContain(`<meta name="twitter:player:stream" content="${ORIGIN}/m/${ID}/teaser.mp4">`);
      expect(body).toContain(`<link rel="alternate" type="application/json+oembed" href="${ORIGIN}/oembed?url=${encodeURIComponent(`${ORIGIN}/${ID}`)}">`);
      expect(body).not.toMatch(/og:video"[^>]*m3u8/);
      expect(body).not.toContain("og:description");
      expect(body.indexOf("og:type")).toBeLessThan(body.indexOf("<style>"));
    }
  });

  it("leave out what doesn't exist yet, and say nothing for unknown ids or tombstones", async () => {
    const bare = await (await get(env(metaOf({}, { poster: null, teaser: null })), `/${ID}`)).text();
    expect(bare).toContain("og:title");
    expect(bare).not.toContain("og:image");
    expect(bare).not.toContain("og:video");
    const uploading = await (await get(env(metaOf({ state: "uploading", media: null })), `/${ID}`)).text();
    expect(uploading).toContain(`<meta property="og:title" content="Q3 plan &amp; “next”">`);
    for (const e of [{ BUCKET: fakeBucket() }, env(JSON.stringify({ schema: 1, id: ID, state: "deleted", title: "Secret" }))]) {
      const body = await (await get(e, `/${ID}`)).text();
      expect(body).not.toContain("og:");
      expect(body).not.toContain("Secret");
    }
  });

  it("refuse framing on the page itself", async () => {
    const res = await get(env(), `/${ID}`);
    expect(res.headers.get("content-security-policy")).toMatch(/(^|;)\s*frame-ancestors 'none'\s*(;|$)/);
  });
});

describe("oEmbed", () => {
  it("answers this space's own links with an iframe of the embed face", async () => {
    const res = await get(env(), `/oembed?url=${encodeURIComponent(`${ORIGIN}/${ID}`)}`);
    const body = await res.json();
    expect(body).toMatchObject({ version: "1.0", type: "video", title: "Q3 plan & “next”", width: 1280, height: 720, thumbnail_url: `${ORIGIN}/m/${ID}/poster.jpg` });
    expect(body.html).toContain(`src="${ORIGIN}/${ID}?embed=1"`);
    expect(res.headers.get("x-robots-tag")).toBe("noindex");
  });

  it("refuses other sites' links and leaks no title for unknown ids", async () => {
    expect((await get(env(), `/oembed?url=${encodeURIComponent(`https://evil.example/${ID}`)}`)).status).toBe(404);
    expect((await get(env(), `/oembed?url=nonsense`)).status).toBe(404);
    const unknown = await (await get({ BUCKET: fakeBucket() }, `/oembed?url=${encodeURIComponent(`${ORIGIN}/${ID}`)}`)).json();
    expect(unknown.title).toBeUndefined();
    expect(unknown.html).toContain("?embed=1");
  });
});

describe("the embed face", () => {
  it("is the player alone, framable, with the mark small", async () => {
    const res = await get(env(), `/${ID}?embed=1`);
    expect(res.headers.get("content-security-policy")).toMatch(/(^|;)\s*frame-ancestors \*\s*(;|$)/);
    expect(res.headers.get("x-frame-options")).toBeNull();
    const body = await res.text();
    expect(body).toContain(`/m/${ID}/master.m3u8`);
    expect(body).toContain(`poster="${ORIGIN}/m/${ID}/poster.jpg"`);
    expect(body).not.toContain("<h1>");
    expect(body).toContain(`title="${COPY.mark}"`);
  });

  it("keeps on-its-way, removed and unavailable, and never shows owner mode", async () => {
    expect(await (await get({ BUCKET: fakeBucket() }, `/${ID}?embed=1`)).text()).toContain(COPY.onItsWay);
    expect(await (await get(env(JSON.stringify({ schema: 1, id: ID, state: "deleted" })), `/${ID}?embed=1`)).text()).toContain(COPY.notFound);
    expect(await (await get(env(metaOf({ state: "live", media: null })), `/${ID}?embed=1`)).text()).toContain(COPY.unavailable);
    const owned = await (await get(env(), `/${ID}?embed=1`, { cookie: "shroom_owner=anything" })).text();
    expect(owned).not.toContain(`class="title"`);
  });
});

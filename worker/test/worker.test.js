import { describe, expect, it } from "vitest";
import { COPY, STALL_SECONDS, escapeHtml, escapeText, handle } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";

const ID = "AbCdEfGhIjKlMnOp";
const NOW = Date.parse("2026-10-02T12:00:00Z");
const get = (env, path, init) => handle(new Request(`https://v.example.workers.dev${path}`, init), env, NOW);
const metaOf = (over) => JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: "2026-10-02T11:00:00Z", title: "Untitled — Oct 2",
  media: { hls: `rec/${ID}/master.m3u8`, mp4: null, width: 1920, height: 1080 }, ...over });

describe("the page", () => {
  it("never 404s an unknown id — it looks like a video on its way, with no title", async () => {
    for (const id of [ID, "short", "not-a-real-id-at-all"]) {
      const res = await get({ BUCKET: fakeBucket() }, `/${id}`);
      expect(res.status).toBe(200);
      const body = await res.text();
      expect(body).toContain(COPY.onItsWay);
      expect(body).not.toContain("<h1>");
    }
  });

  it("shows the title while uploading, and calls it stalled after the window", async () => {
    const fresh = fakeBucket({ [`meta/${ID}.json`]: metaOf({ state: "uploading", progress: { done: 1, total: 4, updatedAt: "2026-10-02T11:59:30Z" } }) });
    const body = await (await get({ BUCKET: fresh }, `/${ID}`)).text();
    expect(body).toContain("<h1>Untitled — Oct 2</h1>");
    expect(body).toContain(COPY.onItsWay);

    const old = new Date(NOW - (STALL_SECONDS + 1) * 1000).toISOString();
    const stale = fakeBucket({ [`meta/${ID}.json`]: metaOf({ state: "uploading", progress: { done: 1, total: 4, updatedAt: old } }) });
    expect(await (await get({ BUCKET: stale }, `/${ID}`)).text()).toContain(COPY.stalled);
  });

  it("plays a live video from its master playlist, offering Download only when the file exists", async () => {
    const live = fakeBucket({ [`meta/${ID}.json`]: metaOf({}) });
    const body = await (await get({ BUCKET: live }, `/${ID}`)).text();
    expect(body).toContain(`/m/${ID}/master.m3u8`);
    expect(body).not.toContain(COPY.download);
    const withMp4 = fakeBucket({ [`meta/${ID}.json`]: metaOf({ media: { hls: "x", mp4: `rec/${ID}/video.mp4`, width: 1, height: 1 } }) });
    expect(await (await get({ BUCKET: withMp4 }, `/${ID}`)).text()).toContain(`<a href="/d/${ID}">${COPY.download}</a>`);
  });

  it("shows only the removed line for a tombstone — no title", async () => {
    const gone = fakeBucket({ [`meta/${ID}.json`]: JSON.stringify({ schema: 1, id: ID, state: "deleted", title: "Secret plans" }) });
    const body = await (await get({ BUCKET: gone }, `/${ID}`)).text();
    expect(body).toContain(COPY.notFound);
    expect(body).not.toContain("Secret plans");
  });

  it("escapes titles", async () => {
    const evil = fakeBucket({ [`meta/${ID}.json`]: metaOf({ title: `</script><script>alert(1)</script>` }) });
    const body = await (await get({ BUCKET: evil }, `/${ID}`)).text();
    expect(body).not.toContain("<script>alert(1)");
    expect(escapeHtml(`<a href="x">'&`)).toBe("&lt;a href=&quot;x&quot;&gt;&#39;&amp;");
    expect(escapeText(`<b>it's & "so"</b>`)).toBe(`&lt;b&gt;it's &amp; "so"&lt;/b&gt;`);
  });

  it("marks every response noindex", async () => {
    const env = { BUCKET: fakeBucket({ [`meta/${ID}.json`]: metaOf({}), [`rec/${ID}/master.m3u8`]: "#EXTM3U" }) };
    for (const path of [`/${ID}`, `/m/${ID}/master.m3u8`, "/nope/nope", `/m/${ID}/missing.m4s`]) {
      expect((await get(env, path)).headers.get("x-robots-tag")).toBe("noindex");
    }
  });
});

describe("media", () => {
  const bytes = new Uint8Array(1000).map((_, i) => i % 256);
  const env = () => ({ BUCKET: fakeBucket({ [`rec/${ID}/v0/seg_00000.m4s`]: bytes, ".shroom/player/hls.min.js": "/*hls*/" }) });

  it("serves a file with its type and length", async () => {
    const res = await get(env(), `/m/${ID}/v0/seg_00000.m4s`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("video/iso.segment");
    expect((await res.arrayBuffer()).byteLength).toBe(1000);
  });

  it("never lets a recording's bytes be sniffed or opened as a page of the space (B125)", async () => {
    const res = await get(env(), `/m/${ID}/v0/seg_00000.m4s`);
    expect(res.headers.get("x-content-type-options")).toBe("nosniff");
    expect(res.headers.get("content-security-policy")).toBe("sandbox; default-src 'none'");
  });

  it("lets only the viewer's own browser keep a recording's files, the pictures too", async () => {
    const e = { BUCKET: fakeBucket({ [`rec/${ID}/v0/seg_00000.m4s`]: bytes, [`rec/${ID}/master.m3u8`]: "#EXTM3U\n", [`rec/${ID}/poster.jpg`]: bytes, [`rec/${ID}/teaser.mp4`]: bytes, [`rec/${ID}/thumb-0123456789abcdef.gif`]: bytes }) };
    for (const f of ["v0/seg_00000.m4s", "master.m3u8", "poster.jpg", "teaser.mp4", "thumb-0123456789abcdef.gif"]) {
      const res = await get(e, `/m/${ID}/${f}`);
      expect(res.status, f).toBe(200);
      const cache = res.headers.get("cache-control") ?? "";
      expect(cache, f).toMatch(/^private\b/);
      expect(cache, f).not.toMatch(/\bpublic\b/);
      expect(Number(cache.match(/max-age=(\d+)/)?.[1]), f).toBeLessThanOrEqual(3600);
    }
  });

  it("serves only the kinds of file a recording is made of (B125)", async () => {
    const e = { BUCKET: fakeBucket({ [`rec/${ID}/page.html`]: "<script>alert(1)</script>", [`rec/${ID}/x`]: "x", [`rec/${ID}/a.svg`]: "<svg/>" }) };
    for (const f of ["page.html", "x", "a.svg"]) expect((await get(e, `/m/${ID}/${f}`)).status, f).toBe(404);
  });

  it("answers Range with 206 and the right Content-Range", async () => {
    const res = await get(env(), `/m/${ID}/v0/seg_00000.m4s`, { headers: { range: "bytes=10-19" } });
    expect(res.status).toBe(206);
    expect(res.headers.get("content-range")).toBe("bytes 10-19/1000");
    expect(new Uint8Array(await res.arrayBuffer())[0]).toBe(10);
    const tail = await get(env(), `/m/${ID}/v0/seg_00000.m4s`, { headers: { range: "bytes=-100" } });
    expect(tail.headers.get("content-range")).toBe("bytes 900-999/1000");
  });

  it("answers HEAD without a body", async () => {
    const res = await get(env(), `/m/${ID}/v0/seg_00000.m4s`, { method: "HEAD" });
    expect(res.status).toBe(200);
    expect(res.headers.get("content-length")).toBe("1000");
  });

  it("never serves outside rec/<id>/ — no meta, no traversal", async () => {
    // (URL parsing already resolves "." and ".." segments; what's left must stay inside.)
    const e = { BUCKET: fakeBucket({ [`meta/${ID}.json`]: metaOf({}), [`rec/${ID}/v0/stream.m3u8`]: "x" }) };
    for (const path of [`/m/${ID}/../../meta/${ID}.json`, `/m/${ID}/%2e%2e/%2e%2e/meta/${ID}.json`, `/m/${ID}//x`, `/m/short/v0/stream.m3u8`]) {
      expect((await get(e, path)).status, path).toBe(404);
    }
  });

  it("serves the bucket's hls.js", async () => {
    const res = await get(env(), "/_shroom/hls.js");
    expect(res.headers.get("content-type")).toBe("text/javascript");
    expect(await res.text()).toBe("/*hls*/");
  });

  it("names a download after the live title", async () => {
    const e = { BUCKET: fakeBucket({ [`meta/${ID}.json`]: metaOf({ title: "Q3 plan: v2/final", media: { hls: "x", mp4: `rec/${ID}/video.mp4`, width: 1, height: 1 } }), [`rec/${ID}/video.mp4`]: "mp4" }) };
    const res = await get(e, `/d/${ID}`);
    expect(res.headers.get("content-disposition")).toContain(`filename*=UTF-8''Q3%20plan%20%20v2%20final.mp4`);
  });

  it("serves the download only from the recording's own fixed key, whatever the record names", async () => {
    const e = { BUCKET: fakeBucket({
      [`meta/${ID}.json`]: metaOf({ media: { hls: "x", mp4: ".shroom/cookie.key", width: 1, height: 1 } }),
      ".shroom/cookie.key": "SECRET-SIGNING-KEY",
    }) };
    const res = await get(e, `/d/${ID}`);
    expect(res.status).toBe(404);
    expect(await res.text()).not.toContain("SECRET");
  });

  it("reads nothing outside the recording, the space's player, mark, cookie key and grants", async () => {
    const reads = [];
    const bucket = fakeBucket({ [`meta/${ID}.json`]: metaOf({ media: { hls: "../../.shroom/x", mp4: "meta/OTHER.json", poster: ".shroom/cookie.key", teaser: ".shroom/registry/roster.json", width: 1, height: 1 } }) });
    const wrap = { ...bucket, get: (k, o) => (reads.push(k), bucket.get(k, o)), head: (k) => (reads.push(k), bucket.head(k)) };
    for (const path of [`/${ID}`, `/${ID}?embed=1`, `/d/${ID}`, `/m/${ID}/master.m3u8`, `/_shroom/status/${ID}`, `/oembed?url=${encodeURIComponent(`https://v.example.workers.dev/${ID}`)}`]) {
      await get({ BUCKET: wrap }, path);
    }
    const allowed = [`meta/${ID}.json`, `.shroom/gone/${ID}`, ".shroom/mark.json", ".shroom/cookie.key", ".shroom/player/hls.min.js"];
    for (const k of reads) expect(allowed.includes(k) || k.startsWith(`rec/${ID}/`), k).toBe(true);
  });

  it("answers the app's reachability probe as a shroom space", async () => {
    const res = await get(env(), "/_shroom/ping");
    expect(await res.json()).toEqual({ shroom: true, iface: 2, version: expect.stringMatching(/^\d+\.\d+\.\d+/) }); // WORKER-INTERFACE §4.2
  });

  it("refuses writes", async () => {
    expect((await get(env(), `/${ID}`, { method: "POST" })).status).toBe(405);
  });
});

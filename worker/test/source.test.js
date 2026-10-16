// What the player plays (DECISIONS B176). Every request a viewer makes is one of the Workers
// Free plan's 100,000 a day; a 5-minute video watched through HLS's 6 s pieces costs ~56 of
// them, through the single-file copy ~4–8 (worker/dev/view-cost.mjs). So once the copy is up
// and the stream is one picture throughout, the page plays the copy; before that, or when a
// later take's picture differs in size (its own init segment, B58 a — a copy joined by
// `-c copy` can't carry the change), it plays the stream as before. And a page opened but
// not played asks for one piece of the stream, not two minutes of it.
import { describe, expect, it } from "vitest";
import { PLAYER_JS, handle } from "../src/worker.js";
import { M1, NOW, ORIGIN, OWNER_AIR, REC, cookieFor, metaKey, metaOf, space } from "./space-fixture.js";

const get = (env, path, init) => handle(new Request(`${ORIGIN}${path}`, init), env, NOW);
const html = async (env, path, init) => (await get(env, path, init)).text();
const media = (mp4) => ({ hls: `rec/${REC}/master.m3u8`, mp4: mp4 ? `rec/${REC}/video.mp4` : null, poster: `rec/${REC}/poster.jpg`, width: 1920, height: 1080 });
const ONE_MAP = "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:6.0,\nseg_00000.m4s\n#EXT-X-DISCONTINUITY\n#EXTINF:4.0,\nseg_00001.m4s\n#EXT-X-ENDLIST\n";
const TWO_MAPS = "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:6.0,\nseg_00000.m4s\n#EXT-X-DISCONTINUITY\n#EXT-X-MAP:URI=\"init_1.mp4\"\n#EXTINF:4.0,\nseg_00001.m4s\n#EXT-X-ENDLIST\n";
const withRecording = (mp4, playlist = ONE_MAP) => space({ objects: { [metaKey(REC)]: metaOf(REC, { durationSec: 10, media: media(mp4) }), ...(playlist === null ? {} : { [`rec/${REC}/v0/stream.m3u8`]: playlist }) } });
const owner = { headers: { cookie: cookieFor(M1, OWNER_AIR) } };
const COPY_SRC = `"/m/${REC}/video.mp4"`;
const STREAM_SRC = `"/m/${REC}/master.m3u8"`;

describe("the single-file copy plays once it's up (B176)", () => {
  it("is the recipient's page's source when the copy is up and the stream is one picture", async () => {
    const body = await html(withRecording(true), `/${REC}`);
    expect(body).toContain(`shroomPlayer(document.querySelector(".player"),${COPY_SRC}`);
    expect(body).not.toContain(STREAM_SRC);
  });

  it("is the embed's source too", async () => {
    const body = await html(withRecording(true), `/${REC}?embed=1`);
    expect(body).toContain(`shroomPlayer(document.querySelector(".player"),${COPY_SRC}`);
  });

  it("is the owner page's source when the page opens on a video whose copy is up", async () => {
    const body = await html(withRecording(true), `/${REC}`, owner);
    expect(body).toContain(`class="title"`);
    expect(body).toContain(`src=${COPY_SRC}`);
  });
});

describe("the stream plays otherwise (B176)", () => {
  it("before the copy is up", async () => {
    for (const [path, init] of [[`/${REC}`], [`/${REC}?embed=1`]]) expect(await html(withRecording(false), path, init)).toContain(STREAM_SRC);
    expect(await html(withRecording(false), `/${REC}`, owner)).toContain(`src=${STREAM_SRC}`);
  });

  it("when a later take's picture has its own init segment (B58 a)", async () => {
    for (const [path, init] of [[`/${REC}`], [`/${REC}?embed=1`], [`/${REC}`, owner]]) {
      const body = await html(withRecording(true, TWO_MAPS), path, init);
      expect(body).toContain(STREAM_SRC);
      expect(body).not.toContain(COPY_SRC);
    }
  });

  it("when the stream's playlist can't be read: nothing vouches the copy is one picture", async () => {
    expect(await html(withRecording(true, null), `/${REC}`)).toContain(STREAM_SRC);
    const env = withRecording(true);
    const read = env.BUCKET.get.bind(env.BUCKET);
    env.BUCKET.get = (key, opts) => key.endsWith("stream.m3u8") ? Promise.reject(new Error("get: We encountered an internal error. (10001)")) : read(key, opts);
    const res = await get(env, `/${REC}`);
    expect(res.status).toBe(200);
    expect(await res.text()).toContain(STREAM_SRC);
  });
});

describe("the player plays the copy as a file (B176)", () => {
  it("hands an .mp4 straight to the video element, never to hls.js", () => {
    const at = PLAYER_JS.indexOf("function shroomPlay(");
    const play = PLAYER_JS.slice(at, PLAYER_JS.indexOf("function shroomClock("));
    expect(play).toMatch(/\.mp4/);
    // The file branch comes before hls.js is ever loaded.
    expect(play.search(/\.mp4/)).toBeLessThan(play.indexOf("/_shroom/hls.js"));
  });
});

describe("a page opened but not played asks for one piece of the stream (B176)", () => {
  it("hls.js starts with a buffer of one piece and takes its usual one at the first play", () => {
    const at = PLAYER_JS.indexOf("new Hls(");
    const config = PLAYER_JS.slice(at, PLAYER_JS.indexOf(")", at));
    expect(config).toMatch(/[{,]maxBufferLength:6\b/);
    expect(config).toMatch(/[{,]maxMaxBufferLength:6\b/);
    expect(PLAYER_JS).toMatch(/"play",function\(\)\{[^}]*\.maxBufferLength=30[^}]*maxMaxBufferLength=600/);
  });
});

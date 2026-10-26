// What a recipient's browser shows, frame by frame: the heartbeat's measured plays (step 4)
// as a page init script. Every frame the player presents (requestVideoFrameCallback) is
// drawn onto a canvas and its counter strip read; the page keeps the counters in
// window.__samples, its media errors in __errors, the unreadable ones described in
// __unread, and __ended once the video has played to its end.
//
// Used by tests/e2e/heartbeat.mjs, and by dev/probe/webkit-frames.mjs, which plays the
// same script against the real page on a loaded hosted Mac.

import { readStrip, crc8 } from "./strip.mjs";

/**
 * @param {{ forceHlsJs?: boolean, rearm?: boolean }} opts  forceHlsJs: the page's hls.js path even
 *   where HLS is native. rearm (default on): the lost wake-up's re-arm below — off only for
 *   the probe, to show what it fixes.
 */
export function frameSampler({ forceHlsJs = false, rearm = true } = {}) {
  return `${crc8.toString()}\n${readStrip.toString()}
      window.__samples = []; window.__errors = []; window.__unread = []; window.__rearmed = 0;
      ${forceHlsJs ? `const orig = HTMLMediaElement.prototype.canPlayType; HTMLMediaElement.prototype.canPlayType = function (t) { return /mpegurl/i.test(t) ? "" : orig.call(this, t); };` : ""}
      document.addEventListener("DOMContentLoaded", () => {
        const v = document.getElementById("v"); if (!v) return;
        const c = document.createElement("canvas"); const ctx = c.getContext("2d", { willReadFrequently: true });
        v.muted = true; v.addEventListener("error", () => __errors.push("media " + (v.error && v.error.code)));
        v.addEventListener("ended", () => { window.__ended = true; });
        let asked = 0, lastCall = performance.now(), tAtCall = 0;
        const onFrame = (_, meta) => { const h = 270, w = Math.round(v.videoWidth / v.videoHeight * h) || 480;
          lastCall = performance.now(); tAtCall = v.currentTime;
          c.width = w; c.height = h; ctx.drawImage(v, 0, 0, w, h);
          const px = ctx.getImageData(0, 0, w, h).data, n = readStrip(px, w, h);
          // An unreadable frame, described: was anything drawn at all (the canvas starts each
          // frame transparent), how bright, and how ready the element said it was.
          let drawn = true;
          if (n === null) { let a = 0, l = 0; for (let i = 0; i < px.length; i += 4) { a += px[i + 3]; l += px[i] + px[i + 1] + px[i + 2]; }
            drawn = a > 0;
            if (__unread.length < 5) __unread.push({ at: __samples.length, drawn, luma: Math.round(l / (px.length / 4) / 3), readyState: v.readyState, presentedFrames: meta.presentedFrames, mediaTime: meta.mediaTime }); }
          // WebKit's native player can call back for the first frame before it can be drawn:
          // nothing lands on the canvas, and the next callback brings that same frame, readable
          // (DECISIONS B128). That one call — the first, nothing drawn — is no frame seen; every
          // other one counts.
          if (n === null && !drawn && __samples.length === 0 && !window.__undrawnFirst) window.__undrawnFirst = true;
          else __samples.push(n);
          window.__t = meta.mediaTime; asked = v.requestVideoFrameCallback(onFrame); };
        asked = v.requestVideoFrameCallback(onFrame);
        // WebKit's lost wake-up (DECISIONS B268): its native player wakes a pending callback
        // once, on the next new picture, and drops that wake while the element is still short
        // of HAVE_CURRENT_DATA — a race a loaded Mac loses, after which the callback never
        // comes though the video plays to its end. A callback asked for and not answered for
        // half a second while the picture moved is asked for again: the frames counted are
        // still only the ones the player says it presented, each drawn and read.
        ${rearm ? `setInterval(() => {
          if (v.paused || v.ended || v.readyState < 2 || performance.now() - lastCall < 500 || v.currentTime - tAtCall < 0.2) return;
          v.cancelVideoFrameCallback(asked); asked = v.requestVideoFrameCallback(onFrame);
          lastCall = performance.now(); window.__rearmed++;
        }, 100);` : ""}
        const go = () => { v.playbackRate = 2; v.play().catch(e => __errors.push("play " + e.message)); };
        if (v.readyState > 0) go(); else v.addEventListener("loadedmetadata", go, { once: true });
      });`;
}

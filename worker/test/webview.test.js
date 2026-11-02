// The player inside an app's in-app browser (iOS WKWebView / SFSafariViewController sheets,
// Android WebView): the player's own fullscreen can be missing, refused or throw there. The
// button then falls back to the phone's own fullscreen on the video, never throws, and hides
// when there is no fullscreen at all. Against a stand-in browser, as edits.test.js's stage.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { PLAYER_JS } from "../src/worker.js";

/** @param {{ el?: any, video?: any, enabled?: boolean }} o */
function stage({ el = {}, video = {}, enabled = true } = {}) {
  const node = () => ({ attrs: {}, on: {}, hidden: false, textContent: "", value: "0", max: "0", style: { setProperty() {} }, classList: { toggle() {}, add() {}, remove() {}, contains: () => false },
    setAttribute(k, v) { this.attrs[k] = String(v); }, getAttribute(k) { return this.attrs[k] ?? null; }, addEventListener(e, f) { (this.on[e] ??= []).push(f); } });
  const v = { ...node(), currentTime: 0, paused: true, ended: false, duration: 10, buffered: { length: 0 }, playbackRate: 1, muted: false, src: "", canPlayType: () => "maybe",
    play() { this.paused = false; return Promise.resolve(); }, pause() { this.paused = true; }, ...video };
  const parts = { video: v, ".pp": node(), ".seek": node(), ".time": node(), ".fs": node(), ".sp": node(), ".mu": node() };
  const p = { ...node(), querySelector: (q) => parts[q], contains: () => false, ...el };
  const document = { addEventListener() {}, createElement: () => ({}), head: { appendChild() {} }, activeElement: null, fullscreenElement: null, fullscreenEnabled: enabled };
  const shroomPlayer = new Function("document", "requestAnimationFrame", "fetch", "setTimeout", "clearTimeout", `${PLAYER_JS};return shroomPlayer`)(
    document, () => 1, () => Promise.reject(new Error("no")), () => 0, () => {});
  shroomPlayer(p, "/m/x/master.m3u8", { play: "Play", pause: "Pause", full: "", exit: "", speed: "", mute: "", unmute: "" }, {});
  const fs = parts[".fs"];
  return { fs, press: () => fs.on.click.forEach((f) => f({ target: { tagName: "BUTTON" } })) };
}

describe("fullscreen in an in-app browser", () => {
  it("falls back to the phone's own fullscreen when the player's own throws (WebKit: “The object is in an invalid state.”)", () => {
    let own = 0;
    const s = stage({ el: { requestFullscreen() { throw new Error("The object is in an invalid state."); } }, video: { webkitEnterFullscreen() { own++; } } });
    expect(s.fs.hidden).toBe(false);
    expect(() => s.press()).not.toThrow();
    expect(own).toBe(1);
  });

  it("falls back to the phone's own fullscreen when the player's own is refused (a web view whose app has no fullscreen)", async () => {
    let own = 0;
    const s = stage({ el: { requestFullscreen: () => Promise.reject(new TypeError("refused")) }, video: { webkitEnterFullscreen() { own++; } } });
    s.press();
    await new Promise((r) => setTimeout(r, 0));
    expect(own).toBe(1);
  });

  it("uses the phone's own fullscreen where elements can't go fullscreen (an iPhone), and a prefixed one that throws doesn't break the page", () => {
    let own = 0;
    const s = stage({ video: { webkitEnterFullscreen() { own++; throw new Error("not allowed"); } } });
    expect(s.fs.hidden).toBe(false);
    expect(() => s.press()).not.toThrow();
    expect(own).toBe(1);
    let pre = 0;
    const t = stage({ el: { webkitRequestFullscreen() { pre++; } } });
    t.press();
    expect(pre).toBe(1);
  });

  it("hides the button where there is no fullscreen at all, and keeps it where there is", () => {
    expect(stage().fs.hidden).toBe(true);
    expect(stage({ el: { requestFullscreen: () => Promise.resolve() }, enabled: false }).fs.hidden).toBe(true);
    expect(stage({ el: { requestFullscreen: () => Promise.resolve() } }).fs.hidden).toBe(false);
    expect(stage({ el: { requestFullscreen: () => Promise.resolve() }, video: { webkitEnterFullscreen() {} }, enabled: false }).fs.hidden).toBe(false);
  });

  it("the hidden button is out of the row", () => {
    expect(readFileSync(new URL("../src/worker.js", import.meta.url), "utf8")).toContain(".row button[hidden]{display:none}");
  });
});

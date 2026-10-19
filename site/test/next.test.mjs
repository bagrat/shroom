// shroom.am/next (site/next.html): the landing as it will be with the voice-cut hero, on the
// canvas board approved 2026-10-18. Hidden until it becomes /; the same words, terms and
// install as the landing; the setup walkthrough behind a button, in a lightbox.
// Run: node --test site/test/next.test.mjs
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import assert from "node:assert/strict";

const ROOT = new URL("../..", import.meta.url).pathname;
const next = readFileSync(`${ROOT}site/next.html`, "utf8");
const landing = readFileSync(`${ROOT}site/index.html`, "utf8");
const visible = (/** @type {string} */ s) => s
  .replace(/<!--[\s\S]*?-->/g, " ")
  .replace(/<(script|style|dialog)[\s\S]*?<\/\1>/g, " ")
  .replace(/<section class="how-to"[\s\S]*?<\/section>/g, " ")
  .replace(/<button type="button" class="walk"[\s\S]*?<\/button>/g, " ")
  .replace(/<[^>]+>/g, "")
  .replace(/\s+/g, " ");

describe("the landing with the hero (/next)", () => {
  it("asks search engines to leave it out", () => {
    assert.match(next, /<meta name="robots" content="noindex, nofollow">/);
  });

  it("says everything the landing says, outside the box's video", () => {
    assert.equal(visible(next), visible(landing));
  });

  it("puts the hero, then Setup walkthrough, then a cut, then Install in the box", () => {
    const box = next.slice(next.indexOf('<div class="box">'), next.indexOf('<section class="install"'));
    const at = (/** @type {string} */ s) => box.indexOf(s);
    assert.ok(at('id="hero"') > 0 && at('class="walk"') > at('id="hero"') && at('class="cut"') > at('class="walk"'));
    assert.match(box, /<span>Setup walkthrough<\/span>/);
    assert.match(box, />2 min</);
  });

  it("opens the walkthrough, the site's own /how-to.mp4, only when asked", () => {
    assert.match(next, /<dialog class="lightbox" id="walk"/);
    assert.match(next, /<video controls playsinline preload="none"[^>]*data-src="\/how-to\.mp4"><\/video>/);
    assert.doesNotMatch(next, /<video[^>]* src=/);
  });

  it("centres the play triangles: no push to the right", () => {
    assert.doesNotMatch(next, /\.play svg \{[^}]*margin-left/);
  });

  it("asks nothing of anyone else's servers", () => {
    const urls = [...next.matchAll(/(?:src|href)="(https?:[^"]+)"/g)].map((m) => new URL(m[1]).hostname);
    for (const host of urls) assert.ok(host === "shroom.am" || host === "github.com", host);
  });
});

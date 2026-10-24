// The landing (Region 1, `landing`): every ratified line is on the page, verbatim, and
// the page asks nothing of anyone else's servers (Cloudflare's own Web Analytics beacon, added
// at its edge as it serves the page, is the one exception: B224). Run: node --test site/test/landing.test.mjs
//
// One of the fixture's lines is ratified as a shape, not as words: the system-requirements
// sentence (“fill the facts in, don't reword”). It is checked against its facts instead —
// the floor and the kinds of Mac from what actually ships. The hero plays in place and the
// setup walkthrough in a lightbox (N58, N61, N70): both are the site's own files, and the page
// fetches them only from script.
import { existsSync, readFileSync, statSync } from "node:fs";
import { describe, it } from "node:test";
import assert from "node:assert/strict";

const ROOT = new URL("../..", import.meta.url).pathname;
const html = readFileSync(`${ROOT}site/index.html`, "utf8");
const visible = (s) => s
  .replace(/<!--[\s\S]*?-->/g, " ")
  .replace(/<(script|style)[\s\S]*?<\/\1>/g, " ")
  .replace(/<[^>]+>/g, "")
  .replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">")
  .replace(/\s+/g, " ");
const text = visible(html);
const landing = JSON.parse(readFileSync(`${ROOT}spec/machine.json`, "utf8")).states.landing;

const SYSREQ = /^For macOS (\d+) or newer, (.+)\.$/;
const isSysReq = (l) => SYSREQ.test(l.text);

// The facts, from the two files that decide them: the installer's floor, and the kinds of
// Mac the release packager lets through (its `case "$arch"` arms that don't exit).
const installer = readFileSync(`${ROOT}installer/install.sh`, "utf8");
const packager = readFileSync(`${ROOT}tools/package-release.sh`, "utf8");
const floor = installer.match(/^MIN_MACOS=(\d+)$/m)?.[1];
const archCase = packager.match(/case "\$arch" in([\s\S]*?)esac/)?.[1] ?? "";
const shipped = [...archCase.matchAll(/^\s*([a-z0-9_|]+)\)\s*;;/gm)].flatMap((m) => m[1].split("|"));
const KIND = { arm64: "Apple silicon", x86_64: "Intel" };
const kinds = ["arm64", "x86_64"].filter((a) => shipped.includes(a)).map((a) => KIND[a]).join(" and ");

const video = html.match(/<section class="how-to"[^>]*>[\s\S]*?<\/section>/)?.[0] ?? "";

describe("the landing", () => {
  it("says every ratified line verbatim", () => {
    for (const l of landing.said) {
      // The copy table's lines (the title, the search description, the footer, the agent
      // door) are the build's, each held by its own test.
      if (isSysReq(l) || (l.cites ?? []).includes("copy table")) continue;
      assert.ok(text.includes(l.text), `missing: ${l.text}`);
    }
  });

  it("states the system requirements in the ratified shape, with the facts that ship", () => {
    const ratified = landing.said.filter(isSysReq);
    assert.equal(ratified.length, 1, "the fixture has one system-requirements line");
    assert.ok(floor, "installer/install.sh names its MIN_MACOS");
    assert.ok(kinds, "tools/package-release.sh lets at least one kind of Mac through");
    const line = `For macOS ${floor} or newer, ${kinds}.`;
    assert.match(line, SYSREQ, "the filled line keeps the ratified shape");
    assert.ok(text.includes(line), `the landing should say: ${line}`);
    const said = [...text.matchAll(/For macOS \d+ or newer, [^.]*\./g)].map((m) => m[0]);
    assert.deepEqual(said, [line], "no other version of the sentence is on the page");
  });

  it("plays the hero in place, right above Install (N58, N59, N61, N70)", () => {
    assert.doesNotMatch(video, /<section class="how-to"[^>]*\bhidden\b/, "the video's block shows");
    assert.equal(video.match(/data-src="([^"]*)"/)?.[1], "/hero-2.mp4", "the video is shroom.am's own file");
    assert.doesNotMatch(video, /href=/, "the video links nowhere: it plays on the landing");
    const words = visible(video.replace(/<span[^>]*aria-hidden="true"[^>]*>[\s\S]*?<\/span>/g, "")).trim();
    assert.equal(words, "", "no caption (Bagrat, 2026-10-17): the still and its play button speak for it; the length is decoration");
    assert.ok(!landing.said.some((l) => l.id === "landing.recorded-with-shroom-itself"), "the caption's said-line is gone from the spec");
    assert.match(video, /<button type="button" class="still" aria-label="Play shroom, recorded with shroom, 2 minutes">/, "its name to VoiceOver (N70)");
    assert.match(video, /<span class="len" aria-hidden="true">2 min<\/span>/, "its length (N70; Bagrat, 2026-10-18)");
    assert.doesNotMatch(html, /\bdemo\b/i, "the page doesn't call it a demo (N61)");
    assert.match(video, /<video\b[^>]*\bpreload="none"/, "nothing loads before the script asks");
    assert.doesNotMatch(video, /<video\b[^>]*\bsrc=/, "the video's address is set by script, as the block nears the screen");
    assert.match(video, /<img src="hero-poster\.webp" alt="" width="1280" height="832"/, "the still is the site's own file");
    assert.doesNotMatch(html, /\.play svg \{[^}]*margin-left/, "the play triangles sit at their optical centre");
    const order = ['class="terms"', 'class="how-to"', 'id="install"'].map((k) => html.indexOf(k));
    assert.ok(order[0] < order[1] && order[1] < order[2], "the video's box comes after the terms (beside them on a laptop), and Install under the video (N59)");
  });

  it("opens the setup walkthrough from a button under the hero, in a lightbox, only when asked (N70)", () => {
    const box = html.slice(html.indexOf('<div class="box">'), html.indexOf('<section class="install"'));
    const at = (/** @type {string} */ k) => box.indexOf(k);
    assert.ok(at('id="hero"') > 0 && at('class="walk"') > at('id="hero"') && at('class="cut"') > at('class="walk"'), "the hero, then Setup walkthrough, then a cut, then Install");
    assert.match(box, /<span>Setup walkthrough<\/span>/);
    assert.match(html, /<dialog class="lightbox" id="walk" aria-label="How to set up and use shroom, 4 minutes">/);
    assert.match(html, /<video controls playsinline preload="none"[^>]*data-src="\/how-to\.mp4"><\/video>/);
  });

  it("ships the videos and their stills with the page", () => {
    const build = readFileSync(`${ROOT}site/build.mjs`, "utf8");
    for (const f of ["hero-poster.webp", "how-to-poster.webp"]) {
      assert.ok(existsSync(`${ROOT}site/${f}`), `site/${f} is there`);
      assert.match(build, new RegExp(`"${f.replace(".", "\\.")}"`), `build.mjs copies ${f} into what the site serves`);
    }
    assert.ok(statSync(`${ROOT}site/hero-poster.webp`).size < 100_000, "the hero's still stays light");
    for (const f of ["hero-2.mp4", "how-to.mp4"]) {
      // Every browser plays H.264 only at Baseline, Main or High (4:2:0); High 4:4:4 shows the
      // still and nothing more (B222). The profile is the byte after avcC's version.
      const mp4 = readFileSync(`${ROOT}site/${f}`), at = mp4.indexOf("avcC");
      assert.ok(at > 0, `${f} is H.264`);
      assert.ok([66, 77, 100].includes(mp4[at + 5]), `${f} is Baseline, Main or High (4:2:0), not profile ${mp4[at + 5]}`);
      // Cloudflare serves no single file over 25 MiB.
      assert.ok(statSync(`${ROOT}site/${f}`).size < 25 * 1024 * 1024, `site/${f} is under the asset limit`);
      assert.match(build, new RegExp(`"${f.replace(".", "\\.")}"`), `build.mjs copies ${f} into what the site serves`);
    }
  });

  it("loads nothing from anyone else", () => {
    // Links people follow are fine; nothing the page fetches may be remote, the video included.
    // (This reads the file in the repo; Cloudflare's edge adds its Web Analytics beacon: B224.)
    assert.doesNotMatch(html, /<(?:script|link|img|video|source|iframe)[^>]*\b(?:src|href)="https?:/, "a remote fetch in the markup");
    const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1]).join("\n");
    assert.deepEqual([...scripts.matchAll(/https?:\/\/[^"'\s]*/g)].map((m) => m[0]), [], "no remote address is built in script");
    assert.match(scripts, /var src = box\.getAttribute\("data-src"\);/);
  });

  it("offers the install line first, then Homebrew, then the agent, in tabs (N59)", () => {
    const tabs = [...html.matchAll(/<button type="button" role="tab" id="([^"]+)" aria-controls="([^"]+)" aria-selected="(true|false)"[^>]*>([^<]+)<\/button>/g)];
    assert.deepEqual(tabs.map((t) => t[4]), ["Terminal", "Homebrew", "Coding agent"]);
    assert.deepEqual(tabs.map((t) => t[3]), ["true", "false", "false"], "Terminal is chosen when the page opens");
    const panel = (id) => html.match(new RegExp(`<div class="panel( on)?" id="${id}" role="tabpanel"[^>]*>([\\s\\S]*?)\\n        </div>`)) ?? [];
    const [term, brew, agent] = tabs.map((t) => panel(t[2]));
    assert.equal(term[1], " on", "its panel is the one showing");
    assert.ok(!brew[1] && !agent[1], "the others wait");
    assert.match(term[2], /<code id="cmd">curl -fsSL https:\/\/shroom\.am\/install \| sh<\/code>/);
    assert.match(brew[2], /<code id="brew">brew install --cask bagrat\/shroom\/shroom &amp;&amp; open ~\/Applications\/shroom\.app<\/code>/);
    assert.match(agent[2], /<code id="agent">Install shroom on my Mac: read https:\/\/shroom\.am\/agent and follow it\.<\/code>/);
    // Each tab's own Read it first, on the line with the system requirements (Homebrew has none).
    const reads = [...html.matchAll(/<span class="read( on)?" data-for="([^"]+)"><a href="([^"]+)">Read it first<\/a>/g)].map((m) => [m[2], m[3], m[1] ?? ""]);
    assert.deepEqual(reads, [[tabs[0][2], "/install", " on"], [tabs[2][2], "/agent", ""]]);
    // Without script every panel shows, Terminal's first; the sheet hides the others only once script runs.
    assert.match(html, /\.js \.panel:not\(\.on\), \.js \.read:not\(\.on\) \{ display: none; \}/);
    assert.match(html, /<script>document\.documentElement\.className = "js";<\/script>/);
  });

  it("puts the terms before the video on a phone, and install last (Bagrat, 2026-10-17)", () => {
    assert.match(html, /grid-template-areas: "brand box" "pitch box" "terms box";/, "a laptop: the box beside the pitch and the terms");
    assert.match(html, /@media \(max-width: 900px\) \{[\s\S]*?grid-template-areas: "brand" "pitch" "terms" "box";/, "a phone: the terms, then the video and install");
  });

  it("says the terms in Bagrat's order, under the price bar (N59)", () => {
    const terms = (html.match(/<ul class="terms">([\s\S]*?)<\/ul>/)?.[1] ?? "").match(/<li>[\s\S]*?<\/li>/g)?.map((li) => visible(li).trim()) ?? [];
    const ids = ["landing.your-videos-live-in", "landing.you-only-ever-pay", "landing.your-team-just-accepts", "landing.no-price-per-person",
      "landing.delete-old-videos-and", "landing.you-set-cloudflare-up", "landing.open-source-every-line"];
    assert.deepEqual(terms, ids.map((id) => landing.said.find((l) => l.id === id).text));
  });

  it("puts the price in its own box, as wide as the headline: the price first, one slider, how long videos are kept, the AI switch (N74)", () => {
    const pitch = html.match(/<div class="pitch">([\s\S]*?)\n  <\/div>/)?.[1] ?? "";
    assert.match(pitch, /<h1>Record\. Share\. Pay-as-you-go\.<\/h1>/);
    assert.doesNotMatch(html, /class="gauge"/, "the bar is gone (a tester read it as loading)");
    const box = pitch.match(/<section class="price"[\s\S]*?<\/section>/)?.[0] ?? "";
    assert.ok(box, "the price box sits in the pitch, under the headline's line");
    const at = (k) => box.indexOf(k);
    assert.ok(at('id="price-h"') < at('id="cost"') && at('id="cost"') < at('id="hours"') && at('id="hours"') < at('class="keep') && at('class="keep') < at('id="ai"'),
      "the heading, then the price, then the slider, then Keep videos, then the AI switch (Bagrat, 2026-10-24)");
    assert.equal([...box.matchAll(/type="range"/g)].length, 1, "one slider");
    assert.match(box, /<input class="live" type="range" id="hours" min="0" max="9" step="1" value="1"/, "hours of video a month, opening at 1 hour");
    assert.match(box, /<button type="button" aria-pressed="true" data-months="1">30 days<\/button><button type="button" aria-pressed="false" data-months="2">60 days<\/button><button type="button" aria-pressed="false" data-months="12">a year<\/button>/,
      "kept 30 days, 60 days or a year, 30 days when the page opens");
    assert.match(box, /<button type="button" role="switch" id="ai" aria-checked="false" aria-labelledby="ai-l"><\/button>/, "the AI switch, off when the page opens");
    assert.match(html, /\.pitch \{[^}]*width: max-content;/, "the headline, its line and the box share the headline's width");
    assert.match(box, new RegExp(`<p class="why">${landing.said.find((l) => l.id === "landing.a-team-of-20").text.replace(/[.$]/g, "\\$&")}</p>\\s*</section>`),
      "the team's month closes the box, nothing under it (Bagrat, 2026-10-24)");
  });

  it("works the price out the page's way, in dollars, and opens at $0.00 (N74)", () => {
    // About 10 GB per three hours at Normal (the FAQ), $0.015 a GB-month past Cloudflare's free
    // 10 GB (B212), 30 days as a month.
    const script = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1]).join("\n");
    assert.match(script, /var HOURS = \[0\.5, 1, 2, 3, 5, 10, 20, 30, 50, 100\], GB_PER_HOUR = 10 \/ 3, FREE_GB = 10, USD_PER_GB = 0\.015;/);
    assert.match(script, /cost\.textContent = "\$" \+ usd\.toFixed\(2\);/, "dollars, never cents (Bagrat, 2026-10-24)");
    const month = (hours, months) => Math.max(0, hours * months * 10 / 3 - 10) * 0.015;
    assert.match(html, /<b id="cost" class="free">\$0\.00<\/b>/, "an hour, kept 30 days: $0.00");
    assert.equal(month(1, 1), 0);
    assert.equal(month(1, 12).toFixed(2), "0.45");
    // The team's line: 20 hours a month.
    assert.equal(month(20, 1).toFixed(2), "0.85");
    assert.equal(month(20, 12).toFixed(2), "11.85");
    assert.doesNotMatch(html, /¢/, "no cents anywhere on the page");
    assert.match(html, /\.price \.live \{ display: none; \}/, "without script the controls stay hidden and the opening reading shows");
  });

  it("points Read it first at the installer's own address", () => {
    assert.match(html, /<a href="\/install">Read it first<\/a>/);
    assert.ok(text.includes("curl -fsSL https://shroom.am/install | sh"));
  });
});

describe("the page's foot", () => {
  const foot = html.match(/<footer class="foot">[\s\S]*?<\/footer>/)?.[0] ?? "";

  it("says visits are counted, and how little is kept (DECISIONS copy table)", () => {
    assert.ok(foot.includes("<p>We count visits to this page — no cookies, and nothing that tells us who you are.</p>"));
  });

  it("holds the links the header and footer held, Send feedback first (N55, N59)", () => {
    const hrefs = [...foot.matchAll(/<a href="([^"]+)">/g)].map((m) => m[1]);
    assert.deepEqual(hrefs, ["/feedback", "https://github.com/bagrat/shroom", "/faq", "/privacy", "/terms"]);
    assert.ok(visible(foot).includes(landing.said.find((l) => l.id === "landing.shroom-trouble-questions").text));
    assert.doesNotMatch(html, /Trouble, questions/, "Send feedback already opens /feedback");
  });

  it("sits at the bottom of the first screen, never over the page (N59)", () => {
    assert.match(html, /\.page \{ min-height: 100vh; min-height: 100svh; display: flex; flex-direction: column;/);
    assert.match(html, /\.foot \{[^}]*margin: auto auto 0;/);
    assert.doesNotMatch(html, /position: (fixed|sticky)/);
  });
});

describe("the header", () => {
  it("is gone from the landing: its links are at the foot (N59)", () => {
    assert.doesNotMatch(html, /<header>/);
  });
});

describe("a shared link's tag", () => {
  it("leaves the address bar, so it isn't passed on", () => {
    assert.match(html, /history\.replaceState\(null, "", u\.pathname \+ u\.search \+ u\.hash\)/);
    assert.match(html, /k === "ref" \|\| k\.indexOf\("utm_"\) === 0/);
  });
});

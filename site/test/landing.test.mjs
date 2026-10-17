// The landing (Region 1, `landing`): every ratified line is on the page, verbatim, and
// the page asks nothing of anyone else's servers. Run: node --test site/test/landing.test.mjs
//
// One of the fixture's lines is ratified as a shape, not as words: the system-requirements
// sentence (“fill the facts in, don't reword”). It is checked against its facts instead —
// the floor and the kinds of Mac from what actually ships. The demo plays in place (N58):
// its block names a real recording, and the page fetches the video only from script.
import { existsSync, readFileSync } from "node:fs";
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

const demo = html.match(/<section class="demo"[^>]*>[\s\S]*?<\/section>/)?.[0] ?? "";
const ID = /^[A-Za-z0-9_-]{16}$/; // DECISIONS B14: 12 random bytes, base64url

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

  it("plays a real recording in place, right above Install (N58, N59)", () => {
    assert.doesNotMatch(demo, /<section class="demo"[^>]*\bhidden\b/, "the demo block shows");
    const id = demo.match(/data-id="([^"]*)"/)?.[1] ?? "";
    assert.match(id, ID, "the demo is a real recording id");
    assert.match(demo.match(/data-space="([^"]*)"/)?.[1] ?? "", /^v\.[a-z0-9-]+\.workers\.dev$/, "the space it plays from");
    assert.doesNotMatch(demo, /href=/, "the demo links nowhere: it plays on the landing, never on the player page");
    const caption = landing.said.find((l) => l.id === "landing.recorded-with-shroom-itself").text;
    const words = visible(demo.replace(/<span[^>]*aria-hidden="true"[^>]*>[\s\S]*?<\/span>/g, "")).trim();
    assert.equal(words, caption, "the caption is the block's only words (the length on the still is decoration)");
    assert.match(demo, /<video\b[^>]*\bpreload="none"/, "nothing loads before the script asks");
    assert.doesNotMatch(demo, /<video\b[^>]*\bsrc=/, "the video's address is set by script, as the block nears the screen");
    assert.match(demo, /<img src="demo-poster\.webp"/, "the still is the site's own file");
    const order = ['class="terms"', 'class="demo"', 'id="install"'].map((k) => html.indexOf(k));
    assert.ok(order[0] < order[1] && order[1] < order[2], "the demo's box comes after the terms (beside them on a laptop), and Install under the demo (N59)");
  });

  it("ships the demo's still with the page", () => {
    const build = readFileSync(`${ROOT}site/build.mjs`, "utf8");
    assert.ok(existsSync(`${ROOT}site/demo-poster.webp`), "site/demo-poster.webp is there");
    assert.match(build, /"demo-poster\.webp"/, "build.mjs copies it into what the site serves");
  });

  it("loads nothing from anyone else, but the demo's video from the maker's own space", () => {
    // Links people follow are fine; nothing the markup itself fetches may be remote.
    assert.doesNotMatch(html, /<(?:script|link|img|video|source|iframe)[^>]*\b(?:src|href)="https?:/, "a remote fetch in the markup");
    // The script's one remote address is the demo's video, in the space the block names (N58).
    const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1]).join("\n");
    assert.deepEqual([...scripts.matchAll(/https?:\/\/[^"'\s]*/g)].map((m) => m[0]), ["https://"], "only the demo's address is built in script");
    assert.match(scripts, /"https:\/\/" \+ box\.getAttribute\("data-space"\) \+ "\/m\/" \+ box\.getAttribute\("data-id"\) \+ "\/video\.mp4"/);
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

  it("says the terms in Bagrat's order, under the price bar (N59)", () => {
    const terms = (html.match(/<ul class="terms">([\s\S]*?)<\/ul>/)?.[1] ?? "").match(/<li>[\s\S]*?<\/li>/g)?.map((li) => visible(li).trim()) ?? [];
    const ids = ["landing.your-videos-live-in", "landing.you-only-ever-pay", "landing.your-team-just-accepts", "landing.no-price-per-person",
      "landing.delete-old-videos-and", "landing.you-set-cloudflare-up", "landing.open-source-every-line"];
    assert.deepEqual(terms, ids.map((id) => landing.said.find((l) => l.id === id).text));
  });

  it("draws the price as one bar, its two ends labelled, inside the headline's own width (N59)", () => {
    const pitch = html.match(/<div class="pitch">([\s\S]*?)\n  <\/div>/)?.[1] ?? "";
    assert.match(pitch, /<h1>Record\. Share\. Pay-as-you-go\.<\/h1>/);
    const ends = [...pitch.matchAll(/<span>(.*?)<\/span>/g)].map((m) => visible(m[1]));
    assert.deepEqual(ends, ["landing.0-month-about-six", "landing.1-50-month-about"].map((id) => landing.said.find((l) => l.id === id).text));
    assert.match(pitch, /<div class="gauge" role="img" aria-label="[^"]+">/, "the bar has a name a screen reader says");
    assert.match(html, /\.pitch \{[^}]*width: max-content;/, "the headline, its line and the bar share the headline's width");
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

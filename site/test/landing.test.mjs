// The landing (Region 1, `landing`): every ratified line is on the page, verbatim, and
// the page asks nothing of anyone else's servers. Run: node --test site/test/landing.test.mjs
//
// Two of the fixture's lines are ratified as a shape, not as words: the system-requirements
// sentence (“fill the facts in, don't reword”) and the demo's address (“id illustrative”).
// Those two are checked against their facts instead — the floor and the kinds of Mac from
// what actually ships, the demo's id from the recording it links (DEVIATIONS #29).
import { readFileSync } from "node:fs";
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
const isDemoAddress = (l) => /illustrative/.test(l.note ?? "");

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
      if (isSysReq(l) || isDemoAddress(l) || (l.cites ?? []).includes("copy table")) continue;
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

  it("links only a real recording as its demo, or holds the block back", () => {
    const address = landing.said.find(isDemoAddress).text;
    const host = address.slice(0, address.lastIndexOf("/") + 1); // v.myshroom.workers.dev/
    const illustrative = address.slice(host.length);
    assert.ok(!html.includes(illustrative), `the spec's illustrative id (${illustrative}) is never linked`);
    const id = demo.match(/data-id="([^"]*)"/)?.[1];
    assert.notEqual(id, undefined, "the demo block carries its id in data-id");
    if (/<section class="demo"[^>]*\bhidden\b/.test(demo)) {
      assert.equal(id, "", "a held-back demo names no recording");
      assert.doesNotMatch(demo, /href=/, "a held-back demo links nowhere");
    } else {
      assert.match(id, ID, "the demo is a real recording id");
      assert.ok(demo.includes(`href="https://${host}${id}"`), "the link opens that recording's page");
      assert.ok(visible(demo).includes(`${host}${id}`), "the address shown is the one linked");
      const caption = landing.said.find((l) => /Recorded with shroom itself/.test(l.text)).text;
      assert.ok(visible(demo).includes(caption), "the caption sits with the link");
    }
  });

  it("loads nothing from anyone else", () => {
    const external = [...html.matchAll(/(?:src|href)="(https?:[^"]+)"/g)].map((m) => m[1]);
    for (const url of external) {
      // Links people follow are fine; nothing the page itself fetches may be remote.
      assert.ok(!/<(?:script|link|img)[^>]*\b(?:src|href)="https?:/.test(html), `fetches ${url}`);
    }
  });

  it("offers Homebrew first, the install line under it (N52)", () => {
    const brew = text.indexOf("brew install --cask bagrat/shroom/shroom && open ~/Applications/shroom.app");
    const curl = text.indexOf("curl -fsSL https://shroom.am/install | sh");
    assert.ok(brew >= 0 && curl > brew, "the Homebrew line comes before the install line");
  });

  it("points Read it first at the installer's own address", () => {
    assert.match(html, /<a href="\/install">Read it first<\/a>/);
    assert.ok(text.includes("curl -fsSL https://shroom.am/install | sh"));
  });
});

describe("the footer", () => {
  it("says visits are counted, and how little is kept (DECISIONS copy table)", () => {
    const footer = html.match(/<footer>[\s\S]*?<\/footer>/)?.[0] ?? "";
    assert.ok(footer.includes("We count visits to this page — no cookies, and nothing that tells us who you are."));
  });

  it("takes Trouble, questions to the feedback page (N55)", () => {
    const footer = html.match(/<footer>[\s\S]*?<\/footer>/)?.[0] ?? "";
    assert.match(footer, /shroom · <a href="\/feedback">Trouble, questions<\/a>/);
  });
});

describe("the header", () => {
  it("offers Send feedback beside GitHub (N55)", () => {
    const nav = html.match(/<header>[\s\S]*?<nav>([\s\S]*?)<\/nav>/)?.[1] ?? "";
    assert.equal(nav, '<a href="/feedback">Send feedback</a><a href="https://github.com/bagrat/shroom">GitHub</a>');
  });
});

describe("a shared link's tag", () => {
  it("leaves the address bar, so it isn't passed on", () => {
    assert.match(html, /history\.replaceState\(null, "", u\.pathname \+ u\.search \+ u\.hash\)/);
    assert.match(html, /k === "ref" \|\| k\.indexOf\("utm_"\) === 0/);
  });
});

// The weekly collector's pure parts (dev/stats/collect.mjs).
// Run: node --test dev/stats/collect.test.mjs
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { cameVia, funnel, webAnalytics, week } from "./collect.mjs";

describe("the week", () => {
  it("is the seven whole UTC days before the end day", () => {
    assert.deepEqual(week(new Date("2026-10-21T15:30:00Z")), { id: "2026-10-14", from: "2026-10-14", to: "2026-10-21" });
  });
  it("is yesterday for a daily run", () => {
    assert.deepEqual(week(new Date("2026-10-21T06:07:00Z"), 1), { id: "2026-10-20", from: "2026-10-20", to: "2026-10-21" });
  });
});

describe("the funnel", () => {
  it("names each count by what it means", () => {
    const f = funnel([
      { route: "/", answer: "html", client: "browser", n: 40 },
      { route: "/", answer: "markdown", client: "agent:claude", n: 3 },
      { route: "/", answer: "markdown", client: "other", n: 9 },
      { route: "/agent", answer: "markdown", client: "agent:openai", n: 5 },
      { route: "/agent.md", answer: "markdown", client: "agent:claude", n: 1 },
      { route: "/agent", answer: "html", client: "browser", n: 2 },
      { route: "/agent", answer: "html", client: "preview", n: 4 },
      { route: "/install", answer: "text", client: "curl", n: 12 },
      { route: "/install", answer: "text", client: "wget", n: 1 },
      { route: "/install", answer: "text", client: "browser", n: 6 },
      { route: "/", answer: "html", client: "bot", n: 20 },
    ]);
    assert.deepEqual(f, {
      landingVisits: 40, landingAgents: 3, agentPageAgents: 6, agentPageBrowsers: 2,
      installRuns: 13, installReads: 6, previews: 4, bots: 20, other: 9,
    });
  });
});

describe("Cloudflare's count", () => {
  it("keeps the totals and the top lists, without shroom.am linking to itself", () => {
    const shaped = webAnalytics({
      t: [{ count: 39, sum: { visits: 35 } }],
      r: [{ sum: { visits: 34 }, dimensions: { refererHost: "" } }, { sum: { visits: 1 }, dimensions: { refererHost: "yandex.com" } }, { sum: { visits: 0 }, dimensions: { refererHost: "shroom.am" } }],
      c: [{ sum: { visits: 27 }, dimensions: { countryName: "AM" } }],
      d: [{ sum: { visits: 28 }, dimensions: { deviceType: "desktop" } }],
      p: [{ count: 33, dimensions: { requestPath: "/" } }],
    });
    assert.deepEqual(shaped, {
      pageviews: 39, visits: 35, referrers: [{ k: "yandex.com", n: 1 }],
      countries: [{ k: "AM", n: 27 }], devices: [{ k: "desktop", n: 28 }], paths: [{ k: "/", n: 33 }],
    });
  });
});

describe("what came via each link tag", () => {
  it("adds up each ?ref= tag's visits, agents and installer runs, busiest first", () => {
    assert.deepEqual(cameVia([
      { route: "/", answer: "html", client: "browser", ref: "readme", n: 30 },
      { route: "/install", answer: "text", client: "curl", ref: "readme", n: 4 },
      { route: "/agent", answer: "markdown", client: "agent:claude", ref: "newsletter", n: 2 },
      { route: "/", answer: "html", client: "browser", ref: "", n: 99 },
    ]), [
      { k: "readme", visits: 30, agents: 0, installRuns: 4, all: 34 },
      { k: "newsletter", visits: 0, agents: 2, installRuns: 0, all: 2 },
    ]);
  });
});

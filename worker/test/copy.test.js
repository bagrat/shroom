// Every line the Worker shows is the spec's, verbatim (TESTING-PLAN §2.4) — Region 4's
// pages and Region 6's join page — or a line the build wrote and logged in docs/DECISIONS.md's
// "Copy written during the build" table for Bagrat to ratify.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { SYSREQ, sysreqLine } from "./sysreq.js";
import { COPY, installLine, inviteExpired, inviteUsed, joinHero, joinIntro, ownSame, ownerNote, untitled, waitingFor } from "../src/worker.js";

const machine = JSON.parse(readFileSync(new URL("../../spec/machine.json", import.meta.url), "utf8"));
const said = (region) => Object.values(machine.states).filter((s) => s.region === region).flatMap((s) => s.said.map((l) => l.text));
const r4 = new Set(said(4));
const r6 = new Set(said(6));
// Any state's line.
const spec = new Set(Object.values(machine.states).flatMap((s) => s.said.map((l) => l.text)));
/** The quoted wordings in docs/DECISIONS.md's copy table. */
const decisions = (() => {
  const md = readFileSync(new URL("../../docs/DECISIONS.md", import.meta.url), "utf8");
  const table = md.split("## Copy written during the build")[1] ?? "";
  return new Set([...table.matchAll(/“([^”]+)”/g)].map((m) => m[1]));
})();

describe("copy", () => {
  // `macos` is ratified as a shape whose facts are filled at build time, so it's checked
  // against those facts below, not against r6's draft words.
  // `fullyLive` is no surface's line now — the page dropped it (DEVIATIONS #46), the popover too
  // (#52) — and the Worker keeps it only so its tests prove the page never says it.
  it.each(Object.entries(COPY).filter(([k]) => k !== "macos" && k !== "fullyLive"))("%s is a said-line of the spec, or logged in DECISIONS' copy table", (_, text) => {
    expect(spec.has(text) || decisions.has(text)).toBe(true);
  });

  it("macos keeps r6's ratified shape and states the facts that ship (the installer's floor, the packager's kinds of Mac)", () => {
    const ratified = [...r6].filter((l) => SYSREQ.test(l));
    expect(ratified.length, "r6 has one system-requirements line").toBe(1);
    expect(sysreqLine, "install.sh's MIN_MACOS and package-release.sh's arch arms are readable").not.toBeNull();
    expect(COPY.macos).toMatch(SYSREQ);
    expect(COPY.macos).toBe(sysreqLine);
  });

  it("macos says what the landing says", () => {
    const landing = readFileSync(new URL("../../site/index.html", import.meta.url), "utf8").replace(/<[^>]+>/g, "").replace(/\s+/g, " ");
    expect(landing).toContain(COPY.macos);
  });

  it("renders the templated lines exactly as the spec's examples", () => {
    expect(r4.has(COPY.finishing)).toBe(true);
    expect(r4.has(untitled("2026-08-21T12:00:00Z"))).toBe(true);
  });

  it("renders the join page's templated lines exactly as r6's examples", () => {
    for (const line of [joinHero("Bagrat"), joinIntro("Bagrat"), installLine("v.myshroom.workers.dev/join/7xk2pm"), ownSame("v.myshroom.workers.dev"),
      ownerNote("Ani"), inviteExpired("Bagrat"), inviteUsed("Bagrat")]) {
      expect(r6.has(line), line).toBe(true);
    }
    expect(r4.has(waitingFor("Bagrat"))).toBe(true);
  });
});

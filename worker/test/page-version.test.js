// B61 (tests-sys #8): the page a space runs moves only with VERSION. B50's refresh puts this
// app's page up only when the live ping answers an *older* VERSION, so a worker.js change
// shipped under the same VERSION never reaches an existing space. Every VERSION keeps the
// fingerprint of the page it shipped; changing the page without a new VERSION fails here.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { VERSION } from "../src/worker.js";
import { fingerprint } from "../dev/page-fingerprint.mjs";

const lock = JSON.parse(readFileSync(new URL("../page-version.json", import.meta.url), "utf8"));
const versions = Object.keys(lock).filter((k) => k !== "_");
const parts = (v) => v.split(".").map(Number);
const newer = (a, b) => {
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < 3; i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0);
  return false;
};

describe("the page's VERSION guard", () => {
  it("VERSION has a fingerprint row", () => {
    expect(lock[VERSION], `worker.js says VERSION ${VERSION}, but page-version.json has no row for it — add "${VERSION}": "${fingerprint()}" (node dev/page-fingerprint.mjs)`).toBeTypeOf("string");
  });

  it("the page is the one VERSION shipped — a change needs a new VERSION", () => {
    const now = fingerprint();
    expect(now, `src/worker.js or vendor/hls.min.js changed since VERSION ${VERSION} was fingerprinted. ` +
      `Existing spaces only take a newer VERSION (DECISIONS B50 c), so: bump VERSION in src/worker.js ` +
      `and the app's version in app/src-tauri/tauri.conf.json (they're held equal), then add a row ` +
      `"<new version>": "${now}" to worker/page-version.json. Never edit an existing row.`).toBe(lock[VERSION]);
  });

  it("VERSION is the newest row", () => {
    for (const v of versions) expect(v === VERSION || newer(VERSION, v), `${v} is newer than VERSION ${VERSION}`).toBe(true);
  });

  it("no two versions shipped the same page", () => {
    const sums = versions.map((v) => lock[v]);
    expect(new Set(sums).size).toBe(sums.length);
  });
});

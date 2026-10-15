// The player's hls.js is pinned: the exact file the heartbeat proved in Chromium, placed
// in every space's bucket by wiring (DECISIONS B30). A changed file is a deliberate act.
// Ahead of it, one comment carries hls.js's notice and the Apache-2.0 licence itself, so
// every viewer's browser receives them with the code, as the licence asks (B124).
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const HLS_VERSION = "1.7.3";
const HLS_SHA256 = "a12e7ee1cd64a69dcdb314157e45dafcba705bfb0b1440b7935cb265d374423e";
const FILE_SHA256 = "28ca13266d7310edf2d6ec6ede47506fe791c7828cc2abccee943247019c63e6";

const vendored = readFileSync(new URL("../vendor/hls.min.js", import.meta.url));
const end = vendored.indexOf("\n*/\n") + 4;
const [banner, code] = [vendored.subarray(0, end).toString("utf8"), vendored.subarray(end)];
const sha = (b) => createHash("sha256").update(b).digest("hex");

describe("vendored hls.js", () => {
  it("is the pinned file", () => {
    expect(sha(vendored)).toBe(FILE_SHA256);
  });

  it("is the pinned build, after its licence", () => {
    expect(sha(code)).toBe(HLS_SHA256);
    expect(code.equals(readFileSync(new URL("../../node_modules/hls.js/dist/hls.min.js", import.meta.url)))).toBe(true);
  });

  it("carries hls.js's notice and the whole Apache-2.0 licence", () => {
    expect(banner.startsWith(`/*! hls.js ${HLS_VERSION} `)).toBe(true);
    const notice = readFileSync(new URL("../../node_modules/hls.js/LICENSE", import.meta.url), "utf8").trim();
    const apache = readFileSync(new URL("../../tools/licence-texts/Apache-2.0.txt", import.meta.url), "utf8").trimEnd();
    expect(banner).toContain(notice);
    expect(banner).toContain(apache);
    expect(banner.slice(2, -4)).not.toContain("*/");
  });

  it("is the version the repo's own tests play with", () => {
    const pkg = JSON.parse(readFileSync(new URL("../../node_modules/hls.js/package.json", import.meta.url), "utf8"));
    expect(pkg.version).toBe(HLS_VERSION);
  });
});

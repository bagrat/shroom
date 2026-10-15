// The page's fingerprint: what a space runs once wiring or a refresh has put this app's page
// up — the one-file Worker and the player's vendored hls.js (DECISIONS B61). Kept beside the
// VERSION it shipped as in `page-version.json`; the Worker suite fails when the files change
// and VERSION doesn't (B50 refreshes a space only for a newer VERSION).
//
// `node dev/page-fingerprint.mjs` prints the current one.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

export const PAGE_FILES = ["src/worker.js", "vendor/hls.min.js"];

export function fingerprint(root = new URL("..", import.meta.url)) {
  const h = createHash("sha256");
  for (const f of PAGE_FILES) {
    h.update(`${f}\0`);
    h.update(readFileSync(new URL(f, root)));
    h.update("\0");
  }
  return h.digest("hex");
}

if (import.meta.url === `file://${process.argv[1]}`) console.log(fingerprint());

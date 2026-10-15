// L5's encode proof, the host's half (DECISIONS B82): the take the release's bundled encoder
// made inside the clean VM (tests/l5/encode-check), copied out, is decoded here by this Mac's
// own ffmpeg — an independent decoder — and held to the recipe's golden
// (tests/e2e/golden/hls-signature.json, the same one the reference capture and the heartbeat
// hold to). The unfurl files are checked for their sizes.
//
// The take is served to the signature reader from a loopback port (127.0.0.1 only) for the
// length of the check.
//
//   node tests/l5/encode-signature.mjs <take-dir>      prints ok/FAIL lines, exits 1 on a FAIL

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { compare, signatureOf } from "../e2e/signature.mjs";

const dir = path.resolve(process.argv[2] ?? "");
if (!process.argv[2] || !fs.existsSync(path.join(dir, "stream.m3u8"))) {
  console.error("usage: node tests/l5/encode-signature.mjs <take-dir>");
  process.exit(2);
}
const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p)) ?? name;
let failed = false;
const check = (what, ok, detail = "") => {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}${detail ? ` (${detail})` : ""}`);
  if (!ok) failed = true;
};

const server = http.createServer((req, res) => {
  const name = decodeURIComponent(new URL(req.url, "http://x").pathname).replace(/^\/v0\//, "");
  const file = path.join(dir, path.basename(name));
  if (!name || !fs.existsSync(file)) return res.writeHead(404).end();
  res.writeHead(200).end(fs.readFileSync(file));
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
try {
  const sig = await signatureOf(`http://127.0.0.1:${server.address().port}`);
  const diff = compare(sig);
  check("the VM's take matches the recipe's golden signature", diff.length === 0, diff.join("; "));
} finally {
  server.close();
}

// Every frame and every sample decodes here, with nothing reported.
const decode = (input, extra = []) => {
  try {
    const err = execFileSync(tool("ffmpeg"), ["-hide_banner", "-v", "error", ...extra, "-i", input, "-f", "null", "-"], { cwd: dir, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
    return err.trim() === "";
  } catch {
    return false;
  }
};
check("its HLS decodes whole here, picture and sound", decode("stream.m3u8", ["-allowed_extensions", "ALL"]));
check("its single-file copy decodes whole here", decode("preview.mp4"));
const probe = (file) => execFileSync(tool("ffprobe"), ["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0", file], { cwd: dir, encoding: "utf8" }).trim();
check("its poster is 1200 wide", probe("poster.jpg").startsWith("1200,"), probe("poster.jpg"));
check("its teaser is 640 wide", probe("teaser.mp4").startsWith("640,"), probe("teaser.mp4"));
process.exit(failed ? 1 : 0);

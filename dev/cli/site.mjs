// Dev only: shroom.am's Worker (site/worker.js) over a fresh build of the site, on
// loopback — so the CLI demo's agent reads /cli exactly as shroom.am will serve it.
//
//   node dev/cli/site.mjs [--port 8788]      # serves until stdin closes
import { createServer } from "node:http";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { build } from "../../site/build.mjs";
import worker from "../../site/worker.js";

const port = Number(process.argv[process.argv.indexOf("--port") + 1]) || 8788;
const out = build(join(mkdtempSync(join(tmpdir(), "shroom-site-")), "public"));
const ASSETS = {
  /** @param {Request} r */
  async fetch(r) {
    try {
      return new Response(readFileSync(join(out, decodeURIComponent(new URL(r.url).pathname))));
    } catch {
      return new Response("", { status: 404 });
    }
  },
};
const env = { ASSETS, STATS: { writeDataPoint() {} } };
const server = createServer(async (req, res) => {
  const headers = new Headers();
  for (const [k, v] of Object.entries(req.headers)) if (typeof v === "string") headers.set(k, v);
  const r = await worker.fetch(new Request(`https://shroom.am${req.url}`, { method: req.method, headers }), env);
  res.writeHead(r.status, Object.fromEntries(r.headers));
  res.end(Buffer.from(await r.arrayBuffer()));
});
server.listen(port, "127.0.0.1", () => console.log(JSON.stringify({ url: `http://127.0.0.1:${port}` })));
process.stdin.on("end", () => process.exit(0));
process.stdin.resume();

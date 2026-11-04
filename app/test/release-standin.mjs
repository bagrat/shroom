// A loopback release stand-in: serves a directory over plain HTTP on 127.0.0.1 and prints
// its port. Logs each request. With SLOW_MS set, everything but the small text files (the manifests) waits that
// long before it's sent: an update that's still downloading a while after the check.
import { createServer } from "node:http";
import { createReadStream, statSync } from "node:fs";
import { join, normalize } from "node:path";
const root = process.argv[2];
const srv = createServer((req, res) => {
  const p = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^(\.\.[/\\])+/, "");
  const f = join(root, p);
  console.error(`${req.method} ${p}`);
  const wait = p.endsWith(".txt") ? 0 : Number(process.env.SLOW_MS ?? 0);
  setTimeout(() => send(f, res), wait);
});
function send(f, res) {
  try {
    const st = statSync(f);
    if (!st.isFile()) throw new Error("not a file");
    res.writeHead(200, { "content-length": st.size });
    createReadStream(f).pipe(res);
  } catch {
    res.writeHead(404).end();
  }
}
srv.listen(0, "127.0.0.1", () => console.log(srv.address().port));

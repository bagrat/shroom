// A loopback release stand-in: serves a directory over plain HTTP on 127.0.0.1 and prints
// its port. Logs each request.
import { createServer } from "node:http";
import { createReadStream, statSync } from "node:fs";
import { join, normalize } from "node:path";
const root = process.argv[2];
const srv = createServer((req, res) => {
  const p = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^(\.\.[/\\])+/, "");
  const f = join(root, p);
  console.error(`${req.method} ${p}`);
  try {
    const st = statSync(f);
    if (!st.isFile()) throw new Error("not a file");
    res.writeHead(200, { "content-length": st.size });
    createReadStream(f).pipe(res);
  } catch {
    res.writeHead(404).end();
  }
});
srv.listen(0, "127.0.0.1", () => console.log(srv.address().port));

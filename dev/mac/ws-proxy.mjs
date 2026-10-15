// ssh's ProxyCommand for a held Mac: this process's stdin and stdout, carried as the binary
// frames of one WebSocket to the tunnel's address — what `cloudflared access tcp` does,
// without installing it. Node ≥ 22 (its own WebSocket); NODE_USE_ENV_PROXY=1 lets a session
// behind an HTTP proxy through.
//
//   ProxyCommand node dev/mac/ws-proxy.mjs %h
const host = process.argv[2];
if (!host || !/^[a-z0-9.-]+$/.test(host)) {
  console.error("usage: ws-proxy.mjs <host>");
  process.exit(64);
}
const ws = new WebSocket(`wss://${host}/`);
ws.binaryType = "arraybuffer";
// ssh speaks first: what it says before the socket is open waits here.
const waiting = [];
let open = false;
process.stdin.on("data", (chunk) => (open ? ws.send(chunk) : waiting.push(chunk)));
process.stdin.on("end", () => ws.close());
ws.addEventListener("open", () => {
  open = true;
  for (const chunk of waiting.splice(0)) ws.send(chunk);
});
ws.addEventListener("message", (event) => process.stdout.write(Buffer.from(event.data)));
ws.addEventListener("close", () => process.exit(0));
ws.addEventListener("error", (event) => {
  console.error(`ws-proxy: ${host}: ${event.message ?? event.error?.message ?? "the connection failed"}`);
  process.exit(1);
});

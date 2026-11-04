// The phone's own player against a live space (the iPhone Simulator's Mobile Safari, on a
// held hosted Mac): a page on loopback with three muted, inline videos — the live space's
// single-file copy, its stream, and the same files served here as the control — each asked
// to play, every media event and error posted to /__log, one JSON line each in the log
// file; and the same with a captions track, the real one or one that answers 404. The live space is the test account's (tests/e2e-live's a_space_held_for_a_phone).
//
//   node worker/dev/ios-live.mjs --live https://<host> --mp4 <id> --hls <id> --log <file> [--port N]
//
// Then `xcrun simctl openurl <udid> http://127.0.0.1:<port>/` and read the log.
import fs from "node:fs";
import { createServer } from "node:http";
import path from "node:path";

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const live = arg("live"), mp4 = arg("mp4"), hls = arg("hls"), logFile = arg("log", "ios-live.log");
const fixtures = path.join(path.dirname(new URL(import.meta.url).pathname), "../../tests/e2e-live/fixtures/phone");
const out = fs.createWriteStream(logFile, { flags: "a" });
const srcs = { "live-mp4": `${live}/m/${mp4}/video.mp4`, "live-hls": `${live}/m/${hls}/master.m3u8`, "local-mp4": "/f/video.mp4", "local-hls": "/f/master.m3u8",
  "local-mp4-cc": "/f/video.mp4", "local-mp4-cc404": "/f/video.mp4", "local-hls-cc": "/f/master.m3u8", "local-hls-cc404": "/f/master.m3u8", "live-mp4-cc404": `${live}/m/${mp4}/video.mp4` };
// A captions track as the page writes it (aiPlayer): the real file, or one that isn't there yet.
const track = (k) => (k.endsWith("-cc") ? `<track kind="captions" src="/f/captions.vtt" srclang="en" label="Captions">` : k.endsWith("-cc404") ? `<track kind="captions" src="/f/none.vtt" srclang="en" label="Captions">` : "");
const page = `<!doctype html><meta name="viewport" content="width=device-width"><body style="font:14px system-ui">
${Object.entries(srcs).map(([k, s]) => `<p>${k}</p><video id="${k}" playsinline muted preload="metadata" width="300" src="${s}">${track(k)}</video>`).join("\n")}
<script>
function log(k,d){fetch("/__log",{method:"POST",body:JSON.stringify({ms:Math.round(performance.now()),k:k,d:d})})}
log("ready",{ua:navigator.userAgent});
document.querySelectorAll("video").forEach(function(v){
 ["loadstart","loadedmetadata","canplay","playing","stalled","error","suspend","abort"].forEach(function(t){v.addEventListener(t,function(){log(v.id,{t:t,ready:v.readyState,net:v.networkState,err:v.error?{code:v.error.code,msg:v.error.message}:null,cur:v.currentSrc})})});
 setTimeout(function(){var r=v.play();if(r&&r.then)r.then(function(){log(v.id,{play:"ok",t:v.currentTime})},function(e){log(v.id,{play:"refused",e:e.name+": "+e.message,err:v.error?{code:v.error.code,msg:v.error.message}:null})})},1500);
 setTimeout(function(){log(v.id,{at6s:{t:v.currentTime,paused:v.paused,ready:v.readyState,err:v.error?v.error.code:null}})},25000)});
</script>`;
const types = { vtt: "text/vtt; charset=utf-8", mp4: "video/mp4", m3u8: "application/vnd.apple.mpegurl", m4s: "video/iso.segment", jpg: "image/jpeg" };
const server = createServer(async (req, res) => {
  const chunks = []; for await (const c of req) chunks.push(c);
  if (req.url === "/__log") { out.write(`${Buffer.concat(chunks).toString("utf8")}\n`); res.writeHead(204).end(); return; }
  out.write(`${JSON.stringify({ k: "http", u: req.url, range: req.headers.range ?? null })}\n`);
  if (req.url === "/") { res.writeHead(200, { "content-type": "text/html; charset=utf-8", "cache-control": "no-store" }).end(page); return; }
  if (req.url.startsWith("/f/")) {
    const file = path.join(fixtures, req.url.slice(3).split("?")[0]);
    if (!file.startsWith(fixtures) || !fs.existsSync(file)) { res.writeHead(404).end(); return; }
    const all = fs.readFileSync(file), type = types[file.split(".").pop()] ?? "application/octet-stream";
    const m = /bytes=(\d*)-(\d*)/.exec(req.headers.range ?? "");
    if (m) { const s = m[1] ? +m[1] : all.length - +m[2], e = m[1] && m[2] ? Math.min(+m[2], all.length - 1) : all.length - 1;
      res.writeHead(206, { "content-type": type, "accept-ranges": "bytes", "content-range": `bytes ${s}-${e}/${all.length}`, "content-length": e - s + 1 }).end(all.subarray(s, e + 1)); return; }
    res.writeHead(200, { "content-type": type, "accept-ranges": "bytes", "content-length": all.length }).end(all); return;
  }
  res.writeHead(404).end();
});
await new Promise((ok) => server.listen(Number(arg("port", 0)), "0.0.0.0", ok));
console.log(JSON.stringify({ port: server.address().port }));
process.stdin.resume();
process.stdin.on("end", () => { server.close(); out.end(); process.exit(0); });

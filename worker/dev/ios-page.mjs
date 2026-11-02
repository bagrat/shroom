// The page on a phone's own browser (the iPhone Simulator's Mobile Safari, on the hosted Mac):
// the real worker.js over loopback HTTP, serving recordings made from an encoded folder, with a
// logger added to every HTML page (under the page's own nonce, so its CSP stays as served).
// The logger posts what the page does — taps, clicks, the video's events, its errors — to
// /__log, one JSON line each in the log file. Never the network, never a real space.
//
//   node worker/dev/ios-page.mjs --media <folder> --log <file> [--port N]
//
// <folder> holds master.m3u8, v0/…, poster.jpg and video.mp4, as the app uploads them.
// It prints {"port", "ids"} once listening and serves until its input closes.
//   ids.stream   the stream only
//   ids.single   the single-file copy up (it plays video.mp4)
//   ids.named    the stream, with chapters and captions (naming on)
// `?probe` on a page's address also plays it muted three seconds in, with no tap.
import fs from "node:fs";
import { createServer } from "node:http";
import path from "node:path";
import { handle } from "../src/worker.js";
import { M1, metaKey, roster, space } from "../test/space-fixture.js";

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const media = arg("media");
const logFile = arg("log", "ios-page.log");
const here = path.dirname(new URL(import.meta.url).pathname);
const ids = { stream: "StrmStrmStrmStrm", single: "OneFOneFOneFOneF", named: "NamdNamdNamdNamd" };

const LOGGER = `(function(){var n=0,t0=Date.now();function log(k,d){try{fetch("/__log",{method:"POST",keepalive:true,body:JSON.stringify({n:n++,ms:Date.now()-t0,page:location.pathname+location.search,k:k,d:d})})}catch(e){}}
function who(e){var x=e&&e.target;if(!x||!x.tagName)return String(x);return x.tagName.toLowerCase()+(x.className&&typeof x.className==="string"?"."+x.className.split(" ").join("."):"")+(x.getAttribute&&x.getAttribute("aria-label")?"["+x.getAttribute("aria-label")+"]":"")}
window.addEventListener("error",function(e){log("error",{msg:e.message,src:e.filename,line:e.lineno,col:e.colno,target:e.message?null:who(e)})},true);
window.addEventListener("unhandledrejection",function(e){var r=e.reason;log("rejection",{name:r&&r.name,msg:String(r&&r.message||r)})});
document.addEventListener("securitypolicyviolation",function(e){log("csp",{dir:e.violatedDirective,uri:e.blockedURI})});
["touchstart","touchend","pointerdown","pointerup","click","dblclick","mouseover"].forEach(function(t){document.addEventListener(t,function(e){var p=e.changedTouches&&e.changedTouches[0]||e;log("input",{t:t,on:who(e),x:Math.round(p.clientX||0),y:Math.round(p.clientY||0),pt:e.pointerType})},true)});
function snap(){var v=document.getElementById("v");if(!v)return null;var r=v.error;return{src:v.currentSrc||v.src,paused:v.paused,t:Math.round(v.currentTime*10)/10,ready:v.readyState,net:v.networkState,err:r?{code:r.code,msg:r.message}:null,muted:v.muted,rate:v.playbackRate,full:!!(document.fullscreenElement||document.webkitFullscreenElement||v.webkitDisplayingFullscreen)}}
document.addEventListener("DOMContentLoaded",function(){var v=document.getElementById("v"),pl=document.querySelector(".player");
log("ready",{ua:navigator.userAgent,w:innerWidth,h:innerHeight,dpr:devicePixelRatio,hls:v?v.canPlayType("application/vnd.apple.mpegurl"):null,mse:!!window.MediaSource,mms:!!window.ManagedMediaSource,
 fsApi:!!(pl&&(pl.requestFullscreen||pl.webkitRequestFullscreen)),fsEnabled:document.fullscreenEnabled,vfs:!!(v&&v.webkitEnterFullscreen),coarse:matchMedia("(pointer:coarse)").matches,hover:matchMedia("(hover:hover)").matches,
 rects:[".pp",".seek",".time",".cc",".sp",".mu",".fs","#v",".big"].map(function(s){var e=document.querySelector(s);if(!e)return[s,null];var r=e.getBoundingClientRect(),c=document.elementFromPoint(r.left+r.width/2,r.top+r.height/2),u=document.elementFromPoint(r.left+r.width/2,r.top+4);return[s,[Math.round(r.left),Math.round(r.top),Math.round(r.width),Math.round(r.height)],who({target:c}),who({target:u}),e.hidden||getComputedStyle(e).display==="none"]})});
 if(!v)return;["loadstart","loadedmetadata","canplay","play","playing","pause","waiting","stalled","error","ended","webkitbeginfullscreen","webkitendfullscreen","seeking","seeked","volumechange","ratechange"].forEach(function(t){v.addEventListener(t,function(){log("video",{t:t,s:snap()})})});
 document.addEventListener("fullscreenchange",function(){log("fs",{s:snap()})});document.addEventListener("webkitfullscreenchange",function(){log("wfs",{s:snap()})});
 setInterval(function(){log("snap",snap())},2000);
 if(/[?&]probe/.test(location.search))setTimeout(function(){v.muted=true;var r;try{r=v.play()}catch(e){log("probe",{threw:e.name+": "+e.message});return}if(r&&r.then)r.then(function(){log("probe",{played:true,s:snap()})},function(e){log("probe",{refused:e.name+": "+e.message,s:snap()})})},3000)})})()`;

const env = space({ roster: roster({}) });
const put = (k, v) => env.BUCKET.put(k, v);
await put(".shroom/player/hls.min.js", new Uint8Array(fs.readFileSync(path.join(here, "../vendor/hls.min.js"))));
const files = ["master.m3u8", "poster.jpg", "video.mp4", ...fs.readdirSync(path.join(media, "v0")).map((n) => `v0/${n}`)];
for (const id of Object.values(ids)) for (const f of files) await put(`rec/${id}/${f}`, new Uint8Array(fs.readFileSync(path.join(media, f))));
const dur = Number(fs.readFileSync(path.join(media, "duration.txt"), "utf8")) || 20;
const base = (id, over = {}) => JSON.stringify({ schema: 1, id, state: "live", createdAt: new Date(Date.now() - 86_400_000).toISOString(), title: "A take from the simulator", durationSec: dur,
  media: { hls: `rec/${id}/master.m3u8`, mp4: null, poster: `rec/${id}/poster.jpg`, width: 1280, height: 800 }, owner: M1, ...over });
await put(metaKey(ids.stream), base(ids.stream));
await put(metaKey(ids.single), base(ids.single, { media: { hls: `rec/${ids.single}/master.m3u8`, mp4: `rec/${ids.single}/video.mp4`, poster: `rec/${ids.single}/poster.jpg`, width: 1280, height: 800 } }));
await put(metaKey(ids.named), base(ids.named, { named: { by: "ai", title: "A take from the simulator", chapters: [[0, "The start"], [Math.floor(dur / 2), "The middle"]] }, captions: { language: "en" } }));

const out = fs.createWriteStream(logFile, { flags: "a" });
const server = createServer(async (req, res) => {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  if (req.url === "/__log") { out.write(`${Buffer.concat(chunks).toString("utf8").replace(/\n/g, " ")}\n`); res.writeHead(204).end(); return; }
  out.write(`${JSON.stringify({ k: "http", m: req.method, u: req.url, range: req.headers.range ?? null, ua: (req.headers["user-agent"] ?? "").slice(0, 60) })}\n`);
  const url = `http://127.0.0.1:${server.address().port}${req.url}`;
  const r = await handle(new Request(url, { method: req.method, headers: req.headers, body: chunks.length ? Buffer.concat(chunks) : undefined }), env, Date.now());
  const headers = Object.fromEntries(r.headers);
  if ((headers["content-type"] ?? "").startsWith("text/html") && r.body) {
    const nonce = /'nonce-([^']+)'/.exec(headers["content-security-policy"] ?? "")?.[1];
    let page = await r.text();
    if (nonce) page = page.replace("<head>", `<head><script nonce="${nonce}">${LOGGER}</script>`);
    delete headers["content-length"];
    res.writeHead(r.status, headers);
    res.end(page);
    return;
  }
  out.write(`${JSON.stringify({ k: "http-out", u: req.url, st: r.status, len: headers["content-length"] ?? null, cr: headers["content-range"] ?? null })}\n`);
  res.writeHead(r.status, headers);
  if (r.body) for await (const c of r.body) res.write(c);
  res.end();
});
await new Promise((ok) => server.listen(Number(arg("port", 0)), "0.0.0.0", ok));
console.log(JSON.stringify({ port: server.address().port, ids }));
process.stdin.resume();
process.stdin.on("end", () => { server.close(); out.end(); process.exit(0); });

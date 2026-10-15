# Copy thumbnail's pasteboard (N46, B164), pasted as a person pastes it: the HTML the app and
# the page write (a picture wrapped in the link) and the plain link, put on the general
# pasteboard, then ⌘V into Safari (WebKit, Apple Mail's compose engine) on a page that
# reports what the paste carried and what it became, and into a TextEdit document. The
# picture is the repo's own test-pattern still and the link a placeholder — no real content.
# Gmail, Slack and Apple Mail need accounts this Mac doesn't have; Basecamp is Bagrat's.
T=$(mktemp -d)
IMG="https://raw.githubusercontent.com/bagrat/shroom/main/dev/proposals/thumbnail-share/picture-still.jpg"
LINK="https://v.example.workers.dev/AbCdEfGhIjKlMnOp"
HTML="<a href=\"$LINK\"><img src=\"$IMG\" width=\"600\" alt=\"Where the 502s come from\" style=\"max-width:100%\"></a>"
key() { osascript -e "tell application \"System Events\" to $1"; }
shot() { screencapture -x "$MAC_OUT/$1.png"; }
put() {
  HTML="$HTML" LINK="$LINK" osascript -l JavaScript -e 'ObjC.import("AppKit"); const e = $.NSProcessInfo.processInfo.environment;
    const pb = $.NSPasteboard.generalPasteboard; pb.clearContents;
    pb.setStringForType(e.objectForKey("HTML"), $.NSPasteboardTypeHTML); pb.setStringForType(e.objectForKey("LINK"), $.NSPasteboardTypeString);
    ObjC.deepUnwrap(pb.types).join(" ")'
}

# Safari: a page with an editor that reports the paste to a loopback server.
cat > "$T/page.html" <<'EOF'
<!doctype html><meta charset="utf-8"><title>paste</title><body style="font:16px -apple-system;margin:24px">
<p>Pasted below:</p><div id="e" contenteditable style="min-height:420px;border:1px solid #999;padding:8px"></div>
<script>const e=document.getElementById("e");e.focus();
e.addEventListener("paste",(ev)=>{const types=[...ev.clipboardData.types],html=ev.clipboardData.getData("text/html"),text=ev.clipboardData.getData("text/plain");
setTimeout(()=>{const a=e.querySelector("a"),img=e.querySelector("img");fetch("/report",{method:"POST",body:JSON.stringify({types,html,text,result:e.innerHTML,
linkedPicture:!!(a&&img&&a.contains(img)),href:a&&a.href,src:img&&img.src,loaded:!!(img&&img.complete&&img.naturalWidth)})})},2500)});</script>
EOF
node -e 'const fs=require("fs"),http=require("http"),[,page,out]=process.argv;http.createServer((q,s)=>{if(q.method==="POST"){let b="";q.on("data",c=>b+=c);q.on("end",()=>{fs.writeFileSync(out,b);s.end("ok")});return}s.setHeader("content-type","text/html");s.end(fs.readFileSync(page))}).listen(8799)' "$T/page.html" "$MAC_OUT/safari.json" &
srv=$!
sleep 1
echo "pasteboard: $(put)"
open -a Safari "http://127.0.0.1:8799/"
sleep 6; shot 1-safari-before
key 'keystroke "v" using command down' || echo "the paste keystroke failed: $?"
sleep 5; shot 2-safari-pasted

# TextEdit: a rich document, the same paste, saved.
printf '{\\rtf1\\ansi }' > "$T/paste.rtf"
put >/dev/null
open -a TextEdit "$T/paste.rtf"
sleep 5
key 'keystroke "v" using command down' || echo "the TextEdit paste failed: $?"
sleep 4; shot 3-textedit-pasted
key 'keystroke "s" using command down'; sleep 2
cp -R "$T"/paste.rtf* "$MAC_OUT/" 2>/dev/null
kill "$srv" 2>/dev/null

node -e 'const fs=require("fs"),out=process.argv[1];let r={};try{r=JSON.parse(fs.readFileSync(out+"/safari.json","utf8"))}catch{}
const rtf=(()=>{for(const f of fs.readdirSync(out).filter(f=>f.startsWith("paste.rtf"))){const p=out+"/"+f;const t=fs.statSync(p).isDirectory()?fs.readdirSync(p).map(x=>fs.readFileSync(p+"/"+x,"latin1")).join(""):fs.readFileSync(p,"latin1");return t}return ""})();
console.log("safari:",JSON.stringify({types:r.types,linkedPicture:r.linkedPicture,href:r.href,src:r.src,loaded:r.loaded}));
console.log("textedit: link "+(rtf.includes("v.example.workers.dev")?"kept":"missing")+", picture "+(/\\pict|NSAttachment|\.jpg|attachment/i.test(rtf)?"kept":"missing"));
process.exit(r.linkedPicture?0:1)' "$MAC_OUT"

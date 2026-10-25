# Does Settings · General fit its window on a real Mac? The window opened at General (the
# shroom command's settings link, as settings-link.sh opens it, on a member's invented space),
# then its frame, its web area and any scroll bar read from the accessibility tree, and the
# screen shot at the top of the pane and at its foot. A pane that outgrows the window shows a
# scroll bar and a web area taller than the window.
set -u
OUT=${MAC_OUT:-$(mktemp -d)}
home="$(getconf DARWIN_USER_TEMP_DIR)settings-fit"
quit() { pkill -f "$MAC_APP/Contents/MacOS/shroom( |$)" 2>/dev/null; sleep 2; true; }
size() {
  osascript -l JavaScript -e '
    ObjC.import("CoreGraphics");
    const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0))) || [];
    const out = list.filter(w => /shroom/.test(w.kCGWindowOwnerName) && w.kCGWindowLayer === 0)
      .map(w => "frame " + JSON.stringify(w.kCGWindowBounds));
    const se = Application("System Events");
    for (const p of se.processes.whose({ name: { _contains: "shroom" } })()) {
      for (const win of p.windows()) {
        out.push("ax window " + JSON.stringify({ size: win.size(), pos: win.position() }));
        for (const e of win.entireContents()) {
          const r = e.role();
          if (r === "AXWebArea" || r === "AXScrollArea" || r === "AXScrollBar") out.push("ax " + r + " " + JSON.stringify({ size: e.size(), pos: e.position() }));
        }
      }
    }
    out.join("\n")' 2>&1
}

quit
rm -rf "$home" && mkdir -p "$home" && chmod 700 "$home"
printf '%s\n' '{"account_id":"acc","label":"v","subdomain":"x","host":"settings-fit.invalid","bucket":"shroom-space","role":"Member","member_id":"m_AbCdEfGhIjKlMnOp","owner_name":"Ann"}' >"$home/space.json"
printf '{"v":1,"at":%s}\n' "$(($(date +%s) * 1000))" >"$home/primed.json"
chmod 600 "$home/space.json" "$home/primed.json"

open -a "$MAC_APP" "shroom://settings/agent" --args --home "$home"
sleep 12
size | tee "$OUT/size.txt"
screencapture -x "$OUT/general-lit.png"
# Back to the pane's top: Home, as a person would press it.
osascript -e 'tell application "System Events" to key code 115' 2>/dev/null
sleep 2
screencapture -x "$OUT/general-top.png"
osascript -e 'tell application "System Events" to key code 119' 2>/dev/null
sleep 2
screencapture -x "$OUT/general-foot.png"
quit
if grep -q "AXScrollBar" "$OUT/size.txt"; then
  echo "NO: Settings · General scrolls on this Mac"
  exit 1
fi
echo "verdict: Settings · General fits its window"

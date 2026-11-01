# Does every Settings tab fit its window on a real Mac (N79)? An owner-class Mac's invented
# space (so the Members tab shows; the host is `.invalid`, so nothing is ever reached), the
# window opened by the shroom command's settings link, as settings-link.sh opens it. Then for
# each tab — General, Videos, Space, Members — a click on its tab, the screen shot at the
# pane's top (Home) and at its foot (End): a pane that outgrows the window scrolls, and the
# window's part of the two shots differs. The window itself must be the WindowSpec's frame,
# 600 × 720.
set -u
OUT=${MAC_OUT:-$(mktemp -d)}
home="$(getconf DARWIN_USER_TEMP_DIR)settings-fit"
bad=0
no() { echo "NO: $*"; bad=1; }
quit() { pkill -f "$MAC_APP/Contents/MacOS/shroom( |$)" 2>/dev/null; sleep 2; true; }
# shroom's layer-0 window: "x y w h", in points (the runner's screen is 1024 × 768, at 1×).
frame() {
  osascript -l JavaScript -e '
    ObjC.import("CoreGraphics");
    const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0))) || [];
    const w = list.find(w => /shroom/.test(w.kCGWindowOwnerName) && w.kCGWindowLayer === 0);
    w ? [w.kCGWindowBounds.X, w.kCGWindowBounds.Y, w.kCGWindowBounds.Width, w.kCGWindowBounds.Height].map(Math.round).join(" ") : ""' 2>/dev/null
}
key() { osascript -e "tell application \"System Events\" to key code $1" 2>/dev/null; sleep 1.5; }

quit
rm -rf "$home" && mkdir -p "$home" && chmod 700 "$home"
printf '%s\n' '{"account_id":"acc","label":"v","subdomain":"x","host":"settings-fit.invalid","bucket":"shroom-space","role":"OwnerClass","member_id":"m_AbCdEfGhIjKlMnOp","owner_name":"Ann"}' >"$home/space.json"
printf '{"v":1,"at":%s}\n' "$(($(date +%s) * 1000))" >"$home/primed.json"
chmod 600 "$home/space.json" "$home/primed.json"

open -a "$MAC_APP" "shroom://settings/agent" --args --home "$home"
for _ in $(seq 1 30); do [ -n "$(frame)" ] && break; sleep 1; done
sleep 4
read -r X Y W H <<<"$(frame)"
echo "the window: ${X:-?},${Y:-?} ${W:-?} × ${H:-?}" | tee "$OUT/size.txt"
[ "${W:-0}" = 600 ] && [ "${H:-0}" = 720 ] || no "the window is ${W:-?} × ${H:-?}, not 600 × 720"

# The tabs' centres across the window, as the toolbar sets them (the gallery's, at 1×), and
# their row: the title bar's 32 points, then the toolbar's middle.
TAB_Y=$((${Y:-0} + 32 + 21))
# The pane under the toolbar, the part of each shot compared — above the Dock, which the
# runner's 768-point screen puts over the window's foot (its top edge alone differs between
# two shots).
CROP_Y=$((${Y:-0} + 32 + 44))
foot=$((${Y:-0} + ${H:-720})); [ "$foot" -gt 690 ] && foot=690
CROP_H=$((foot - CROP_Y))
# General last, by its own click: the settings link lights the agent's row for a moment, and
# another tab puts that light out.
for tab in Videos:258 Space:333 Members:416 General:180; do
  name=${tab%%:*} x=$((${X:-0} + ${tab##*:}))
  cliclick "c:$x,$TAB_Y"
  sleep 2.5
  # An empty spot of the toolbar takes the keyboard from any field (Members' composer has
  # it), so Home and End scroll the page rather than move a caret.
  cliclick "c:$((${X:-0} + 570)),$TAB_Y"
  sleep 0.5
  key 115 # Home
  screencapture -x "$OUT/$name-top.png"
  key 119 # End
  screencapture -x "$OUT/$name-foot.png"
  for s in top foot; do sips -c "$CROP_H" 600 --cropOffset "$CROP_Y" "${X:-0}" "$OUT/$name-$s.png" --out "$OUT/$name-$s-pane.png" >/dev/null; done
  if cmp -s "$OUT/$name-top-pane.png" "$OUT/$name-foot-pane.png"; then
    echo "  $name fits its window"
  else
    no "Settings · $name scrolls on this Mac"
  fi
done
quit
[ "$bad" = 0 ] && echo "verdict: Settings is 600 × 720 and every tab fits it"
exit "$bad"

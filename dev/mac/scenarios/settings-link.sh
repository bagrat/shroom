# The shroom command's settings link (spec N62, DECISIONS B216; Bagrat, 2026-10-18): with
# General's switch off, the command's recording refusal carries `shroom://settings/agent`,
# and the link opens shroom — running or not — at Settings · General with the switch's row
# lit. It never flips the switch. The link is handed over as `open -a <app> <link>` (the
# release bundle claims the scheme; a dev bundle never does, as with join links: N50), cold
# and warm. The home holds an invented member's space (Settings needs a space, B160; the
# host is `.invalid`, so nothing is ever reached). Mid-take it opens nothing: the core's
# tests (crates/shroom-core/tests/cli.rs) — this runner records nothing.
set -u
OUT=${MAC_OUT:-$(mktemp -d)}
LINK="shroom://settings/agent"
cli="$MAC_APP/Contents/MacOS/shroom-cli"
home="$(getconf DARWIN_USER_TEMP_DIR)settings-link"
LOG="$home/logs/run.log"
bad=0
no() { echo "NO: $*"; bad=1; }
quit() { pkill -f "$MAC_APP/Contents/MacOS/shroom( |$)" 2>/dev/null; sleep 2; true; }
# How many times the core took the link (its arm, as the runtime logs every event's).
taken() { local n; n=$(grep -c "core settings-link " "$LOG" 2>/dev/null); echo "${n:-0}"; }
# shroom's Settings window on screen: a layer-0 window of its process, 600 × 620 (WindowSpec).
settings_shown() {
  osascript -l JavaScript -e '
    ObjC.import("CoreGraphics");
    const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0))) || [];
    list.some(w => /shroom/.test(w.kCGWindowOwnerName) && w.kCGWindowLayer === 0 && Math.round(w.kCGWindowBounds.Width) === 600 && Math.round(w.kCGWindowBounds.Height) === 620) ? "yes" : "no"' 2>/dev/null
}
switch_state() { grep -q '"agent_recording":true' "$home/prefs.json" 2>/dev/null && echo on || echo off; }
refusal() { SHROOM_HOME="$home" "$cli" record start --no-launch --json; }
wait_socket() {
  for _ in $(seq 1 120); do [ -S "$home/control.sock" ] && return 0; sleep 0.5; done
  no "the app made no control socket"
  return 1
}

[ -x "$cli" ] || { echo "NO: the dev bundle carries no shroom command at $cli"; exit 1; }
quit
rm -rf "$home" && mkdir -p "$home" && chmod 700 "$home"
printf '%s\n' '{"account_id":"acc","label":"v","subdomain":"x","host":"settings-link.invalid","bucket":"shroom-space","role":"Member","member_id":"m_AbCdEfGhIjKlMnOp","owner_name":"Ann"}' >"$home/space.json"
# A Mac whose permissions chapter is done (primed), as a set-up Mac is: no setup window.
printf '{"v":1,"at":%s}\n' "$(($(date +%s) * 1000))" >"$home/primed.json"
chmod 600 "$home/space.json" "$home/primed.json"

echo "── the refusal carries the link"
open -n "$MAC_APP" --args --home "$home"
wait_socket
j=$(refusal); code=$?
echo "  $j"
[ "$code" = 10 ] || no "record start exited $code, wanted 10 (recording_off)"
printf '%s' "$j" | grep -q '"error":"recording_off"' || no "not recording_off"
printf '%s' "$j" | grep -q '"settings_link":"shroom://settings/agent"' || no "no settings_link in the answer"
SHROOM_HOME="$home" "$cli" record start --no-launch 2>&1 | sed 's/^/  says: /'
quit

echo "── cold: open -a <app> \"$LINK\""
open -a "$MAC_APP" "$LINK" --args --home "$home"; echo "  open exit $?"
for i in $(seq 1 30); do [ "$(taken)" -ge 1 ] && break; sleep 1; done
[ "$(taken)" -ge 1 ] && echo "  the core took the link after ~${i}s" || no "the link never reached the core at a cold launch"
for _ in $(seq 1 10); do [ "$(settings_shown)" = yes ] && break; sleep 0.5; done
[ "$(settings_shown)" = yes ] && echo "  Settings is on screen" || no "no Settings window on screen after the link"
# The window lights the row for its own first moments, however long it took to draw.
sleep 1
screencapture -x "$OUT/cold-lit.png"
sleep 4
screencapture -x "$OUT/cold-after.png"
[ "$(switch_state)" = off ] || no "the link turned the switch on"
ps -axo args | grep "$MAC_APP/Contents/MacOS/" | grep -v grep | grep -q "settings/agent" && no "the link is on the app's command line"
quit

echo "── warm: the app running, Settings closed, the link"
open -n "$MAC_APP" --args --home "$home"
wait_socket
sleep 2
before=$(taken)
[ "$(settings_shown)" = no ] || echo "  (a window of Settings' size was already up)"
open -a "$MAC_APP" "$LINK"
for i in $(seq 1 15); do [ "$(taken)" -gt "$before" ] && break; sleep 1; done
[ "$(taken)" -gt "$before" ] && echo "  the core took the link (warm)" || no "the link never reached the running app"
for _ in $(seq 1 10); do [ "$(settings_shown)" = yes ] && break; sleep 0.5; done
[ "$(settings_shown)" = yes ] && echo "  Settings is on screen" || no "no Settings window on screen after the link (warm)"
sleep 0.5
screencapture -x "$OUT/warm-lit.png"

echo "── the link changed nothing: still off, still refused"
[ "$(switch_state)" = off ] || no "the link turned the switch on"
refusal | grep -q '"error":"recording_off"' || no "after the link, the command isn't refused"
grep -E "core settings-link|link:" "$LOG" | tail -n 6 | sed 's/^/  log: /'
quit
[ "$bad" = 0 ] && echo "verdict: the refusal carries the link; the link opens Settings at the switch, cold and warm, and flips nothing"
exit "$bad"

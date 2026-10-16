# N50 (DECISIONS B176): a join link handed over by `open -a <app> "https://…/join/<code>"`
# opens the invite, cold and warm; any other web address is ignored; the link is never on
# the app's command line. The space is invented, so the invite window ends at its
# "isn't there anymore" face — which is the invite window, reached by this door.
set -u
LINK="https://probe-owner.example.workers.dev/join/AbCdEfGh12345678"
OUT=${MAC_OUT:-$(mktemp -d)}
home="$(getconf DARWIN_USER_TEMP_DIR)open-join"
LOG="$home/logs/run.log"
rm -rf "$home"
bad=0
no() { echo "NO: $*"; bad=1; }
lines() { grep -c "member: an invite for probe-owner.example.workers.dev arrived by a link opened with shroom" "$LOG" 2>/dev/null || echo 0; }
quit() { pkill -f "$MAC_APP/Contents/MacOS/" 2>/dev/null; sleep 2; true; }

echo "── cold: open -a <app> \"<join link>\""
open -a "$MAC_APP" "$LINK" --args --home "$home"; echo "  open exit $?"
for i in $(seq 1 30); do [ "$(lines)" -ge 1 ] && break; sleep 1; done
[ "$(lines)" -ge 1 ] && echo "arrived (cold) after ~${i}s" || no "no arrival logged at cold launch"
sleep 4
grep -E "member:|\"setup\":\"join" "$LOG" | sed 's/^/  log: /' | tail -n 6
grep -q '"setup":"join-' "$LOG" && echo "the invite window opened" || no "the invite window never opened"
ps -axo args | grep "$MAC_APP/Contents/MacOS/" | grep -v grep | sed 's/^/  argv: /'
ps -axo args | grep "$MAC_APP/Contents/MacOS/" | grep -v grep | grep -q "join/" && no "the link is on the app's command line"
screencapture -x "$OUT/cold.png"

echo "── warm: a web address that isn't a join link is ignored"
before=$(grep -c "member:" "$LOG")
open -a "$MAC_APP" "https://shroom.am/"; sleep 4
after=$(grep -c "member:" "$LOG")
[ "$after" = "$before" ] && echo "ignored: nothing logged" || no "a non-join web address reached the member flow: $(grep member: "$LOG" | tail -n 1)"
pgrep -lx Safari >/dev/null && echo "  (Safari is running)"

echo "── warm: the join link again"
open -a "$MAC_APP" "$LINK"
for i in $(seq 1 15); do [ "$(lines)" -ge 2 ] && break; sleep 1; done
[ "$(lines)" -ge 2 ] && echo "arrived (warm)" || no "no arrival logged while running"
screencapture -x "$OUT/warm.png"
quit
[ "$bad" = 0 ] && echo "verdict: a join link opened with shroom opens the invite, cold and warm; nothing else gets in"
exit "$bad"

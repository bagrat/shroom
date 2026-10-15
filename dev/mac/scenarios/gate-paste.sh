# The card gate reached as a person reaches it: the dev app opened plain (no drive) against
# the Cloudflare fake, Return through the welcome and the account step, the token pasted
# with ⌘V into the window, then the gate held 40 s with the screen shot every 8 s
# (CHECKPOINT §10, 3.2: the stranger run's window stayed on a spinner after the paste).
T=$(mktemp -d)
mkfifo "$T/in"
target/debug/cloudfake --state card-gate < "$T/in" > "$T/fake.out" &
exec 3> "$T/in"
for _ in $(seq 1 50); do [ -s "$T/fake.out" ] && break; sleep 0.2; done
api=$(head -1 "$T/fake.out" | python3 -c 'import json,sys; print(json.load(sys.stdin)["api"])')
r2=$(head -1 "$T/fake.out" | python3 -c 'import json,sys; print(json.load(sys.stdin)["r2"])')
open -n "$MAC_APP" --args --home "$T/home" --cloud-fake "$api" "$r2" --relaunched=install
key() { osascript -e "tell application \"System Events\" to $1"; }
shot() { screencapture -x "$MAC_OUT/$1.png"; }
sleep 8; shot 1-welcome
key 'key code 36'; sleep 3; shot 2-account
key 'key code 36'; sleep 3; shot 3-token
printf %s "$(openssl rand -hex 20)" | pbcopy
key 'keystroke "v" using command down' || echo "the paste keystroke failed: $?"
for i in 1 2; do sleep 8; shot "4-after-paste-$i"; cp "$T/home/status.json" "$MAC_OUT/status-$i.json" 2>/dev/null; done
# Paint or scene? Poke the window: hover, a click on its blank foot, then a one-point resize
# (a full relayout and repaint) — a screenshot after each.
cliclick m:512,600 || true; sleep 3; shot 6-hover
cliclick c:512,640 || true; sleep 3; shot 7-click
key 'tell process "shroom-dev" to set size of window 1 to {521, 601}' || echo "resize failed"; sleep 3; shot 8-resized
sleep 8; shot 4-after-paste-3
echo clear-gates >&3
sleep 12; shot 5-cleared
exec 3>&-
pkill -f "$MAC_APP/Contents/MacOS" 2>/dev/null
cp -R "$T/home/logs" "$MAC_OUT/" 2>/dev/null; ls -R "$T/home" > "$MAC_OUT/home.txt"
exit 0

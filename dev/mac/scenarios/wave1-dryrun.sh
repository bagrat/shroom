# The launch dry run on a clean Mac, the way a person meets shroom: the signed release the
# install line gave (e2e-dryrun.yml ran `curl -fsSL https://shroom.am/install | sh` before
# this, and checked it opened), set up through its own wizard on the test account with the
# product's space cleared first, one take whose link plays, Send feedback from the popover
# with the log ticked, the popover kept at 2× for the record, a teammate joined by the
# installer's invite — then the Homebrew line on the same Mac. Only e2e-dryrun.yml runs it:
# it needs the run's token (E2E_TOKEN_FILE) and the release installed at MAC_APP.
#
#   setup    the welcome to all-set, a person's clicks; a space that was already there fails
#   take     Start recording, then Finish & Share; the link's page answers, the take goes
#            live, its stream answers, and Safari opens it
#   feedback the waving hand, the words, the log box ticked, Send, "Sent — thank you."
#   2x       the popover on a 2× display (dev/mac/hidpi.m), whole, the waving hand hovered
#   member   an invite from the owner's shroom command; the install line with --join on a
#            fresh home; Join to all-set, a member
#   brew     the app and its home moved off, then `brew install --cask bagrat/shroom/shroom
#            && open ~/Applications/shroom.app`: the expected version, the release's signer,
#            it runs; then zapped
#
# Each window a person sees is kept whole in $MAC_OUT/shots, the checks in checks.tsv, and
# the moments that matter (when feedback was sent) in times.txt. Nothing secret is written:
# the token goes by the clipboard, cleared after, and the invite link by a mode-600 file.
# mac-limit: 3000
set -u
APP=${MAC_APP:?}
OUT=${MAC_OUT:?}
SHOTS="$OUT/shots"
CHECKS="$OUT/checks.tsv"
TIMES="$OUT/times.txt"
HOME_DIR="$HOME/.shroom"
EXPECT=$(sed -n 's/^expect=//p' dev/e2e/dryrun-request 2>/dev/null | head -n 1)
SHROOM="$HOME/.local/bin/shroom"
AX="$PWD/dev/mac/ax.js"
export SHROOM_REF=ci HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1 HOMEBREW_NO_ANALYTICS=1
mkdir -p "$SHOTS"
: >"$CHECKS"
: >"$TIMES"
failed=0
n=0

say_() { printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*"; }
check() { # <name> <ok 0|1> <seen>
  local mark=pass
  [ "$2" = 0 ] || { mark=fail; failed=1; }
  printf '%s\t%s\t%s\n' "$mark" "$1" "$(printf '%s' "$3" | tr '\t\n' '  ' | cut -c1-300)" | tee -a "$CHECKS"
}
stop() { check "$1" 1 "$2"; shot "stopped"; finish_up; exit 1; }
ax() { osascript -l JavaScript "$AX" shroom 2>/dev/null; }
has() { ax | awk -F'\t' -v l="$1" '$1 != "W" && index($3, l) == 1 { f = 1 } END { exit !f }'; }
wait_for() { # <label prefix> <seconds>
  local t=0
  while [ "$t" -lt "$2" ]; do has "$1" && return 0; sleep 2; t=$((t + 2)); done
  return 1
}
# A button, switch or field by its exact label — never a line of text that says the same.
click() { # <label> [seconds to wait for it]
  local xy t=0
  while [ "$t" -le "${2:-20}" ]; do
    xy=$(ax | awk -F'\t' -v l="$1" '$1 != "W" && $2 != "AXStaticText" && $2 != "AXHeading" && $3 == l && $7 == "on" { print $4; exit }')
    [ -n "$xy" ] && break
    sleep 2
    t=$((t + 2))
  done
  [ -n "$xy" ] || { say_ "no '$1' to click"; return 1; }
  cliclick "c:$xy"
  say_ "clicked '$1' at $xy"
}
where() { ax | awk -F'\t' -v r="$1" -v l="$2" '$1 != "W" && $2 == r && (l == "" || $3 == l) { print $4; exit }'; }
value_of() { ax | awk -F'\t' -v r="$1" -v l="$2" '$1 != "W" && $2 == r && $3 == l { print $6; exit }'; }
# Every shroom window, whole, at its own frame; and the screen.
shot() { # <name>
  n=$((n + 1))
  local name i=0 frame
  name=$(printf '%02d-%s' "$n" "$1")
  cliclick "m:120,600"
  sleep 1.5
  ax | awk -F'\t' '$1 == "W" { print $4 }' | while read -r frame; do
    i=$((i + 1))
    [ "${frame##*,}" -gt 20 ] 2>/dev/null || continue
    screencapture -x -o -R "$frame" "$SHOTS/$name-w$i.png"
  done
  screencapture -x "$SHOTS/$name-screen.png"
  ax >"$SHOTS/$name.ax.txt"
  say_ "shot $name"
}
popover() {
  local try xy
  for try in 1 2 3 4; do
    xy=$(osascript -e 'tell application "System Events" to tell process "shroom" to get {position, size} of menu bar item 1 of menu bar 2' 2>/dev/null | awk -F', ' '{ printf "%d,%d", $1 + $3 / 2, $2 + $4 / 2 }')
    if [ -n "$xy" ]; then cliclick "c:$xy"; else osascript -e 'tell application "System Events" to tell process "shroom" to click menu bar item 1 of menu bar 2' >/dev/null 2>&1; fi
    sleep 4
    ax | grep -q '^W' && return 0
    sleep $((try * 3))
  done
  say_ "no popover came up"
  return 1
}
esc() { osascript -e 'tell application "System Events" to key code 53' >/dev/null 2>&1; sleep 1; }
close_window() { osascript -e 'tell application "System Events" to tell process "shroom" to keystroke "w" using command down' >/dev/null 2>&1; sleep 1; }
quit_app() {
  pkill -f "$APP/Contents/MacOS/" 2>/dev/null
  for _ in $(seq 1 20); do pgrep -f "$APP/Contents/MacOS/" >/dev/null || return 0; sleep 1; done
  pkill -9 -f "$APP/Contents/MacOS/" 2>/dev/null
  sleep 1
}
version() { plutil -extract CFBundleShortVersionString raw -o - "$APP/Contents/Info.plist" 2>/dev/null; }
host() { node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).host' "$HOME_DIR/space.json" 2>/dev/null; }
keep_log() { cp "$HOME_DIR/logs/run.log" "$OUT/run-$1.log" 2>/dev/null; }
when() { printf '%s\t%s\n' "$1" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" | tee -a "$TIMES"; }
finish_up() {
  pbcopy </dev/null
  keep_log last
  [ -n "${INVITE:-}" ] && rm -f "$INVITE"
  [ -n "${display:-}" ] && kill "$display" 2>/dev/null
  quit_app
}

# ── setup ─────────────────────────────────────────────────────────────────────────────
say_ "the release the install line gave: $(version)"
[ -z "$EXPECT" ] || [ "$(version)" = "$EXPECT" ] || stop "the install line gave $EXPECT" "$(version)"
open "$APP"
wait_for "Set up my space" 90 || stop "setup: the welcome came up" "no Set up my space"
shot "welcome"
click "Set up my space" || stop "setup: Set up my space" "not clickable"
click "I have one" || stop "setup: I have one" "not clickable"
click "Paste your token" || stop "setup: the token field" "not found"
pbcopy <"${E2E_TOKEN_FILE:?}"
osascript -e 'tell application "System Events" to keystroke "v" using command down'
sleep 1
pbcopy </dev/null
say_ "pasted the run's token (the clipboard cleared)"
seen=""
t=0
while [ "$t" -lt 480 ]; do
  now=$(ax | awk -F'\t' '$1 != "W" && ($2 == "AXButton" || $2 == "AXHeading") { print $3 }' | sort | tr '\n' '|')
  if [ "$now" != "$seen" ]; then shot "wizard"; seen=$now; fi
  has "You're all set" && break
  has "Found your space" && stop "setup: a fresh space" "a space was already there: $(ax | grep -F 'Found your space' | cut -f3 | head -n 1)"
  case "|$now" in
    *"|Add credit card|"* | *"|Claim this name|"*) stop "setup: the test account's gates" "the wizard asks: $now" ;;
  esac
  for b in "Use this account" "Allow recording" "Reopen shroom"; do
    case "|$now" in *"|$b|"*) click "$b" 2 && sleep 3 ;; esac
  done
  sleep 3
  t=$((t + 3))
done
has "You're all set" || stop "setup: the wizard reached all-set" "it ended at: $now"
shot "all-set"
check "setup: a fresh space, through the release's own wizard" 0 "$(version) on $(host) after ${t} s"
click "Close" 5 || true
sleep 3

# ── 2x ────────────────────────────────────────────────────────────────────────────────
# Before any take, so the popover shows no video of the run's. Its text is kept beside the
# shots (2x-*.ax.txt) and checked for a space's address or an email.
say_ "2x: a Retina display, the popover on it"
tool="$(getconf DARWIN_USER_TEMP_DIR)hidpi"
if clang -fobjc-arc -framework Foundation -framework CoreGraphics dev/mac/hidpi.m -o "$tool" 2>"$OUT/hidpi-build.log"; then
  "$tool" >"$OUT/hidpi.txt" 2>"$OUT/hidpi.log" &
  display=$!
  for _ in $(seq 1 20); do [ -s "$OUT/hidpi.txt" ] && break; sleep 1; done
  sleep 5
fi
if grep -q "main yes" "$OUT/hidpi.txt" 2>/dev/null && popover; then
  sleep 3
  frame=$(ax | awk -F'\t' '$1 == "W" { print $4; exit }')
  ax >"$SHOTS/2x-popover.ax.txt"
  cliclick "m:120,600"
  sleep 1.5
  screencapture -x -o -R "$frame" "$SHOTS/2x-popover.png"
  hand=$(where AXButton "Send feedback")
  if [ -n "$hand" ]; then
    cliclick "m:$hand"
    sleep 3
    # The window and a margin below it, so the hover's tooltip is in the picture.
    wide=$(printf '%s' "$frame" | awk -F, '{ printf "%d,%d,%d,%d", $1 - 40, $2, $3 + 80, $4 + 60 }')
    screencapture -x -R "$wide" "$SHOTS/2x-popover-hover.png"
  fi
  screencapture -x "$SHOTS/2x-screen.png"
  px=$(sips -g pixelWidth "$SHOTS/2x-popover.png" 2>/dev/null | awk '/pixelWidth/ { print $2 }')
  pt=${frame#*,*,}
  pt=${pt%,*}
  check "2x: the popover at twice its points, the waving hand in it" "$([ -n "$hand" ] && [ "${px:-0}" -ge $((pt * 2)) ] && echo 0 || echo 1)" "${px:-?} px over ${pt} pt; Send feedback at ${hand:-nowhere}"
  leaks=$(cut -f3,6 "$SHOTS/2x-popover.ax.txt" | grep -Eio '[a-z0-9.-]+\.workers\.dev|[^ ]+@[^ ]+\.[a-z]+' | sort -u | tr '\n' ' ')
  check "2x: no space address or email in the popover's text" "$([ -z "$leaks" ] && echo 0 || echo 1)" "${leaks:-none}"
  esc
else
  check "2x: a 2× display with the popover on it" 1 "$(cat "$OUT/hidpi.txt" "$OUT/hidpi.log" "$OUT/hidpi-build.log" 2>/dev/null | tail -n 3)"
fi
[ -n "${display:-}" ] && kill "$display" 2>/dev/null
display=""
sleep 5

# ── take ──────────────────────────────────────────────────────────────────────────────
say_ "take: Start recording, then Finish & Share"
popover || stop "take: the popover" "didn't come up"
shot "popover"
click "Start recording" 10 || stop "take: Start recording" "not in the popover"
sleep 6
say -r 170 "This is the launch dry run, recording a short take to check the link works." &
shot "recording"
wait
sleep 4
popover
shot "paused"
click "Finish & Share" 10
check "take: recorded with Start recording and Finish & Share" $? "$(ax | awk -F'\t' '$2 == "AXButton" { printf "%s · ", $3 }')"
sleep 5
first=$("$SHROOM" list --json --limit 1 2>/dev/null)
id=$(printf '%s' "$first" | node -p 'JSON.parse(require("fs").readFileSync(0,"utf8")).videos?.[0]?.id ?? ""' 2>/dev/null)
link=$(printf '%s' "$first" | node -p 'JSON.parse(require("fs").readFileSync(0,"utf8")).videos?.[0]?.link ?? ""' 2>/dev/null)
copied=$(pbpaste 2>/dev/null)
say_ "the take: ${id:-none}, ${link:-no link}; the clipboard has ${copied:-nothing}"
[ -n "$link" ] || stop "take: a link" "shroom list said: $(printf '%s' "$first" | cut -c1-200)"
check "take: the link was copied when it stopped" "$([ "$copied" = "$link" ] && echo 0 || echo 1)" "clipboard: ${copied:-nothing}"
code=$(curl -s -o "$OUT/page.html" -w '%{http_code}' --max-time 30 "$link")
check "take: the link's page answers" "$([ "$code" = 200 ] && echo 0 || echo 1)" "HTTP $code"
t=0
state=""
while [ "$t" -lt 600 ]; do
  status=$(curl -fsS --max-time 15 "https://$(host)/_shroom/status/$id" 2>/dev/null)
  state=$(printf '%s' "$status" | node -p 'const s=JSON.parse(require("fs").readFileSync(0,"utf8")); s.state + (s.drained ? " drained" : "")' 2>/dev/null)
  [ "$state" = "live drained" ] && break
  sleep 5
  t=$((t + 5))
done
check "take: live and whole on the page" "$([ "$state" = "live drained" ] && echo 0 || echo 1)" "${state:-unread} after ${t} s"
code=$(curl -s -o "$OUT/master.m3u8" -w '%{http_code}' --max-time 30 "https://$(host)/m/$id/master.m3u8")
check "take: its stream answers" "$([ "$code" = 200 ] && head -n 1 "$OUT/master.m3u8" | grep -q '#EXTM3U' && echo 0 || echo 1)" "HTTP $code, $(head -c 40 "$OUT/master.m3u8" | tr '\n' ' ')"
open -a Safari "$link"
sleep 15
cliclick "m:120,600"
screencapture -x "$SHOTS/link-in-safari.png"
say_ "Safari has the link open"
sleep 10
screencapture -x "$SHOTS/link-in-safari-later.png"
pkill -x Safari 2>/dev/null
sleep 2
keep_log take

# ── feedback ──────────────────────────────────────────────────────────────────────────
say_ "feedback: the waving hand, the log ticked, Send"
popover || stop "feedback: the popover" "didn't come up"
click "Send feedback" 10 || stop "feedback: the waving hand" "not in the popover"
sleep 4
shot "feedback-open"
words=$(where AXTextArea "")
[ -n "$words" ] || stop "feedback: the words' field" "none"
cliclick "c:$words"
sleep 1
osascript -e 'tell application "System Events" to keystroke "TEST from the wave-1 launch dry run, sent from the Send feedback window in shroom on a hosted test Mac, with the log ticked. Not a real report - please ignore."'
sleep 2
log_on=$(value_of AXCheckBox "Include shroom’s recent log")
check "feedback: Include shroom’s recent log is ticked" "$([ "$log_on" = 1 ] || [ "$log_on" = true ] && echo 0 || echo 1)" "value ${log_on:-not found}"
shot "feedback-filled"
click "Send" 60 || stop "feedback: Send" "never enabled"
when "feedback from the app's popover: Send pressed"
wait_for "Sent — thank you." 60
ok=$?
shot "feedback-sent"
check "feedback: Sent — thank you." "$ok" "$(ax | awk -F'\t' '$2 == "AXStaticText" { printf "%s · ", $3 }' | cut -c1-240)"
[ "$ok" = 0 ] && when "feedback from the app's popover: sent"
click "Done" 5 || close_window
sleep 2

# ── member ────────────────────────────────────────────────────────────────────────────
say_ "member: an invite, the install line with it, Join"
INVITE=$(mktemp)
chmod 600 "$INVITE"
"$SHROOM" invite "Ann Lee" --json 2>/dev/null | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const j=JSON.parse(s);process.stdout.write(j.link ?? j.invite?.link ?? "")})' >"$INVITE"
[ -s "$INVITE" ] || stop "member: an invite from the owner's shroom command" "no link came back"
check "member: the owner invited a teammate" 0 "Ann Lee"
keep_log owner
quit_app
mv "$HOME_DIR" "$HOME/.shroom-owner"
curl -fsSL "https://shroom.am/install?ref=ci" | sh -s -- --join "$(cat "$INVITE")" 2>&1 | tr '\r' '\n' | grep -v '^#' | tail -n 3
rm -f "$INVITE"
INVITE=""
wait_for "Join" 90 || stop "member: the teammate's welcome" "no Join"
shot "member-welcome"
click "Join" || stop "member: Join" "not clickable"
t=0
while [ "$t" -lt 240 ]; do
  has "You're all set" && break
  click "Allow recording" 2 && sleep 3
  click "Reopen shroom" 1 && sleep 3
  sleep 3
  t=$((t + 6))
done
shot "member-all-set"
has "You're all set"
check "member: joined by the invite" $? "$(node -p 'const s=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")); s.role + " of " + s.owner_name + "'"'"'s space"' "$HOME_DIR/space.json" 2>/dev/null)"
role=$(node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).role' "$HOME_DIR/space.json" 2>/dev/null)
check "member: a member of the owner's space, on its address" "$([ "$role" = member ] && echo 0 || echo 1)" "${role:-no role} on $(host)"
click "Close" 5
sleep 3
popover && shot "member-popover" && esc
keep_log member
quit_app

# ── brew ──────────────────────────────────────────────────────────────────────────────
# The same Mac, the install line's copy and both homes moved off, as a person who chose the
# other line would have it.
say_ "brew: the Homebrew line"
mv "$HOME_DIR" "$HOME/.shroom-member"
rm -rf "$APP" "$SHROOM"
said=$(brew install --cask bagrat/shroom/shroom 2>&1 && open ~/Applications/shroom.app 2>&1); rc=$?
printf '%s\n' "$said" | tail -n 8
check "brew: brew install --cask bagrat/shroom/shroom && open ~/Applications/shroom.app" "$rc" "exit $rc"
sleep 10
check "brew: it gave ${EXPECT:-the latest}" "$([ -z "$EXPECT" ] || [ "$(version)" = "$EXPECT" ] && echo 0 || echo 1)" "$(version)"
leaf=$(tr -d ' \n' <tools/release-identity.sha1)
codesign --verify --deep --strict -R "=identifier \"am.shroom\" and certificate leaf = H\"$leaf\"" "$APP" 2>/dev/null
check "brew: the release's signer" $? ""
pgrep -f "$APP/Contents/MacOS/" >/dev/null
check "brew: it opened" $? ""
wait_for "Set up my space" 60
shot "brew-welcome"
quit_app
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
brew untap bagrat/shroom >/dev/null 2>&1

finish_up
echo "checks: $(grep -c '^pass' "$CHECKS") passed, $(grep -c '^fail' "$CHECKS") failed"
cat "$TIMES"
exit "$failed"

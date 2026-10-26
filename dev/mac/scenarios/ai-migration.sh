# The AI-on migration (spec N76, DECISIONS B235) the way a person meets it, on the test
# account: the signed 0.1.27 release installed by its version through the one-line
# installer, and set up through its own wizard with the AI left off and the old offer
# never answered, then updated in place to the early build, as an early-channel Mac is.
# Then the two guards: a Mac that said Not now to the old offer, and a member's Mac. Only
# e2e-migration.yml runs it: it needs the run's token (E2E_TOKEN_FILE) and the release
# installed at MAC_APP.
#
#   A. never answered → updated: the first popover says “AI features are turned on.” with
#      Turn off and OK; the page answers its ping with the AI; a take with words in it gets
#      a name; Turn off re-wires the page without it and says so; Settings' switch agrees.
#   B. Not now on 0.1.27 (another Mac of the owner's: a fresh home, reconnected) → updated:
#      no banner, and the AI stays off.
#   C. a member's Mac (a fresh home, joined by the installer's invite) of a space whose AI
#      is on: “AI is on in <owner>'s space.” with OK, once.
#
# Each window a person sees is kept whole in $MAC_OUT/shots (the window's own frame), the
# checks in checks.tsv. Nothing secret is written: the token goes by the clipboard, cleared
# after, and the invite link by a mode-600 file.
# mac-limit: 3600
set -u
APP=${MAC_APP:?}
OUT=${MAC_OUT:?}
SHOTS="$OUT/shots"
CHECKS="$OUT/checks.tsv"
HOME_DIR="$HOME/.shroom"
# The last release before the AI was on by default (B235 shipped in 0.1.30-early.1 and
# 0.1.33): a Mac still on it is the one the migration meets. It is installed by its version
# (dev/e2e/install-release.sh, here and by e2e-migration.yml), since the installer's own
# latest already has the AI on and would show no old offer to migrate from.
FROM=0.1.27
# The build an early-channel Mac takes (B220): the early pointer's, or the latest release
# once that is newer — read, rather than pinned, so the next build doesn't outdate it.
pointer() { curl -fsSL --retry 3 --max-time 30 "https://github.com/bagrat/shroom/releases/$1/manifest.txt" | sed -n 's/^version=//p'; }
TO=$(printf '%s\n%s\n' "$(pointer download/early)" "$(pointer latest/download)" | awk -F'[.-]' 'NF {
  k = sprintf("%09d %09d %09d %09d", $1, $2, $3, (NF > 3 ? $5 : 999999999)); if (k > best) { best = k; v = $0 } } END { print v }')
[ -n "$TO" ] || { echo "ai-migration: couldn't read the early pointer's or the latest release's version" >&2; exit 1; }
SHROOM="$HOME/.local/bin/shroom"
AX="$PWD/dev/mac/ax.js"
export SHROOM_REF=ci
mkdir -p "$SHOTS"
: >"$CHECKS"
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
# A switch: the whole row is the element and its toggle is drawn at one end, so it is
# pressed as VoiceOver presses it.
toggle() { # <label>
  local said
  said=$(osascript -l JavaScript "$AX" shroom press AXCheckBox "$1" 2>&1)
  say_ "switch '$1': $said"
  [ "$said" = pressed ]
}
value_of() { ax | awk -F'\t' -v l="$1" '$1 != "W" && $2 != "AXStaticText" && $3 == l { print $6; exit }'; }
# Every shroom window, whole, at its own frame; and the screen.
shot() { # <name>
  n=$((n + 1))
  local name i=0 frame
  name=$(printf '%02d-%s' "$n" "$1")
  # The pointer out of the picture, so no tooltip covers what is said.
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
# The mushroom clicked, as a person clicks it; again if no window came up (a popover asked
# for while the app is still busy with what came before can miss its first click).
popover() {
  local try
  local xy
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
quit_app() {
  pkill -f "$APP/Contents/MacOS/" 2>/dev/null
  for _ in $(seq 1 20); do pgrep -f "$APP/Contents/MacOS/" >/dev/null || return 0; sleep 1; done
  pkill -9 -f "$APP/Contents/MacOS/" 2>/dev/null
  sleep 1
}
version() { plutil -extract CFBundleShortVersionString raw -o - "$APP/Contents/Info.plist" 2>/dev/null; }
host() { node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).host' "$HOME_DIR/space.json" 2>/dev/null; }
ping_ai() { curl -fsS --max-time 15 "https://$(host)/_shroom/ping" 2>/dev/null | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{try{console.log(JSON.parse(s).ai===true?"on":"off")}catch{console.log("unread")}})'; }
wait_ai() { # <on|off> <seconds>
  local t=0 seen
  while [ "$t" -lt "$2" ]; do seen=$(ping_ai); [ "$seen" = "$1" ] && { echo "$seen"; return 0; }; sleep 5; t=$((t + 5)); done
  echo "${seen:-unread}"; return 1
}
once() { cat "$HOME_DIR/once.json" 2>/dev/null | tr -d '\n ' ; }
keep_log() { cp "$HOME_DIR/logs/run.log" "$OUT/run-$1.log" 2>/dev/null; }
finish_up() {
  pbcopy </dev/null
  keep_log last
  [ -n "${INVITE:-}" ] && rm -f "$INVITE"
  quit_app
}

# The wizard from the welcome to all-set, a person's clicks; `reconnect` when a space is
# already there for this account (leg B), else that is a leftover and the run stops.
wizard() { # <leg> <fresh|reconnect>
  local leg=$1 seen="" now t=0
  wait_for "Set up my space" 90 || stop "$leg: the welcome came up" "no Set up my space"
  shot "$leg-welcome"
  click "Set up my space" || stop "$leg: Set up my space" "not clickable"
  click "I have one" || stop "$leg: I have one" "not clickable"
  click "Paste your token" || stop "$leg: the token field" "not found"
  pbcopy <"${E2E_TOKEN_FILE:?}"
  osascript -e 'tell application "System Events" to keystroke "v" using command down'
  sleep 1
  pbcopy </dev/null
  say_ "pasted the run's token (the clipboard cleared)"
  while [ "$t" -lt 480 ]; do
    now=$(ax | awk -F'\t' '$1 != "W" && ($2 == "AXButton" || $2 == "AXHeading") { print $3 }' | sort | tr '\n' '|')
    if [ "$now" != "$seen" ]; then shot "$leg-wizard"; seen=$now; fi
    if has "You're all set"; then break; fi
    if has "Found your space"; then
      [ "$2" = reconnect ] || stop "$leg: a fresh space" "a space was already there: $(ax | grep -F 'Found your space' | cut -f3 | head -n 1)"
    fi
    case "|$now" in
      *"|Add credit card|"* | *"|Claim this name|"*) stop "$leg: the test account's gates" "the wizard asks: $now" ;;
    esac
    for b in "Use this account" "Allow recording" "Reopen shroom"; do
      case "|$now" in *"|$b|"*) click "$b" 2 && sleep 3 ;; esac
    done
    sleep 3
    t=$((t + 3))
  done
  has "You're all set" || stop "$leg: the wizard reached all-set" "it ended at: $now"
  shot "$leg-all-set"
  check "$leg: set up through the release's own wizard" 0 "$(version) on $(host)"
  click "Close" 5 || true
  sleep 2
}

# The early channel, as the installer's --early writes it, then the update a person takes:
# the popover's Update.
update_in_place() { # <leg>
  local leg=$1 t=0
  printf 'early\n' >"$HOME_DIR/channel"
  quit_app
  open "$APP"
  sleep 8
  while [ "$t" -lt 300 ]; do
    popover
    if has "Update"; then break; fi
    esc
    sleep 10
    t=$((t + 13))
  done
  has "Update" || stop "$leg: the update was offered" "no Update in the popover after $t s; $(grep -F 'updater' "$HOME_DIR/logs/run.log" | tail -n 2)"
  shot "$leg-update-offered"
  check "$leg: the popover offers the update on the early channel" 0 "$(ax | awk -F'\t' -v to="$TO" '$2 == "AXStaticText" && index($3, to) { print $3; exit }')"
  click "Update" || stop "$leg: Update" "not clickable"
  t=0
  while [ "$t" -lt 300 ]; do
    [ "$(version)" = "$TO" ] && pgrep -f "$APP/Contents/MacOS/shroom" >/dev/null && break
    # Still offered half a minute on: the click didn't land. Said, and clicked again.
    if [ "$t" -gt 0 ] && [ $((t % 30)) -eq 0 ] && [ "$(version)" = "$FROM" ] && has "Update"; then
      say_ "Update is still offered after $t s; clicking it again"
      shot "$leg-update-still-offered"
      click "Update" 2
    fi
    sleep 3
    t=$((t + 3))
  done
  [ "$(version)" = "$TO" ] || stop "$leg: updated in place to $TO" "still $(version) after $t s"
  codesign --verify --deep --strict "$APP" 2>/dev/null
  check "$leg: updated in place to $TO, relaunched, signature whole" $? "$(version) after $t s"
  sleep 12
}

# ── A ─────────────────────────────────────────────────────────────────────────────────
say_ "A: $FROM, set up, the AI never answered"
[ "$(version)" = "$FROM" ] || stop "A: the release installed is $FROM" "$(version)"
open "$APP"
wizard A fresh
check "A: the space's AI is off on $FROM" "$([ "$(wait_ai off 30)" = off ] && echo 0 || echo 1)" "ping: $(ping_ai)"
popover
shot "A-popover-$FROM"
has "Not now"
check "A: $FROM's popover shows the old offer, left unanswered" $? "$(ax | awk -F'\t' '$2 == "AXButton" { printf "%s · ", $3 }')"
esc
check "A: the offer was never answered on this Mac" "$(once | grep -q '"ai_offer_answered":false' && echo 0 || echo 1)" "$(once)"
keep_log A-before

update_in_place A
popover
wait_for "AI features are turned on." 30
shot "A-first-popover-after-update"
has "AI features are turned on."
check "A: the first popover after the update says AI features are turned on." $? "$(ax | awk -F'\t' '$2 == "AXStaticText" && /AI/ { printf "%s · ", $3 }')"
has "Turn off" && has "OK"
check "A: …with Turn off and OK" $? "$(ax | awk -F'\t' '$2 == "AXButton" { printf "%s · ", $3 }')"
ai=$(wait_ai on 120)
check "A: the page really has the AI (its ping)" "$([ "$ai" = on ] && echo 0 || echo 1)" "ping: $ai"
shot "A-popover-ai-on"
esc

# A take with words in it, said by the Mac's own voice (the Mac's sound is in every display
# take, B172); the space names it.
say_ "A: a take with words in it"
popover
click "Start recording" 10 || stop "A: Start recording" "not in the popover"
sleep 6
say -r 170 "Hi, this is a short test of shroom. I am recording my screen to show how the new version names a video by itself. First I open the settings, then I pick a display, and then I press stop. The link is copied, and the title comes from what I said. That is all for today, thank you for watching." &
shot "A-recording"
wait
sleep 3
# The mushroom mid-take pauses it; Finish & Share ends it.
popover
shot "A-paused"
click "Finish & Share" 10
check "A: a take recorded with the popover's Start recording and Finish & Share" $? "$(ax | awk -F'\t' '$2 == "AXButton" { printf "%s · ", $3 }')"
sleep 5
first=$("$SHROOM" list --json --limit 1 2>/dev/null)
id=$(printf '%s' "$first" | node -p 'JSON.parse(require("fs").readFileSync(0,"utf8")).videos?.[0]?.id ?? ""' 2>/dev/null)
title0=$(printf '%s' "$first" | node -p 'JSON.parse(require("fs").readFileSync(0,"utf8")).videos?.[0]?.title ?? ""' 2>/dev/null)
say_ "the take: ${id}, at first “${title0}”"
t=0
named=""
while [ "$t" -lt 600 ] && [ -n "$id" ]; do
  named=$(grep -F "naming" "$HOME_DIR/logs/run.log" | grep -F "named by" | tail -n 1)
  title=$("$SHROOM" info "$id" --json 2>/dev/null | node -p 'JSON.parse(require("fs").readFileSync(0,"utf8")).video?.title ?? ""' 2>/dev/null)
  [ -n "$named" ] && break
  sleep 10
  t=$((t + 10))
done
check "A: the new take got a name from the AI" "$([ -n "$named" ] && echo 0 || echo 1)" "“${title0}” → “${title}” after ${t} s; ${named}"
grep -F "naming" "$HOME_DIR/logs/run.log" | tail -n 12 >"$OUT/naming.log"
popover
click "Library" 10 && sleep 4 && shot "A-library-named" && esc
osascript -e 'tell application "System Events" to tell process "shroom" to keystroke "w" using command down' >/dev/null 2>&1
sleep 1

# Turn off, from the banner (still up: OK was never pressed).
popover
has "AI features are turned on."
check "A: the banner stays until OK or Turn off" $? "$(ax | awk -F'\t' '$2 == "AXStaticText" && /AI/ { printf "%s · ", $3 }')"
click "Turn off" || stop "A: Turn off" "not clickable"
wait_for "AI is off" 60
shot "A-turned-off-said"
has "AI is off — turn it on any time in Settings."
check "A: …and says so: AI is off — turn it on any time in Settings." $? "$(ax | awk -F'\t' '$2 == "AXStaticText" && /AI/ { printf "%s · ", $3 }')"
ai=$(wait_ai off 120)
check "A: Turn off took the AI off the page (its ping)" "$([ "$ai" = off ] && echo 0 || echo 1)" "ping: $ai"
esc
popover
shot "A-turned-off"
! has "AI features are turned on."
check "A: the banner is gone" $? ""
click "Settings" 10 || stop "A: Settings" "no gear in the popover"
sleep 3
click "Space" 10
sleep 3
shot "A-settings-space"
v=$(value_of "AI for new videos")
check "A: Settings · Space's AI switch matches (off)" "$([ "$v" = 0 ] || [ "$v" = false ] && echo 0 || echo 1)" "AI for new videos = ${v:-not found}"
check "A: the AI is settled on this Mac" "$(once | grep -q '"ai_offer_answered":true' && echo 0 || echo 1)" "$(once)"
osascript -e 'tell application "System Events" to tell process "shroom" to keystroke "w" using command down' >/dev/null 2>&1
keep_log A
quit_app

# ── B ─────────────────────────────────────────────────────────────────────────────────
say_ "B: another Mac of the owner's on $FROM, Not now on the old offer"
mv "$HOME_DIR" "$HOME/.shroom-legA"
rm -rf "$APP"
bash dev/e2e/install-release.sh "$FROM" | tail -n 3
[ "$(version)" = "$FROM" ] || stop "B: the release reinstalled" "$(version)"
wizard B reconnect
check "B: the space's AI is off" "$([ "$(wait_ai off 30)" = off ] && echo 0 || echo 1)" "ping: $(ping_ai)"
popover
shot "B-popover-$FROM"
click "Not now" 10 || stop "B: Not now on the old offer" "not there"
sleep 2
shot "B-not-now"
esc
check "B: Not now is remembered" "$(once | grep -q '"ai_offer_answered":true' && echo 0 || echo 1)" "$(once)"
keep_log B-before
update_in_place B
popover
sleep 5
shot "B-first-popover-after-update"
! has "AI features are turned on."
check "B: no AI banner after the update" $? "$(ax | awk -F'\t' '$2 == "AXStaticText" { printf "%s · ", $3 }' | cut -c1-240)"
esc
sleep 60
ai=$(ping_ai)
check "B: the AI stays off (its ping, a minute on)" "$([ "$ai" = off ] && echo 0 || echo 1)" "ping: $ai"
popover
shot "B-popover-a-minute-on"
! has "AI features are turned on."
check "B: still no banner" $? ""
click "Settings" 10 && sleep 3 && click "Space" 10 && sleep 3
shot "B-settings-space"
v=$(value_of "AI for new videos")
check "B: Settings · Space's AI switch is off" "$([ "$v" = 0 ] || [ "$v" = false ] && echo 0 || echo 1)" "AI for new videos = ${v:-not found}"

# ── C ─────────────────────────────────────────────────────────────────────────────────
say_ "C: the owner turns the AI on in Settings; a member joins"
toggle "AI for new videos"
ai=$(wait_ai on 120)
shot "C-owner-switch-on"
check "C: the owner's switch put the AI on the page" "$([ "$ai" = on ] && echo 0 || echo 1)" "ping: $ai"
osascript -e 'tell application "System Events" to tell process "shroom" to keystroke "w" using command down' >/dev/null 2>&1
INVITE=$(mktemp)
chmod 600 "$INVITE"
"$SHROOM" invite "Ann Lee" --json 2>/dev/null | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const j=JSON.parse(s);process.stdout.write(j.link ?? j.invite?.link ?? "")})' >"$INVITE"
[ -s "$INVITE" ] || stop "C: an invite from the owner's shroom command" "no link came back"
check "C: the owner invited a member" 0 "Ann Lee"
keep_log B
quit_app
mv "$HOME_DIR" "$HOME/.shroom-legB"
# The invite as a person gets it: the install line with the link (the early channel's).
curl -fsSL "https://shroom.am/install?ref=ci" | sh -s -- --early --join "$(cat "$INVITE")" 2>&1 | tr '\r' '\n' | grep -v '^#' | tail -n 3
rm -f "$INVITE"
INVITE=""
wait_for "Join" 90 || stop "C: the member's welcome" "no Join"
shot "C-member-welcome"
click "Join" || stop "C: Join" "not clickable"
t=0
while [ "$t" -lt 240 ]; do
  has "You're all set" && break
  click "Allow recording" 2 && sleep 3
  click "Reopen shroom" 1 && sleep 3
  sleep 3
  t=$((t + 6))
done
shot "C-member-all-set"
has "You're all set"
check "C: joined as a member" $? "$(node -p 'const s=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")); s.role + " of " + s.owner_name + "'"'"'s space"' "$HOME_DIR/space.json" 2>/dev/null)"
click "Close" 5
sleep 3
owner=$(node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).owner_name ?? ""' "$HOME_DIR/space.json" 2>/dev/null)
line="AI is on in ${owner}'s space."
t=0
while [ "$t" -lt 180 ]; do
  popover
  has "$line" && break
  esc
  sleep 15
  t=$((t + 18))
done
shot "C-member-popover"
has "$line"
check "C: the member's Mac says “${line}”" $? "$(ax | awk -F'\t' '$2 == "AXStaticText" && /AI/ { printf "%s · ", $3 }')"
has "OK" && ! has "Turn off"
check "C: …with OK only" $? "$(ax | awk -F'\t' '$2 == "AXButton" { printf "%s · ", $3 }')"
click "OK" 5
sleep 2
esc
popover
shot "C-member-after-ok"
! has "$line"
check "C: said once — gone after OK" $? ""
esc
keep_log C

finish_up
echo "checks: $(grep -c '^pass' "$CHECKS") passed, $(grep -c '^fail' "$CHECKS") failed"
exit "$failed"

# N43 on a real Mac: the permissions chapter's screen step shows one surface at a time.
# The dev app's screen grant is reset, so the first ask is a fresh install's: macOS's own
# alert must come up and System Settings must not open beside it; the app's log says the
# alert leads. Then the alert's own Open System Settings opens the pane. A second launch
# asks again: macOS stays silent, and shroom opens the pane itself.
set -u
. dev/mac/lib.sh
H="dev/mac/hand.sh"
S="$MAC_OUT/shots"
mkdir -p "$S"
home=one-surface
HOME_DIR="$(getconf DARWIN_USER_TEMP_DIR)$home"
shot() { screencapture -x "$S/$1.png" >/dev/null 2>&1; { echo "── $1"; windows; } >>"$MAC_OUT/windows.txt"; }
has() { windows | grep -q "^$1"; }
log_says() { grep -h "$1" "$HOME_DIR"/logs/*.log 2>/dev/null | tail -n 1; }
fail=0
verdict() { if [ "$1" = pass ]; then note pass "$2" "$3"; else note fail "$2" "$3"; fail=1; fi; }
settings_quit() { osascript -e 'tell application "System Settings" to quit' >/dev/null 2>&1; sleep 2; }

$H fake found >/dev/null
$H seed "$home" >"$MAC_OUT/seed.txt" 2>&1
# This Mac's chapter not done: the plan rests at `done`, row six current.
rm -f "$HOME_DIR/primed.json"
ls "$HOME_DIR" >>"$MAC_OUT/seed.txt"
# The grant the job wrote is kept and put back at the end: the scenarios after this one
# record.
row="access where service='kTCCServiceScreenCapture' and client='am.shroom.dev'"
q="select * from $row"
sqlite3 -cmd ".mode insert access" "$USER_DB" "$q" >"$MAC_OUT/grant-user.sql" 2>>"$MAC_OUT/seed.txt"
sudo sqlite3 -cmd ".mode insert access" "$SYS_DB" "$q" >"$MAC_OUT/grant-sys.sql" 2>>"$MAC_OUT/seed.txt"
restore() {
  $H quit >/dev/null 2>&1
  # Over the row the scenario's own ask left (switched off).
  sed 's/^INSERT INTO/INSERT OR REPLACE INTO/' "$MAC_OUT/grant-user.sql" | sqlite3 "$USER_DB" >>"$MAC_OUT/seed.txt" 2>&1
  sed 's/^INSERT INTO/INSERT OR REPLACE INTO/' "$MAC_OUT/grant-sys.sql" | sudo sqlite3 "$SYS_DB" >>"$MAC_OUT/seed.txt" 2>&1
  echo "restored, auth_value: $(sudo sqlite3 "$SYS_DB" "select auth_value from $row" | tr '\n' ' ')" >>"$MAC_OUT/seed.txt"
}
trap restore EXIT
tccutil reset ScreenCapture am.shroom.dev >>"$MAC_OUT/seed.txt" 2>&1
settings_quit

# 1. The first ask: a fresh Mac's.
$H start "$home" --live-shell
sleep 8
shot 1-done
$H key return   # Allow recording: the mic and camera are granted already, so the screen step
sleep 5
shot 2-first-ask
if has "universalAccessAuthWarn:" && ! has "System Settings:"; then
  verdict pass "first ask: macOS's alert alone" "the alert is up, System Settings isn't"
else
  verdict fail "first ask: macOS's alert alone" "$(windows | grep -E '^(universalAccess|System Settings)' | tr '\n' ' ')"
fi
said=$(log_says "shell asked for screen recording")
case $said in *"alert leads"*) verdict pass "first ask: the app heard the alert" "$said" ;; *) verdict fail "first ask: the app heard the alert" "${said:-nothing in the log}" ;; esac

# The alert's own button opens the pane.
osascript -e 'tell application "System Events" to tell process "universalAccessAuthWarn" to click button "Open System Settings" of window 1' >>"$MAC_OUT/seed.txt" 2>&1
sleep 5
shot 3-alerts-button
has "System Settings:" && verdict pass "the alert's button opens the pane" "System Settings is up" || verdict fail "the alert's button opens the pane" "no System Settings window"
settings_quit
$H quit >/dev/null

# 2. A later ask: macOS is silent, shroom opens the pane.
$H start "$home" --live-shell
sleep 8
shot 4-relaunched
$H key return
sleep 6
shot 5-second-ask
if has "System Settings:" && ! has "universalAccessAuthWarn:"; then
  verdict pass "later ask: the pane alone" "System Settings is up, no alert"
else
  verdict fail "later ask: the pane alone" "$(windows | grep -E '^(universalAccess|System Settings)' | tr '\n' ' ')"
fi
said=$(log_says "shell asked for screen recording")
case $said in *"opened the pane"*) verdict pass "later ask: the app opened the pane" "$said" ;; *) verdict fail "later ask: the app opened the pane" "${said:-nothing in the log}" ;; esac
settings_quit
$H quit >/dev/null
cp "$HOME_DIR"/logs/*.log "$MAC_OUT/" 2>/dev/null
exit $fail

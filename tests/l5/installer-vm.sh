#!/bin/sh
# L5: the one-line installer on a clean Mac (TESTING-PLAN §5). A throwaway clone of the
# `shroom-clean-tahoe` golden image holds releases as files (curl reads file:// URLs —
# no server, no host ports, no firewall prompts) and runs the real `curl … | sh` three
# times against the real app:
#
#   1. a fresh install — the app is in ~/Applications, carries no quarantine flag, keeps
#      its signature, and is running, opened as an install (its welcome window on
#      screen), with no Gatekeeper prompt;
#   2. a newer release while a take is open — refused, nothing touched, shroom still
#      running (the take is the app's own status seam with its own pid, flipped to
#      take_open: a clean VM has no screen-recording grant to open a real one);
#   3. the same release with no take open — the update lane: quit → swap → reopen, the
#      reopen silent (`--relaunched=install-update`, and no window on screen — N9).
#
# Throughout, `shroom://` (the join page's Open shroom, SM:1442, 1751) is asked of
# LaunchServices: a release build (tauri.release.conf.json's URL type) must map it to the
# installed ~/Applications/shroom.app and nothing else, after the install and after the
# swap; a dev build must never claim it (B42: a real invite can't launch one).
#
# A release build is also proven to record and to take a link (DECISIONS B82, B84):
#
#   4. its encoder — the VM has no ffmpeg of its own; the installed app says it found its
#      bundled one, and that encoder records a synthetic take through the app's own
#      recording code (tests/l5/encode-check: two takes, picture and sound, then the
#      single-file copy, poster and teaser) whose HLS decodes; the take is copied out and
#      this Mac's ffmpeg decodes it and holds it to the recipe's golden
#      (tests/l5/encode-signature.mjs);
#   5. a join link handed over by macOS — `open shroom://join/<host>/<code>` inside the VM,
#      to the running app and then to a quit one (a cold launch by the link), with a made-up
#      host that can't resolve (`.invalid`, RFC 6761) and a made-up code: the app says it
#      received it on its status seam (`setup` and `join.invite` = `join-space-missing`: a
#      space host that doesn't resolve is a space that's gone — the face a real link to a
#      deleted space gets) and in its run log. Only ever inside the VM: no link is opened here.
#
# The clone is deleted after.
#
# Usage: tests/l5/installer-vm.sh [path/to/app]   (default: ~/Applications/shroom-dev.app)
# A dev build is packaged as a stand-in (SHROOM_STAND_IN=1, B62); a release-config build
# (app/src-tauri/tauri.release.conf.json, tests/l5/build-release.sh) as a release.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
APP_SRC=${1:-$HOME/Applications/shroom-dev.app}
VM=shroom-l5-$$
WORK=$(mktemp -d)
OLD=0.0.1
NEW=0.0.2

cleanup() {
  tart stop "$VM" >/dev/null 2>&1 || true
  tart delete "$VM" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

# The app keeps its folder by its own name (B4): ~/.shroom, or ~/.shroom-dev for a dev build.
case "$(plutil -extract CFBundleIdentifier raw -o - "$APP_SRC/Contents/Info.plist")" in
  *.dev) SEAM='$HOME/.shroom-dev/status.json'; DEV=1 ;;
  *) SEAM='$HOME/.shroom/status.json'; DEV= ;;
esac
# Whether this build declares the shroom URL type.
if plutil -extract CFBundleURLTypes xml1 -o - "$APP_SRC/Contents/Info.plist" 2>/dev/null | grep -q '<string>shroom</string>'; then SCHEME=1; else SCHEME=; fi
# Two releases of the same app, exactly as real ones are packaged (tools/package-release.sh);
# a dev build only as a named stand-in (B62).
SHROOM_STAND_IN=$DEV "$ROOT/tools/package-release.sh" "$APP_SRC" "$OLD" "$WORK/$OLD" >/dev/null
SHROOM_STAND_IN=$DEV "$ROOT/tools/package-release.sh" "$APP_SRC" "$NEW" "$WORK/$NEW" >/dev/null

# A release build's encode proof runs a dev tool inside the VM (never shipped): built here.
if [ -n "$SCHEME" ]; then
  CARGO_TARGET_DIR="$ROOT/target/l5-encode-check" cargo build -q --release --manifest-path "$ROOT/tests/l5/encode-check/Cargo.toml"
  ENCODE_CHECK=$ROOT/target/l5-encode-check/release/encode-check
fi

tart clone shroom-clean-tahoe "$VM"
tart run "$VM" --no-graphics --no-audio >/dev/null 2>&1 &
i=0
until tart exec "$VM" true >/dev/null 2>&1; do
  i=$((i + 1)); [ "$i" -gt 90 ] && { echo "the VM never came up"; exit 1; }
  sleep 2
done

vm() { tart exec "$VM" sh -c "$1" </dev/null 2>/dev/null; }
put() { tart exec -i "$VM" sh -c "mkdir -p \"\$(dirname '$2')\" && cat > '$2'" < "$1"; }
publish() {
  put "$WORK/$1/manifest.txt" /tmp/rel/latest/download/manifest.txt
  put "$WORK/$1/shroom-$1-arm64.zip" "/tmp/rel/download/v$1/shroom-$1-arm64.zip"
}

# The real one-liner, its said-lines kept in $WORK/said. A run that hangs (a prompt
# nobody can answer) is stopped after two minutes and counts as a failure.
install() {
  tart exec "$VM" sh -c 'curl -fsSL file:///tmp/rel/install | SHROOM_RELEASES=file:///tmp/rel sh' </dev/null >"$WORK/said" 2>/dev/null &
  p=$!
  t=0
  while kill -0 "$p" 2>/dev/null; do
    t=$((t + 1))
    if [ "$t" -gt 120 ]; then kill "$p" 2>/dev/null; echo "(the installer hung)" >>"$WORK/said"; break; fi
    sleep 1
  done
  wait "$p" 2>/dev/null || true
  sed 's/^/     said: /' "$WORK/said"
}

fail=0
check() { if vm "$2" >/dev/null; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
host() { if sh -c "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
PROC='Applications/shroom.app/Contents/MacOS'
pid() { vm "pgrep -f '$PROC' | head -n 1" || true; }
inode() { vm 'stat -f %i "$HOME/Applications/shroom.app"' || true; }
# The app's ordinary windows on screen (layer 0 — not its menu-bar icon), read from the
# window server by owner pid: no Accessibility or Screen Recording grant needed.
windows() { vm "osascript -l JavaScript /tmp/windows.js $(pid)" || true; }
# Waits (up to 20 s) for the running app to report itself on its status seam.
seam_up() {
  n=0
  until [ -n "$(pid)" ] && [ "$(vm "plutil -extract pid raw -o - \"$SEAM\"")" = "$(pid)" ]; do
    n=$((n + 1)); [ "$n" -gt 20 ] && return 1
    sleep 1
  done
}

put "$ROOT/installer/install.sh" /tmp/rel/install
cat >"$WORK/windows.js" <<'JS'
ObjC.import('CoreGraphics');
function run(argv) {
  const pid = Number(argv[0]);
  const all = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0))) || [];
  return all.filter((w) => w.kCGWindowOwnerPID === pid && w.kCGWindowLayer === 0).length;
}
JS
put "$WORK/windows.js" /tmp/windows.js
# Every app LaunchServices would open a shroom:// link with, one path a line — asked, never
# opened (the link is a made-up one).
cat >"$WORK/handlers.js" <<'JS'
ObjC.import('AppKit');
function run() {
  const url = $.NSURL.URLWithString('shroom://join/v.example.workers.dev/AbCdEfGhIjKlMnOp');
  const apps = $.NSWorkspace.sharedWorkspace.URLsForApplicationsToOpenURL(url);
  const out = [];
  for (let i = 0; i < apps.count; i++) out.push(apps.objectAtIndex(i).path.js);
  return out.join('\n');
}
JS
put "$WORK/handlers.js" /tmp/handlers.js
handlers() { vm "osascript -l JavaScript /tmp/handlers.js" || true; }
scheme_check() {
  hs=$(handlers | sed 's|/$||' | tr '\n' ' ' | sed 's/ $//')
  if [ -n "$SCHEME" ]; then
    want="$(vm 'echo $HOME')/Applications/shroom.app"
    host "shroom:// opens the installed app, and only it ($1; handlers: ${hs:-none})" "[ '$hs' = '$want' ]"
  else
    host "a dev build never claims shroom:// ($1; handlers: ${hs:-none})" "[ -z '$hs' ]"
  fi
}
publish "$OLD"

echo "── 1. a fresh install ──"
install
sleep 5
check "placed in ~/Applications" 'test -d "$HOME/Applications/shroom.app"'
check "no quarantine flag" '! xattr -r "$HOME/Applications/shroom.app" | grep -q com.apple.quarantine'
check "signature intact after the trip" 'codesign --verify --deep "$HOME/Applications/shroom.app"'
check "running (opened with no prompt)" "pgrep -f '$PROC' >/dev/null"
check "opened as a fresh install" "ps -o args= -p \$(pgrep -f '$PROC' | head -n 1) | grep -qE -- '--relaunched=install( |\$)'"
check "the temporary download is gone" '! ls -d "$HOME"/.shroom-install.* 2>/dev/null'
check "nothing written outside home" '! test -e /Applications/shroom.app'
host "said: Installed — opening shroom now." "[ \"\$(tail -n 1 '$WORK/said')\" = 'Installed — opening shroom now.' ]"
if seam_up; then echo "ok   the app reports itself on its status seam"; else echo "FAIL the app reports itself on its status seam"; fail=1; fi
w1=$(windows)
host "a first launch shows its window (on screen: ${w1:-none})" "[ '${w1:-0}' -ge 1 ]"
scheme_check "after the install"

echo "── 2. a newer release, mid-take ──"
publish "$NEW"
before_pid=$(pid)
before_inode=$(inode)
vm "printf '{\"v\":1,\"pid\":%s,\"take_open\":true,\"setup\":\"welcome\"}' $before_pid > \"$SEAM\""
install
host "said: refused, nothing changed" "[ \"\$(cat '$WORK/said')\" = 'shroom is in the middle of a recording — finish that take, then run this again.
Nothing was changed.' ]"
host "shroom still running, never quit" "[ '$(pid)' = '$before_pid' ]"
host "the same copy in ~/Applications" "[ '$(inode)' = '$before_inode' ]"

echo "── 3. the same release, no take open: the update lane ──"
vm "printf '{\"v\":1,\"pid\":%s,\"take_open\":false,\"setup\":\"welcome\"}' $before_pid > \"$SEAM\""
install
sleep 5
after_pid=$(pid)
host "said: Updated shroom to $NEW — opening it now." "[ \"\$(tail -n 1 '$WORK/said')\" = 'Updated shroom to $NEW — opening it now.' ]"
host "the old shroom quit" "[ -n '$before_pid' ] && [ '$after_pid' != '$before_pid' ] && ! tart exec '$VM' kill -0 '$before_pid' 2>/dev/null"
host "a new copy swapped in" "[ -n '$(inode)' ] && [ '$(inode)' != '$before_inode' ]"
check "reopened" "pgrep -f '$PROC' >/dev/null"
check "reopened quietly (the update lane's launch)" "ps -o args= -p \$(pgrep -f '$PROC' | head -n 1) | grep -qE -- '--relaunched=install-update( |\$)'"
w3=$(windows)
host "the reopen opens no window — the menu bar only (on screen: ${w3:-none})" "[ '$w3' = 0 ]"
check "no quarantine flag" '! xattr -r "$HOME/Applications/shroom.app" | grep -q com.apple.quarantine'
check "signature intact" 'codesign --verify --deep "$HOME/Applications/shroom.app"'
check "the old copy and the download are gone" '! ls -d "$HOME"/.shroom-install.* 2>/dev/null'
check "one shroom in ~/Applications" '[ "$(ls -d "$HOME"/Applications/shroom*.app | wc -l | tr -d " ")" = 1 ]'
scheme_check "after the swap — no stale copy claims it"

if [ -z "$SCHEME" ]; then
  echo "── 4–5. (a dev build: no bundled encoder, no shroom:// — the release build proves them) ──"
  exit $fail
fi

echo "── 4. its own encoder, on a Mac with none ──"
LOG='$HOME/.shroom/logs/run.log'
check "the VM has no ffmpeg of its own" '! command -v ffmpeg >/dev/null && ! test -e /opt/homebrew/bin/ffmpeg && ! test -e /usr/local/bin/ffmpeg'
check "the installed app found its bundled encoder (run log: boot: encoder bundled)" "grep -q 'boot: encoder bundled' \"$LOG\""
check "and never any other" "! grep -E 'boot: encoder (override|homebrew|none)' \"$LOG\""
put "$ENCODE_CHECK" /tmp/l5/encode-check
vm 'chmod +x /tmp/l5/encode-check'
said=$(tart exec "$VM" sh -c '/tmp/l5/encode-check "$HOME/Applications/shroom.app" /tmp/l5/take' </dev/null 2>"$WORK/encode.err") && enc_ok=1 || enc_ok=
echo "     encode-check: ${said:-nothing} $(tr '\n' ' ' <"$WORK/encode.err")"
host "the bundled encoder recorded a synthetic take whose HLS decodes there" "[ -n '$enc_ok' ]"
mkdir -p "$WORK/take"
tart exec "$VM" sh -c 'tar -C /tmp/l5/take -cf - .' </dev/null 2>/dev/null | tar -C "$WORK/take" -xf - 2>/dev/null || true
if node "$ROOT/tests/l5/encode-signature.mjs" "$WORK/take" >"$WORK/sig" 2>&1; then sig_ok=1; else sig_ok=; fi
sed 's/^/     /' "$WORK/sig"
host "copied out, it decodes here and matches the recipe's golden" "[ -n '$sig_ok' ]"

echo "── 5. a join link, handed over by macOS ──"
LINK='shroom://join/l5-no-such-space.invalid/L5fakeInviteCode'
# Waits (up to 30 s) for the seam to say the invite window's state: the host doesn't
# resolve, so the space is gone (`member.rs` peek: no such host → SpaceMissing).
FACE=join-space-missing
joined() {
  n=0
  until [ "$(vm "plutil -extract setup raw -o - \"$SEAM\"")" = "$FACE" ] && [ "$(vm "plutil -extract join.invite raw -o - \"$SEAM\"")" = "$FACE" ]; do
    n=$((n + 1)); [ "$n" -gt 30 ] && return 1
    sleep 1
  done
}
running=$(pid)
vm "open '$LINK'"
if joined; then echo "ok   the running app received it: setup and join.invite say $FACE"; else echo "FAIL the running app received it (setup $(vm "plutil -extract setup raw -o - \"$SEAM\""), join.invite $(vm "plutil -extract join.invite raw -o - \"$SEAM\""))"; fail=1; fi
host "handed to the app already running (no second copy)" "[ '$(pid)' = '$running' ] && [ \"\$(tart exec '$VM' pgrep -f '$PROC' </dev/null | wc -l | tr -d ' ')\" = 1 ]"
check "its run log says the page's Open shroom brought it" "grep -q 'member: an invite for l5-no-such-space.invalid arrived by the page' \"$LOG\""
# Quit, forget the invite, and let the link open shroom cold.
vm "kill $running"
n=0
while tart exec "$VM" kill -0 "$running" 2>/dev/null; do n=$((n + 1)); [ "$n" -gt 20 ] && break; sleep 1; done
vm 'rm -f "$HOME/.shroom/invite" "$HOME/.shroom/status.json"'
vm "open '$LINK'"
if seam_up && joined; then echo "ok   a link opened shroom cold, and it received it: $FACE"; else echo "FAIL a link opened shroom cold, and it received it (setup $(vm "plutil -extract setup raw -o - \"$SEAM\""))"; fail=1; fi
host "a new shroom, launched by the link" "[ -n '$(pid)' ] && [ '$(pid)' != '$running' ]"
exit $fail

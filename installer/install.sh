#!/bin/sh
# shroom installer
#
# You're reading exactly what will run — this address serves one file,
# and this is it. Four things, in order:
#
#   1. Download the shroom app for your Mac.
#   2. Check it against its published fingerprint — if anything is off,
#      nothing gets installed.
#   3. Place it in ~/Applications — no admin password, nothing outside
#      your home folder.
#   4. Open it.
#
# Run it again anytime — that's also the update: same four steps, newer
# shroom. If shroom is in the middle of a recording, the script stops
# and changes nothing.
#
# Run it with --uninstall to remove shroom from this Mac.
#
# Trouble, questions: https://github.com/bagrat/shroom/issues/new
# No GitHub? hi@shroom.am works too.

# How it works, for the curious: releases are GitHub releases of the open-source
# repository. Each one carries manifest.txt — the version, the oldest macOS it runs
# on, and for each kind of Mac the file to download and its SHA-256 fingerprint.
# Everything runs inside main(), so a half-downloaded copy of this script does nothing.
# Placing shroom also links its `shroom` command into ~/.local/bin (see link_cli below).

set -u

# SHROOM_RELEASES exists for the installer's own tests; nobody else needs it.
RELEASES="${SHROOM_RELEASES:-https://github.com/bagrat/shroom/releases}"
APP="$HOME/Applications/shroom.app"
MIN_MACOS=14
# The release certificate's SHA-1 (tools/release-identity.sha1; its test holds them equal):
# a public release's shroom.app must be signed by it, as am.shroom (DECISIONS B125).
RELEASE_LEAF=8344cbe040722ea0a5632320c1d0d4348eaa3af3
# The `shroom` command (dev/proposals/cli-agent-skill.md; Bagrat, 2026-10-18: always
# installed): a link in your home folder to the one inside the app, so it updates with it.
CLI_DIR="$HOME/.local/bin"
CLI_LINK="$CLI_DIR/shroom"
CLI_MARK="# shroom's command line (shroom.am) — run shroom --help"

NET_DROP="Couldn't finish the download — check your connection and run the same command again."
CHECKSUM="The download didn't check out, so nothing was installed. Run it once more — if it repeats, tell us: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too."
MIDTAKE="shroom is in the middle of a recording — finish that take, then run this again."
UNCHANGED="Nothing was changed."
NO_BUILD="shroom doesn't run on this kind of Mac yet — Apple silicon only, for now."
DIDNT_QUIT="shroom didn't close — quit it from the menu bar, then run this again."
NO_ROOM="Couldn't place shroom on this Mac — check there's room for it, then run the same command again."
DIDNT_OPEN="shroom is in ~/Applications but didn't open — open it from there."
NOT_A_MAC="shroom runs on a Mac — run this in Terminal on yours."
NOT_ALL_THERE="This release isn't all there on our side, so nothing was installed. Tell us and we'll fix it: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too."
SIGNATURE="shroom's signature didn't verify, so nothing was installed. Run the same command in Terminal — if it repeats, tell us: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too."
REMOVED="Removed shroom from this Mac — the app, its settings and its permissions."
SPACE_STAYS="Every video and link is just as it was — only this Mac changed."
NOTHING_HERE="shroom isn't on this Mac — nothing to remove."
RESTRICTED="This looks like it's running somewhere restricted, so nothing was installed — run the same command in Terminal."

say() { printf '%s\n' "$*"; }
fail() { for line in "$@"; do say "$line"; done; exit 1; }

# The shroom already in ~/Applications names itself, and keeps its own folder:
# ~/.shroom (a developer build keeps ~/.shroom-dev).
bundle_id() { plutil -extract CFBundleIdentifier raw -o - "$APP/Contents/Info.plist" 2>/dev/null || echo am.shroom; }
state_dir() {
  case "$(bundle_id)" in
    *.dev) echo "$HOME/.shroom-dev" ;;
    *) echo "$HOME/.shroom" ;;
  esac
}

# Asks the app, through its status file, whether a recording is open. A file left by an
# app that's no longer running says nothing.
recording() {
  status="$(state_dir)/status.json"
  [ -f "$status" ] || return 1
  pid=$(plutil -extract pid raw -o - "$status" 2>/dev/null) || return 1
  kill -0 "$pid" 2>/dev/null || return 1
  [ "$(plutil -extract take_open raw -o - "$status" 2>/dev/null)" = "true" ]
}

running() {
  [ "$(osascript -e 'try' -e "return application id \"$(bundle_id)\" is running" -e 'on error' -e 'return false' -e 'end try' </dev/null 2>/dev/null)" = "true" ]
}

# Quits the running app the way you would, and waits for it to be gone — a new copy is
# never swapped in under a running one.
quit_app() {
  running || return 0
  was_running=1
  osascript -e "tell application id \"$(bundle_id)\" to quit" </dev/null >/dev/null 2>&1
  tries=0
  while running; do
    tries=$((tries + 1))
    [ "$tries" -gt 50 ] && return 1
    sleep 0.2
  done
}

# An update that couldn't finish puts shroom back as it was: open again if it was open.
reopen() { [ -z "$was_running" ] || open "$APP" --args --relaunched=install-update </dev/null >/dev/null 2>&1; }

# This kind of Mac. Apple silicon says so even to a Terminal running under Rosetta,
# where `uname -m` would answer x86_64.
mac_kind() {
  if [ "$(sysctl -n hw.optional.arm64 2>/dev/null)" = 1 ]; then echo arm64; else uname -m; fi
}

# One value from the release's manifest.txt (lines of key=value).
field() { sed -n "s/^$1=//p" "$work/manifest.txt" | head -n 1; }

# A download that failed: the release says something isn't there (ours to fix — tell
# us), or the connection didn't hold (run it again).
fetch_failed() { case "$1" in 4??) fail "$NOT_ALL_THERE" ;; *) fail "$NET_DROP" ;; esac; }

# A write this Mac refused, by what it said: somewhere that doesn't allow it (a sandbox
# says "Operation not permitted"), or no room. Any further lines follow the first.
cant_place() {
  case "$1" in *"Operation not permitted"*) shift; fail "$RESTRICTED" "$@" ;; esac
  shift; fail "$NO_ROOM" "$@"
}

# A whole signature, untouched since it was signed — and, from the public releases, the
# release's own signer: a release someone else published (with a fingerprint to match) is
# never installed, nor taken for the latest. (`-R` and its text apart: glued, `-R="=…"`,
# codesign reads a syntax error.)
signed() {
  codesign --verify --deep --strict "$1" </dev/null >/dev/null 2>&1 || return 1
  if [ "$RELEASES" = "https://github.com/bagrat/shroom/releases" ]; then
    codesign --verify --deep --strict -R "=identifier \"am.shroom\" and certificate leaf = H\"$RELEASE_LEAF\"" \
      "$1" </dev/null >/dev/null 2>&1 || return 1
  fi
}

# The release's shroom is already here (DECISIONS B156): the version it names, under the
# release's own name. Whether it's still as it was signed is signed's to say.
installed_version() { plutil -extract CFBundleShortVersionString raw -o - "$APP/Contents/Info.plist" 2>/dev/null; }
latest_here() { [ -d "$APP" ] && [ "$(bundle_id)" = am.shroom ] && [ "$(installed_version)" = "$1" ]; }

# Somewhere restricted that says so: a coding agent's sandbox can't write to
# ~/Applications, check a signature or open an app, so nothing is fetched there.
restricted() { [ "${SANDBOX_RUNTIME:-}" = 1 ] || [ -n "${CODEX_SANDBOX:-}" ]; }

# Somewhere restricted that doesn't say so shows itself on a signature every Mac can
# check: a sandbox that keeps codesign from the system's trust service fails Calculator's
# too. Then a signature that didn't verify says nothing about shroom's.
KNOWN_GOOD=/System/Applications/Calculator.app
unverified() {
  [ -d "$KNOWN_GOOD" ] && ! codesign --verify "$KNOWN_GOOD" </dev/null >/dev/null 2>&1 && fail "$RESTRICTED"
  fail "$SIGNATURE"
}

# The command, linked into ~/.local/bin — and, when that folder isn't on your PATH yet,
# one marked line in your shell's login profile that puts it there, for new Terminal
# windows. Never over a file of yours named shroom, and never a reason for the install
# to fail: an app without the command (an older release) links nothing.
link_cli() {
  target="$APP/Contents/MacOS/shroom-cli"
  [ -x "$target" ] || return 0
  if [ -e "$CLI_LINK" ] || [ -L "$CLI_LINK" ]; then
    [ "$(readlink "$CLI_LINK" 2>/dev/null)" = "$target" ] || return 0
  fi
  mkdir -p "$CLI_DIR" 2>/dev/null && ln -sfn "$target" "$CLI_LINK" 2>/dev/null || return 0
  case ":${PATH:-}:" in *":$CLI_DIR:"*) return 0 ;; esac
  case "${SHELL:-}" in
    */zsh) profile="$HOME/.zprofile" ;;
    */bash) profile="$HOME/.bash_profile" ;;
    *) return 0 ;;
  esac
  grep -qF "$CLI_MARK" "$profile" 2>/dev/null && return 0
  printf '\n%s\nexport PATH="$HOME/.local/bin:$PATH"\n' "$CLI_MARK" >>"$profile" 2>/dev/null || true
}

# Takes back exactly what link_cli made: the link (only if it's ours) and the marked line
# with the one after it; the rest of the profile is left as it was.
unlink_cli() {
  [ "$(readlink "$CLI_LINK" 2>/dev/null)" = "$APP/Contents/MacOS/shroom-cli" ] && rm -f "$CLI_LINK"
  for profile in "$HOME/.zprofile" "$HOME/.bash_profile"; do
    grep -qF "$CLI_MARK" "$profile" 2>/dev/null || continue
    awk -v m="$CLI_MARK" '$0 == m { skip = 1; next } skip { skip = 0; next } { print }' "$profile" >"$profile.shroom-tmp" &&
      cat "$profile.shroom-tmp" >"$profile"
    rm -f "$profile.shroom-tmp"
  done
}

# ── --uninstall (DECISIONS B135) ──────────────────────────────────────────────────────
# For a Mac whose shroom won't open. Only this Mac: the space, its videos and its links are
# never asked, and copies saved to Movies stay.

# The takes in shroom's folder that never reached the space: each one not uploaded, and not
# deleted elsewhere.
unsent() {
  n=0
  for t in "$(state_dir)"/takes/*/; do
    [ -d "$t" ] || continue
    [ -e "$t/uploaded" ] || [ -e "$t/tombstoned" ] || n=$((n + 1))
  done
  echo "$n"
}

count_word() {
  case "$1" in 1) echo One ;; 2) echo Two ;; 3) echo Three ;; 4) echo Four ;; 5) echo Five ;;
    6) echo Six ;; 7) echo Seven ;; 8) echo Eight ;; 9) echo Nine ;; *) echo "$1" ;; esac
}

uninstall() {
  id=$(bundle_id)
  dir=$(state_dir)
  [ -d "$APP" ] || [ -d "$dir" ] || { say "$NOTHING_HERE"; exit 0; }
  recording && fail "$MIDTAKE" "$UNCHANGED"
  n=$(unsent)
  if [ "$n" = 1 ]; then
    fail "One recording on this Mac never reached your space — open shroom to send it or save it to Movies, then run this again."
  elif [ "$n" -gt 1 ]; then
    fail "$(count_word "$n") recordings on this Mac never reached your space — open shroom to send them or save them to Movies, then run this again."
  fi
  quit_app || fail "$DIDNT_QUIT" "$UNCHANGED"
  # The permissions first, while the app is there: tccutil finds an app only through
  # LaunchServices, so it's registered (again) before the resets and forgotten after.
  ls_path="$PATH:/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support"
  [ -d "$APP" ] && PATH="$ls_path" lsregister -f "$APP" </dev/null >/dev/null 2>&1
  # Every grant it holds — screen, microphone, camera — in both of macOS's tables at once.
  tccutil reset All "$id" </dev/null >/dev/null 2>&1
  [ -d "$APP" ] && PATH="$ls_path" lsregister -u "$APP" </dev/null >/dev/null 2>&1
  defaults delete "$id" </dev/null >/dev/null 2>&1
  unlink_cli
  lib="$HOME/Library"
  rm -rf "$APP" "$dir" "$lib/WebKit/$id" "$lib/Caches/$id" "$lib/HTTPStorages/$id" \
    "$lib/Saved Application State/$id.savedState" "$lib/Preferences/$id.plist"
  say "$REMOVED"
  say "$SPACE_STAYS"
}

main() {
  join=""
  was_running=""
  if [ "${1:-}" = "--join" ]; then join="${2:-}"; fi

  # This Mac, before anything is fetched. A machine that isn't a Mac (a server, a Linux
  # box, a coding agent's sandbox) has no macOS version to read: it's told so, plainly.
  [ "$(uname -s 2>/dev/null)" = Darwin ] || fail "$NOT_A_MAC"
  if [ "${1:-}" = "--uninstall" ]; then uninstall; return; fi
  macos=$(sw_vers -productVersion 2>/dev/null)
  major=${macos%%.*}
  case "$major" in ""|*[!0-9]*) fail "$NOT_A_MAC" ;; esac
  [ "$major" -ge "$MIN_MACOS" ] || fail "shroom needs macOS $MIN_MACOS or newer — this Mac is on $macos."
  recording && fail "$MIDTAKE" "$UNCHANGED"
  restricted && fail "$RESTRICTED"

  # Which release, and which file is this Mac's. The temporary folder is in your home
  # folder and goes away when the script ends.
  work=$(mktemp -d "$HOME/.shroom-install.XXXXXX" 2>&1) || cant_place "$work"
  trap 'rm -rf "$work"' EXIT
  code=$(curl -fsL -w '%{http_code}' --retry 2 --connect-timeout 15 --max-time 60 \
    -o "$work/manifest.txt" "$RELEASES/latest/download/manifest.txt" </dev/null 2>/dev/null) || fetch_failed "$code"
  version=$(field version)
  floor=$(field macos)
  [ -n "$version" ] || fail "$NET_DROP"
  # The same shapes the app's own updater accepts: 1.2.3, a whole-number macOS, and for
  # each kind of Mac a plain (non-empty) file name and a 64-character fingerprint.
  case "$version" in .*|*.|*..*|*.*.*.*|*[!0-9.]*) fail "$NOT_ALL_THERE" ;; esac
  case "${floor:=$MIN_MACOS}" in *[!0-9]*) fail "$NOT_ALL_THERE" ;; esac
  [ "$major" -ge "$floor" ] || fail "shroom needs macOS $floor or newer — this Mac is on $macos."
  entry=$(field "$(mac_kind)")            # "<file> <sha256>"
  [ -n "$entry" ] || fail "$NO_BUILD"
  file=${entry%% *}
  sum=$(printf '%s' "${entry##* }" | tr 'A-F' 'a-f')
  case "$file" in ""|.*|*[!A-Za-z0-9._-]*) fail "$NOT_ALL_THERE" ;; esac
  case "$sum" in *[!0-9a-f]*) fail "$NOT_ALL_THERE" ;; esac
  [ "${#sum}" -eq 64 ] && [ "$entry" = "$file ${entry##* }" ] || fail "$NOT_ALL_THERE"

  # Already the latest, and it checks out: said before anything is downloaded. An open
  # shroom is only brought forward — nothing asks it to quit (B44) — unless an invite is
  # coming, which it reads as it opens. A copy that names this version but doesn't check
  # out (broken, or changed since it was signed) is downloaded again and swapped in, as an
  # update is.
  repaired=""
  if latest_here "$version"; then
    if signed "$APP"; then
      [ -z "$join" ] || quit_app || fail "$DIDNT_QUIT" "$UNCHANGED"
      say "shroom $version is up to date — opening it now."
      launch install-update
      return
    fi
    repaired=1
  fi

  # 1. Download, under curl's own progress bar. curl's own error lines are left out —
  #    the line after the bar says what happened. A connection that stalls (under
  #    1 KB/s for half a minute) counts as dropped.
  say "Getting shroom…"
  { curl -fL --progress-bar --retry 2 --connect-timeout 15 --speed-limit 1024 --speed-time 30 \
      -w '%{http_code}' -o "$work/$file" "$RELEASES/download/v$version/$file" </dev/null 2>&1 >"$work/code"
    echo $? >"$work/fetched"
  } | awk 'BEGIN { RS = "\r" } {
      gsub(/(curl: \(|Warning: )[^\n]*\n?/, ""); if (NR == 1) sub(/^\n+/, "")
      printf "%s%s", (NR > 1 ? "\r" : ""), $0; fflush()
    }' >&2
  [ "$(cat "$work/fetched")" = 0 ] || fetch_failed "$(cat "$work/code")"

  # 2. The fingerprint. A mismatch is never installed.
  got=$(shasum -a 256 "$work/$file" | cut -d ' ' -f 1)
  [ "$got" = "$sum" ] || fail "$CHECKSUM"
  err=$(ditto -x -k "$work/$file" "$work/new" </dev/null 2>&1) || cant_place "$err"
  [ -d "$work/new/shroom.app" ] || fail "$NOT_ALL_THERE"
  #    Its signature, whole, and its signer.
  signed "$work/new/shroom.app" || unverified
  say "verified ✓"

  # 3. Into ~/Applications. An update quits the running app first; if the swap can't
  #    finish, the old copy goes back and opens again.
  err=$(mkdir -p "$HOME/Applications" 2>&1) || cant_place "$err"
  if [ -d "$APP" ]; then
    recording && fail "$MIDTAKE" "$UNCHANGED"
    quit_app || fail "$DIDNT_QUIT" "$UNCHANGED"
    if ! err=$(mv "$APP" "$work/old.app" 2>&1); then
      reopen; cant_place "$err" "$UNCHANGED"
    fi
    if ! err=$(mv "$work/new/shroom.app" "$APP" 2>&1); then
      mv "$work/old.app" "$APP" 2>/dev/null || fail "$NO_ROOM"
      reopen; cant_place "$err" "$UNCHANGED"
    fi
    how=install-update
    if [ -n "$repaired" ]; then
      say "Repaired shroom $version — opening it now."
    else
      say "Updated shroom to $version — opening it now."
    fi
  else
    err=$(mv "$work/new/shroom.app" "$APP" 2>&1) || cant_place "$err"
    how=install
    say "Installed — opening shroom now."
  fi
  launch "$how"
}

# 4. Open it, saying how it was opened (an update reopens quietly, in the menu bar).
#    A join link goes to the app by file (owner-only), never on its command line,
#    where it would sit for as long as shroom runs.
launch() {
  how=$1
  case "$join" in
    *[!A-Za-z0-9._:/-]*) join="" ;;   # not a link's shape: open shroom plainly
  esac
  invite=""
  if [ -n "$join" ]; then
    dir=$(state_dir)
    ( umask 077 && mkdir -p "$dir" &&
      printf '{"v":1,"link":"%s","arrived":"%s"}\n' "$join" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$dir/invite.tmp" &&
      mv -f "$dir/invite.tmp" "$dir/invite" ) 2>/dev/null && invite=--invite
  fi
  link_cli
  # If macOS won't open it, the copy stays placed and the line says where it is — never
  # the system's own error text.
  open "$APP" --args "--relaunched=$how" $invite </dev/null >/dev/null 2>&1 || fail "$DIDNT_OPEN"
}

main "$@"

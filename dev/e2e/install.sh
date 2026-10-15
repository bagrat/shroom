#!/bin/bash
# The pre-merge e2e run's install (TESTING-PLAN D3; .github/workflows/e2e-full.yml): this
# branch's dev build, packaged as a release is (tools/package-release.sh's shape — a ditto
# zip of shroom.app and manifest.txt with its SHA-256), published to a folder, and put in
# place by the real one-line installer through file:// (as tests/l5/installer-vm.sh does):
# the fingerprint checked, the signature verified, ~/Applications/shroom.app placed and
# opened as an install. Then it is seen running, and quit — the run's wizard drive opens
# it again, as the installer would, with the drive's flags.
#
#   dev/e2e/install.sh <bundle> <releases dir>
#
# A hosted runner only: it installs into this user's ~/Applications.
set -euo pipefail
# A test machine only — the hosted Mac or a Tart clone, SIP off — never Bagrat's Mac
# (tests/test-machine.mjs, dev/proposals/heartbeat-in-vm.md §3).
{ [ "${GITHUB_ACTIONS:-}" = true ] && [ "${RUNNER_ENVIRONMENT:-}" = github-hosted ]; } || /usr/bin/csrutil status 2>/dev/null | grep -q 'status: disabled\.' || { echo "$(basename "$0") launches a dev build, and this isn't a test machine — run it on the hosted Mac or in a Tart clone (dev/proposals/heartbeat-in-vm.md §3)" >&2; exit 64; }
{ [ "${GITHUB_ACTIONS:-}" = true ] || [ -f /etc/shroom-l5-clone ]; } || { echo "install.sh: a hosted runner or a marked Tart clone only"; exit 64; }
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BUNDLE=$1
REL=$2
APP="$HOME/Applications/shroom.app"

version=$(node -p "require('$ROOT/app/src-tauri/tauri.conf.json').version")
file="shroom-$version-arm64.zip"
stage=$(mktemp -d)
mkdir -p "$REL/download/v$version" "$REL/latest/download"
ditto "$BUNDLE" "$stage/shroom.app"
ditto -c -k --keepParent "$stage/shroom.app" "$REL/download/v$version/$file"
sum=$(shasum -a 256 "$REL/download/v$version/$file" | cut -d ' ' -f 1)
printf 'version=%s\nmacos=14\narm64=%s %s\n' "$version" "$file" "$sum" >"$REL/latest/download/manifest.txt"
rm -rf "$stage"

echo "the installer, through file://:"
SHROOM_RELEASES="file://$REL" sh "$ROOT/installer/install.sh" </dev/null 2>&1 | tr '\r' '\n' | sed 's/^/  /'

[ -d "$APP" ] || { echo "not placed: $APP"; exit 1; }
codesign --verify --deep --strict "$APP" || { echo "the installed app doesn't verify"; exit 1; }
[ -z "$(xattr -p com.apple.quarantine "$APP" 2>/dev/null)" ] || { echo "the installed app carries a quarantine flag"; exit 1; }
cmp -s "$BUNDLE/Contents/MacOS/shroom" "$APP/Contents/MacOS/shroom" || { echo "the installed app isn't this build"; exit 1; }
for _ in $(seq 1 30); do
  pgrep -f "$APP/Contents/MacOS/" >/dev/null && break
  sleep 1
done
pgrep -f "$APP/Contents/MacOS/" >/dev/null || { echo "the installer's launch never ran"; exit 1; }
echo "installed $version at $APP, verified, opened"
sleep 3
pkill -f "$APP/Contents/MacOS/" || true
for _ in $(seq 1 10); do pgrep -f "$APP/Contents/MacOS/" >/dev/null || break; sleep 1; done

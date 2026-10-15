#!/bin/sh
# The update drive: a loopback release stand-in serving this build's version (a copy of this dev build,
# packaged by tools/package-release.sh), a throwaway update target, and the drive running
# as 0.0.1. The installed ~/Applications/shroom-dev.app is fingerprinted before and after.
# A drive that passes leaves nothing in the temp folder; one that fails keeps its work — the
# two copies, the stand-in's log, the drive's own home — and says where.
set -eu
# A test machine only — the hosted Mac or a Tart clone, SIP off — never Bagrat's Mac
# (tests/test-machine.mjs, dev/proposals/heartbeat-in-vm.md §3).
{ [ "${GITHUB_ACTIONS:-}" = true ] && [ "${RUNNER_ENVIRONMENT:-}" = github-hosted ]; } || /usr/bin/csrutil status 2>/dev/null | grep -q 'status: disabled\.' || { echo "$(basename "$0") launches a dev build, and this isn't a test machine — run it on the hosted Mac or in a Tart clone (dev/proposals/heartbeat-in-vm.md §3)" >&2; exit 64; }
repo=$(cd "$(dirname "$0")/../.." && pwd)
app=$repo/target/debug/bundle/macos/shroom-dev.app
w=$(mktemp -d -t shroom-r5-upd)
installed="$HOME/Applications/shroom-dev.app"
before=$(find "$installed" -type f -exec shasum {} + 2>/dev/null | shasum | cut -c1-16)
mkdir -p "$w/target" "$w/src"
ditto "$app" "$w/target/shroom-dev.app"
ditto "$app" "$w/src/shroom-dev.app"
v=$(node -p 'require(process.argv[1]).version' "$repo/app/src-tauri/tauri.conf.json")
SHROOM_STAND_IN=1 sh "$repo/tools/package-release.sh" "$w/src/shroom-dev.app" "$v" "$w/rel/download/v$v" >/dev/null
mkdir -p "$w/rel/latest/download"
cp "$w/rel/download/v$v/manifest.txt" "$w/rel/latest/download/manifest.txt"
node "$(dirname "$0")/release-standin.mjs" "$w/rel" >"$w/port" 2>"$w/http.log" &
srv=$!
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
# The throwaway copies were opened (LaunchServices registered them): unregister them as the
# drive ends, so no stale temp copy stays in this Mac's app registry. Then the work goes —
# once the drive has passed, never before.
passed=
trap 'kill $srv 2>/dev/null || true; for a in "$w/target/shroom-dev.app" "$w/src/shroom-dev.app"; do "$lsregister" -u "$a" 2>/dev/null || true; done; if [ -n "$passed" ]; then rm -rf "$w"; else echo "kept for a look: $w"; fi' EXIT
port=""
for i in $(seq 1 50); do
  port=$(head -n 1 "$w/port" 2>/dev/null || true)
  [ -n "$port" ] && curl -fsS "http://127.0.0.1:$port/latest/download/manifest.txt" >/dev/null 2>&1 && break
  sleep 0.2
done
echo "release stand-in on $port"
node "$(dirname "$0")/region5-drive.mjs" update --releases "http://127.0.0.1:$port" --update-target "$w/target/shroom-dev.app" --as-version 0.0.1 > "$w/out.json"
after=$(find "$installed" -type f -exec shasum {} + 2>/dev/null | shasum | cut -c1-16)
echo "installed app untouched: $([ "$before" = "$after" ] && echo yes || echo NO) ($before)"
[ "$before" = "$after" ] || exit 1
echo "target now: $(ls "$w/target")"
echo "http:"; cat "$w/http.log"
cat "$w/out.json"
passed=1

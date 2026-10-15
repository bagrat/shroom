#!/bin/sh
# The uninstall drive (DECISIONS B135): a throwaway copy of this dev build, provisioned
# against the fake in a throwaway home, walks Settings · General → Uninstall shroom… →
# Uninstall, and quits. Then: the copy is gone, its home is gone, macOS holds no grant for
# its bundle id, and the build it was copied from — and the installed dev app — are as they
# were. It resets the dev id's grants: on the hosted Mac, run it after every scenario that
# needs them.
set -eu
{ [ "${GITHUB_ACTIONS:-}" = true ] && [ "${RUNNER_ENVIRONMENT:-}" = github-hosted ]; } || /usr/bin/csrutil status 2>/dev/null | grep -q 'status: disabled\.' || { echo "$(basename "$0") launches a dev build, and this isn't a test machine — run it on the hosted Mac or in a Tart clone (dev/proposals/heartbeat-in-vm.md §3)" >&2; exit 64; }
repo=$(cd "$(dirname "$0")/../.." && pwd)
app=$repo/target/debug/bundle/macos/shroom-dev.app
id=$(plutil -extract CFBundleIdentifier raw -o - "$app/Contents/Info.plist")
# Under /tmp, not $TMPDIR (/var/folders): LaunchServices never resolves a bundle id to an
# app there, so tccutil couldn't find the copy (-10814) — a real install lives in
# ~/Applications. Both are throwaway places to the app (runtime::TEMP_ROOTS).
w=$(mktemp -d /tmp/shroom-r5-uninstall.XXXXXX)
copy="$w/shroom-dev.app"
fingerprint() { find "$1" -type f -exec shasum {} + 2>/dev/null | shasum | cut -c1-16; }
installed="$HOME/Applications/shroom-dev.app"
before_built=$(fingerprint "$app"); before_installed=$(fingerprint "$installed")
ditto "$app" "$copy"
grants() { { sqlite3 "$HOME/Library/Application Support/com.apple.TCC/TCC.db" "select service from access where client='$id'"; sqlite3 "/Library/Application Support/com.apple.TCC/TCC.db" "select service from access where client='$id'"; } 2>/dev/null | tr '\n' ' '; }
echo "grants before: $(grants)"
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
"$lsregister" -f "$copy"
# tccutil resolves the id through LaunchServices, which never answers with an app under
# /tmp or /var/folders (the throwaway places): on the hosted Mac every reset with only the
# copy registered found no app (-10814), from the app or a shell. The build it was copied
# from — the same id, at a workspace path — stands in for an install in ~/Applications.
"$lsregister" -f "$app"
echo "LaunchServices knows the copy: $("$lsregister" -dump 2>/dev/null | grep -c "path: *$(cd "$w" && pwd -P)/shroom-dev.app")"
# The drive's home is region5-drive.mjs's own; it checks the copy removed it.
SHROOM_DRIVE_APP="$copy" node "$(dirname "$0")/region5-drive.mjs" uninstall > "$w/out.json" || true
cat "$w/out.json"
# The helper finishes once the app has quit.
i=0; while [ -e "$copy" ] && [ "$i" -lt 100 ]; do i=$((i + 1)); sleep 0.1; done
bad=0
[ ! -e "$copy" ] && echo "the copy: gone" || { echo "NO: the copy is still there"; bad=1; }
after=$(grants)
[ -z "$after" ] && echo "grants after: none — every service not determined" || { echo "NO: grants left: $after"; bad=1; }
[ "$(fingerprint "$app")" = "$before_built" ] && echo "the build: untouched" || { echo "NO: the build changed"; bad=1; }
[ "$(fingerprint "$installed")" = "$before_installed" ] && echo "the installed dev app: untouched" || { echo "NO: the installed dev app changed"; bad=1; }
grep -q '"ok": true' "$w/out.json" && ! grep -q '"work"' "$w/out.json" && echo "the drive: passed, and its home is gone" || { echo "NO: the drive didn't pass"; bad=1; }
rm -rf "$w"
exit $bad

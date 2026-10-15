# The Terminal uninstall, for real (DECISIONS B135): the dev app placed where the installer
# puts shroom, in a home folder of its own, granted the screen, the microphone and the
# camera, with a folder and caches beside it; then `install.sh --uninstall`. Passes when it
# says the spec's two lines, nothing of the app is left on disk, and neither of macOS's
# grant tables holds a row for it. Copies in Movies stay.
set -u
LIB=$(cd "$(dirname "$0")/.." && pwd)/lib.sh
export OUT=$MAC_OUT
. "$LIB"
CLIENT=$(plutil -extract CFBundleIdentifier raw -o - "$MAC_APP/Contents/Info.plist")
CSREQ=$(requirement_hex "$(requirement_of "$MAC_APP")")
H="$RUNNER_TEMP/uninstall-home"
rm -rf "$H"; mkdir -p "$H/Applications" "$H/.shroom-dev/takes/20261014-120000-0001" "$H/Movies" "$H/Library/Preferences"
ditto "$MAC_APP" "$H/Applications/shroom.app"
printf '{}' >"$H/.shroom-dev/token.json"; : >"$H/.shroom-dev/takes/20261014-120000-0001/uploaded"
for d in WebKit Caches HTTPStorages; do mkdir -p "$H/Library/$d/$CLIENT"; done
mkdir -p "$H/Library/Saved Application State/$CLIENT.savedState"; : >"$H/Library/Preferences/$CLIENT.plist"
: >"$H/Movies/shroom recording.mp4"
write_grant "$USER_DB" kTCCServiceMicrophone no
write_grant "$USER_DB" kTCCServiceCamera no
write_grant "$USER_DB" kTCCServiceScreenCapture no
write_grant "$SYS_DB" kTCCServiceScreenCapture yes
rows() { { sqlite3 "$USER_DB" "select service from access where client='$CLIENT'"; sqlite3 "$SYS_DB" "select service from access where client='$CLIENT'"; } 2>/dev/null | tr '\n' ' '; }
echo "grants before: $(rows)"
# The screen video wants three seconds to judge; the uninstall itself takes under one.
sleep 3

bad=0
said=$(HOME="$H" sh installer/install.sh --uninstall 2>&1); code=$?
echo "exit $code; said:"; printf '%s\n' "$said"
want=$(node -e 'const m = require("./spec/machine.json"); console.log(m.states["uninstall-sh"].said.slice(0, 2).map((l) => l.text).join("\n"))')
[ "$code" = 0 ] && [ "$said" = "$want" ] || { echo "NO: not the spec's two lines"; bad=1; }
left=""
for p in Applications/shroom.app .shroom-dev "Library/WebKit/$CLIENT" "Library/Caches/$CLIENT" "Library/HTTPStorages/$CLIENT" "Library/Saved Application State/$CLIENT.savedState" "Library/Preferences/$CLIENT.plist"; do
  [ -e "$H/$p" ] && left="$left $p"
done
[ -z "$left" ] && echo "files: none left" || { echo "NO: left on disk:$left"; bad=1; }
[ -e "$H/Movies/shroom recording.mp4" ] && echo "Movies: the copy stays" || { echo "NO: the copy in Movies went"; bad=1; }
after=$(rows)
[ -z "$after" ] && echo "grants after: none — every service not determined" || { echo "NO: grants left: $after"; bad=1; }
exit $bad

# Can an uninstall clear shroom's permissions from the person's own account, with no admin?
# For each grant prepare() wrote (screen, microphone, camera): the rows before, then
# `tccutil reset <service> <bundle id>` run as the user (no sudo), then the rows after. Then
# the same once the app is gone from disk and from LaunchServices — the Terminal uninstall's
# case, when the app is deleted first. Passes when `reset All` on the installed, registered
# app left no row behind (the other cases are reported, not judged).
set -u
LIB=$(cd "$(dirname "$0")/.." && pwd)/lib.sh
export OUT=$MAC_OUT
. "$LIB"
CLIENT=$(plutil -extract CFBundleIdentifier raw -o - "$MAC_APP/Contents/Info.plist")
CSREQ=$(requirement_hex "$(requirement_of "$MAC_APP")")
LSREG=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
SERVICES="ScreenCapture Microphone Camera"
bad=0

rows() { # what both tables hold for the app
  printf 'user:   %s\n' "$(sqlite3 "$USER_DB" "select service||'='||auth_value from access where client='$CLIENT'" 2>&1 | tr '\n' ' ')"
  printf 'system: %s\n' "$(sqlite3 "$SYS_DB" "select service||'='||auth_value from access where client='$CLIENT'" 2>&1 | tr '\n' ' ')"
}
left() { # rows left for one service, across both tables
  { sqlite3 "$USER_DB" "select 1 from access where client='$CLIENT' and service='kTCCService$1'"
    sqlite3 "$SYS_DB" "select 1 from access where client='$CLIENT' and service='kTCCService$1'"; } 2>/dev/null | wc -l | tr -d ' '
}
grant_all() {
  write_grant "$USER_DB" kTCCServiceMicrophone no
  write_grant "$USER_DB" kTCCServiceCamera no
  write_grant "$USER_DB" kTCCServiceScreenCapture no
  write_grant "$SYS_DB" kTCCServiceScreenCapture yes
}
reset_each() { # <case>
  for s in $SERVICES; do
    said=$(tccutil reset "$s" "$CLIENT" 2>&1); code=$?
    n=$(left "$s")
    echo "$1 · $s: tccutil exit $code ($said); rows left $n"
    [ "$code" = 0 ] && [ "$n" = 0 ] || bad=1
  done
}

echo "client $CLIENT, as $(id -un) (uid $(id -u)), macOS $(sw_vers -productVersion)"
echo "-- 0. the app not yet known to LaunchServices (never opened)"
grant_all >/dev/null
reset_each unregistered; bad=0
"$LSREG" -f "$MAC_APP"
echo "-- 1. the app installed, registered with LaunchServices"
grant_all >/dev/null; rows
keep=$bad; reset_each installed; bad=$keep   # reported, not judged: ScreenCapture leaves the user table's row
rows

echo "-- 2. reset All, the app installed"
grant_all >/dev/null; rows
said=$(tccutil reset All "$CLIENT" 2>&1); code=$?; echo "reset All: exit $code ($said)"
rows
for s in $SERVICES; do [ "$code" = 0 ] && [ "$(left "$s")" = 0 ] || bad=1; done

echo "-- 3. the app deleted first, and unknown to LaunchServices"
grant_all >/dev/null; rows
gone="$RUNNER_TEMP/uninstall-probe-away"; mkdir -p "$gone"
mv "$MAC_APP" "$gone/" && "$LSREG" -u "$gone/$(basename "$MAC_APP")" >/dev/null 2>&1
mv "$gone/$(basename "$MAC_APP")" "$gone/hidden.noapp" 2>/dev/null
keep=$bad; reset_each deleted; bad=$keep   # reported, not judged
rows
mv "$gone/hidden.noapp" "$MAC_APP" && "$LSREG" -f "$MAC_APP" >/dev/null 2>&1


exit $bad

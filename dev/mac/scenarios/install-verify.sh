# The installer's signature checks, run against the public release on a Mac nobody set up:
# the latest release from GitHub, opened as the installer opens it, then each way of
# asking codesign — the installer's own lines word for word, the forms beside them, a
# requirement that must fail (so a pass means something), and a system app every Mac can
# verify. Then the installer itself, whole, into a throwaway home. Passes when the
# installer installs the release.
set -u
rel=https://github.com/bagrat/shroom/releases
work=$(mktemp -d)
curl -fsSL -o "$work/manifest.txt" "$rel/latest/download/manifest.txt" || { echo "no manifest"; exit 1; }
version=$(sed -n 's/^version=//p' "$work/manifest.txt")
file=$(sed -n 's/^arm64=//p' "$work/manifest.txt" | cut -d ' ' -f 1)
curl -fsSL -o "$work/$file" "$rel/download/v$version/$file" || { echo "no $file"; exit 1; }
ditto -x -k "$work/$file" "$work/new"
app="$work/new/shroom.app"
leaf=$(tr -d ' \n' <tools/release-identity.sha1)
req="identifier \"am.shroom\" and certificate leaf = H\"$leaf\""
bad="identifier \"am.shroom\" and certificate leaf = H\"0000000000000000000000000000000000000000\""
try() { # <label> <codesign args…>
  label=$1; shift
  said=$(codesign "$@" 2>&1 </dev/null); rc=$?
  echo "$label: exit $rc${said:+ — $(printf '%s' "$said" | tr '\n' ' ')}"
}
echo "release $version ($file)"
try "whole, strict"                     --verify --deep --strict "$app"
try "installer's pin: -R=\"=…\""        --verify --deep --strict -R="=$req" "$app"
try "short, one =: -R=\"…\""            --verify --deep --strict -R="$req" "$app"
try "short, apart: -R \"=…\""           --verify --deep --strict -R "=$req" "$app"
try "long: --test-requirement=\"=…\""   --verify --deep --strict --test-requirement="=$req" "$app"
try "wrong leaf, -R \"=…\" (must fail)" --verify --deep --strict -R "=$bad" "$app"
try "wrong leaf, -R=\"=…\" (must fail)" --verify --deep --strict -R="=$bad" "$app"
try "a system app"                      --verify /System/Applications/Calculator.app
codesign -d -r- "$app" 2>&1 | sed -n 's/^/  /p'
# The installer, whole, from this branch, into a home of its own.
home=$(mktemp -d)
HOME=$home sh installer/install.sh; rc=$?
echo "installer: exit $rc"
osascript -e 'tell application id "am.shroom" to quit' >/dev/null 2>&1
rm -rf "$work" "$home"
exit "$rc"

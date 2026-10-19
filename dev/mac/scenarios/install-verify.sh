# The installer's signature checks, run against the public release on a Mac nobody set up:
# the latest release from GitHub, opened as the installer opens it, then each way of
# asking codesign — the installer's own lines word for word, the forms beside them, a
# requirement that must fail (so a pass means something), and a system app every Mac can
# verify. Then the installer itself, whole, into a throwaway home. Passes when the
# installer installs the release and, when the release carries it, links its shroom command.
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
# The shroom command (spec N62, DECISIONS B217), when the release carries it: linked into
# ~/.local/bin, to the placed app's own; it answers with the release's version; and the
# login shell's profile puts that folder on PATH, once.
if [ "$(sed -n 's/^cli=//p' "$work/manifest.txt")" = shroom-cli ]; then
  link="$home/.local/bin/shroom"
  to=$(readlink "$link" 2>/dev/null)
  if [ "$to" = "$home/Applications/shroom.app/Contents/MacOS/shroom-cli" ]; then echo "command: $link → $to"; else echo "NO: the command isn't linked to the app's own (${to:-nothing at $link})"; rc=1; fi
  said=$(HOME=$home "$link" --version 2>&1)
  echo "command: $said"
  case $said in
    "shroom $version") ;;
    # 0.1.25's command named its crate's version; #73 fixed it from 0.1.26.
    "shroom 0.1.0") [ "$version" = 0.1.25 ] && echo "  (0.1.25's known slip: fixed from 0.1.26)" || { echo "NO: the command says ${said}, not shroom $version"; rc=1; } ;;
    *) echo "NO: the command says ${said:-nothing}, not shroom $version"; rc=1 ;;
  esac
  HOME=$home "$link" --help >/dev/null 2>&1 && echo "command: --help answers" || { echo "NO: the command's --help failed"; rc=1; }
  marks=$(cat "$home/.zprofile" "$home/.bash_profile" 2>/dev/null | grep -c "^# shroom's command line")
  echo "command: PATH line in the login profile ×$marks (shell ${SHELL:-none})"
  case ${SHELL:-} in */zsh | */bash) [ "$marks" = 1 ] || { echo "NO: the profile carries the PATH line $marks times"; rc=1; } ;; esac
fi
osascript -e 'tell application id "am.shroom" to quit' >/dev/null 2>&1
rm -rf "$work" "$home"
exit "$rc"

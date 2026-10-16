#!/bin/sh
# Prints the tap's Casks/shroom.rb: installer/homebrew/shroom.rb with a release's version
# and SHA-256, read from that release's manifest.txt — the same file, and the same checks
# on it, as installer/install.sh. With no argument it asks the public releases for the
# latest one; with a path, it reads that manifest instead.
#
#   installer/homebrew/render-cask.sh > Casks/shroom.rb
#   installer/homebrew/render-cask.sh dist/manifest.txt
set -eu
here=$(cd "$(dirname "$0")" && pwd)
if [ $# -gt 0 ]; then
  manifest=$(cat "$1")
else
  manifest=$(curl -fsSL --retry 2 --max-time 60 https://github.com/bagrat/shroom/releases/latest/download/manifest.txt)
fi
field() { printf '%s\n' "$manifest" | sed -n "s/^$1=//p" | head -n 1; }
version=$(field version)
entry=$(field arm64)
file=${entry%% *}
sum=$(printf '%s' "${entry##* }" | tr 'A-F' 'a-f')
case "$version" in ""|.*|*.|*..*|*.*.*.*|*[!0-9.]*) echo "render-cask: no version in the manifest" >&2; exit 1 ;; esac
[ "$file" = "shroom-$version-arm64.zip" ] || { echo "render-cask: the arm64 file isn't shroom-$version-arm64.zip" >&2; exit 1; }
case "$sum" in *[!0-9a-f]*) echo "render-cask: the fingerprint isn't hex" >&2; exit 1 ;; esac
[ "${#sum}" -eq 64 ] || { echo "render-cask: the fingerprint isn't 64 characters" >&2; exit 1; }
sed -e "s/^  version \"[0-9.]*\"$/  version \"$version\"/" \
    -e "s/^  sha256 \"[0-9a-f]*\"$/  sha256 \"$sum\"/" "$here/shroom.rb"

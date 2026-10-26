#!/bin/sh
# A page that's out keeps its fingerprint (DECISIONS B61, as amended by B248).
#
# worker/test/page-version.test.js holds the page to its VERSION's row in
# worker/page-version.json. While no build carrying that VERSION is published, a page change
# re-stamps the row: the number moves only when a build goes out. Once v<VERSION> is tagged
# (a release or an early build, published), the row is what installed apps and spaces already
# run, so it never changes again, and a page change needs the next version
# (tools/bump-version.sh). This is that second half: it asks the origin for the tag and,
# when it's there, compares the tag's row with the tree's.
#
#   tools/page-version-out.sh
#
# Needs git, sed and the network (CI's web lane runs it); it changes nothing in the tree.
set -eu
cd "$(git rev-parse --show-toplevel)"

version=$(sed -n 's/^export const VERSION = "\(.*\)";$/\1/p' worker/src/worker.js)
[ -n "$version" ] || { echo "page-version-out: no VERSION in worker/src/worker.js" >&2; exit 1; }
row() { sed -n "s/^  \"$(printf '%s' "$version" | sed 's/[.]/\\./g')\": \"\([0-9a-f]\{64\}\)\",\{0,1\}\$/\1/p"; }

if ! git ls-remote --exit-code --tags origin "refs/tags/v$version" >/dev/null; then
  echo "page-version-out: $version isn't out yet, so its row may still be re-stamped"
  exit 0
fi

git fetch --quiet --no-tags --depth=1 origin "refs/tags/v$version:refs/tags/v$version" 2>/dev/null ||
  git fetch --quiet --no-tags --depth=1 origin "+refs/tags/v$version:refs/tags/v$version"
out=$(git show "v$version:worker/page-version.json" | row)
now=$(row <worker/page-version.json)
if [ "$out" != "$now" ]; then
  echo "::error::$version is out (v$version), and its page was $out; the tree's row says $now." \
    "A page that's out never changes: move to the next version (tools/bump-version.sh) and keep $version's row as it was." >&2
  exit 1
fi
echo "page-version-out: $version is out, and its row is the one it shipped"

#!/bin/bash
# One signed release by its version, put in place by the real one-line installer: the
# release's own manifest and zip, mirrored into a folder as the installer reads its releases
# (latest/download/manifest.txt, download/v<version>/<file>), and the installer served on
# shroom.am run over it through file:// — the fingerprint checked, the signature verified,
# ~/Applications/shroom.app placed and opened, its `shroom` command linked. For a run that
# needs a Mac on a release older than the latest (the AI migration, e2e-migration.yml): the
# installer itself always gives the latest. The download goes straight to GitHub, so it's
# never counted as an install.
#
#   dev/e2e/install-release.sh <version>
#
# A hosted runner only: it installs into this user's ~/Applications.
set -euo pipefail
{ [ "${GITHUB_ACTIONS:-}" = true ] && [ "${RUNNER_ENVIRONMENT:-}" = github-hosted ]; } || { echo "install-release.sh: a hosted runner only" >&2; exit 64; }
version=${1:?a version, e.g. 0.1.27}
src="https://github.com/bagrat/shroom/releases/download/v$version"
rel=$(mktemp -d)
mkdir -p "$rel/latest/download" "$rel/download/v$version"
curl -fsSL --retry 3 --max-time 60 -o "$rel/latest/download/manifest.txt" "$src/manifest.txt"
[ "$(sed -n 's/^version=//p' "$rel/latest/download/manifest.txt")" = "$version" ] || { echo "v$version's manifest names another version" >&2; exit 1; }
file=$(sed -n 's/^arm64=\([^ ]*\) .*/\1/p' "$rel/latest/download/manifest.txt")
curl -fsSL --retry 3 --max-time 300 -o "$rel/download/v$version/$file" "$src/$file"
curl -fsSL --retry 3 --max-time 60 "https://shroom.am/install?ref=ci" | SHROOM_RELEASES="file://$rel" sh 2>&1 | tr '\r' '\n' | grep -v '^#'
rm -rf "$rel"

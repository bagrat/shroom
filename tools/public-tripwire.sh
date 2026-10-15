#!/bin/bash
# The public tripwire: this repo is public, and what's private stays out of it (CLAUDE.md,
# "Public and private"). Fails on, in the tracked tree and in what's given below:
#   - a claude.ai/code session link or a session id;
#   - a `Claude-Session:` trailer;
#   - an email address that isn't example.com, a role address on shroom.am, or a noreply;
#   - the name of a file that moved to the private repo.
#
#   tools/public-tripwire.sh                 the tree
#   tools/public-tripwire.sh <base> [<head>] the tree, and the commit messages in base..head
#   PR_TEXT="<title and body>"               also checks that text (CI passes the PR's)
#
# Third-party licence texts are left out: their authors' addresses are theirs to print.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

SESSION='claude\.ai/code/session_|session_0[0-9A-Za-z]{20,}'
TRAILER='Claude-Session:[[:space:]]*http'
MOVED='RETHINK|analytics\.md|after-v1-accounts-ai|demo-recording|demo-script'
EMAIL='[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}'
# Allowed: invented, role and noreply addresses, the URL-parsing test vector, and asset
# names like icon@2x.png that only look like one.
ALLOWED='@(([A-Za-z0-9-]+\.)*example\.(com|org|net)|shroom\.am|evil\.com|users\.noreply\.github\.com)$|^noreply@(anthropic|github)\.com$|\.(png|jpg|jpeg|svg|webp|gif)$'
SKIP=(':!tools/public-tripwire.sh' ':!THIRD-PARTY-NOTICES.md' ':!tools/licence-texts'
      ':!vendor' ':!worker/vendor' ':!Cargo.lock' ':!package-lock.json')

fail=0
report() { echo "public tripwire: $1"; cut -c1-200 | sed 's/^/  /'; fail=1; }

# The tree.
hits=$(git grep -nIE "$SESSION|$TRAILER|$MOVED" -- . "${SKIP[@]}")
[ -n "$hits" ] && report "session links, trailers or private file names in the tree" <<<"$hits"
hits=$(git grep -nIoE "$EMAIL" -- . "${SKIP[@]}" | while IFS= read -r l; do
  addr=${l#*:*:}; grep -qE "$ALLOWED" <<<"$addr" || echo "$l"; done)
[ -n "$hits" ] && report "personal email addresses in the tree" <<<"$hits"

# Text: commit messages and the PR's title and body.
check_text() { # <what> ; text on stdin
  local text; text=$(cat)
  local h; h=$(grep -nE "$SESSION|^Claude-Session:|$TRAILER|$MOVED" <<<"$text")
  [ -n "$h" ] && report "session links, trailers or private file names in $1" <<<"$h"
  h=$(grep -noE "$EMAIL" <<<"$text" | while IFS= read -r l; do
    grep -qE "$ALLOWED" <<<"${l#*:}" || echo "$l"; done)
  [ -n "$h" ] && report "personal email addresses in $1" <<<"$h"
}
if [ -n "${1:-}" ] && ! [[ "$1" =~ ^0+$ ]]; then
  git log --format='commit %h%n%B' "$1..${2:-HEAD}" | check_text "commit messages ($1..${2:-HEAD})"
fi
[ -n "${PR_TEXT:-}" ] && check_text "the PR's title or body" <<<"$PR_TEXT"

[ "$fail" = 0 ] && echo "public tripwire: clean"
exit "$fail"

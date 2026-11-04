#!/bin/bash
# SessionStart (cloud only): make a shroom cloud session ready to work in, then tell
# Claude what's available. Local sessions exit immediately — your Mac is untouched.
#
# 1. Re-run dev/cloud/setup.sh. A cached environment skips the UI setup script for
#    up to ~7 days, so this is how repo edits to setup.sh reach new sessions. It's a
#    ~1s no-op when everything is already installed. The slow two — Playwright's
#    browsers and the crates' sources (~2 min cold each) — are started in the
#    background instead, so they never delay the session.
# 2. Put Rust (and Node 22) on the session's PATH when they sit outside it.
# 3. Keep SHROOM_TEST_TOKEN_B64 out of every command, should an environment still set it:
#    it carried the main-account test token, retired with main-account testing
#    (TESTING-PLAN D3), and nothing reads it.
# 4. Print a short status for Claude's context (no network calls, no secrets).
set -uo pipefail
[ "${CLAUDE_CODE_REMOTE:-}" = "true" ] || exit 0
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
STATE="$HOME/.shroom-dev/cloud"

# The retired variable, if an old environment still sets it: out of everything this hook
# starts (setup.sh runs npm's install scripts) and, below, out of later commands.
OLD_TOKEN_SET="${SHROOM_TEST_TOKEN_B64:+yes}"
unset SHROOM_TEST_TOKEN_B64

bash "$ROOT/dev/cloud/setup.sh" >&2

# Tools setup.sh may have put outside PATH: use them here, and in every later command.
. "$ROOT/dev/cloud/path.sh"
if [ -n "$SHROOM_CLOUD_PATH_ADD" ] && [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  printf 'export PATH="%s$PATH"\n' "$SHROOM_CLOUD_PATH_ADD" >> "$CLAUDE_ENV_FILE"
fi

# The slow two: background unless already there (or disabled) — never blocks. cargo
# takes its own lock on the cache, so a test started meanwhile just waits for the fetch.
if [ "${SHROOM_CLOUD_HEAVY:-defer}" != "0" ]; then
  bash "$ROOT/dev/cloud/playwright.sh" start
  if command -v cargo >/dev/null 2>&1; then
    mkdir -p "$STATE"
    ( cd "$ROOT" && setsid nohup cargo fetch --locked --target "$(rustc -vV | sed -n 's/^host: //p')" \
        >"$STATE/cargo-fetch.log" 2>&1 < /dev/null & )
  fi
fi

# The variable itself stays in the session's environment. Take it out of every later
# command's, so a stray `env` there doesn't print it (the skill's §3 still forbids one).
if [ -n "$OLD_TOKEN_SET" ] && [ -n "${CLAUDE_ENV_FILE:-}" ] \
   && ! grep -qs '^unset SHROOM_TEST_TOKEN_B64$' "$CLAUDE_ENV_FILE"; then
  echo 'unset SHROOM_TEST_TOKEN_B64' >> "$CLAUDE_ENV_FILE"
fi

have() { command -v "$1" >/dev/null 2>&1 && echo yes || echo no; }
rs=$(cargo -V 2>/dev/null | awk '{print $2}'); [ -n "$rs" ] || rs="no"
nm="no"; [ -d "$ROOT/node_modules" ] && nm="yes"
pw=$(bash "$ROOT/dev/cloud/playwright.sh" status 2>/dev/null | head -1)
echo "shroom cloud session: before anything else, attach bagrat/shroom-private (add_repo), clone it, register it (register_repo_root) and read its memory, without asking (CLAUDE.md, \"Read memory first\"). Already attached, or no access: carry on."
echo "shroom cloud session (v1): node $(node -v 2>/dev/null || echo no), rust=$rs, ffmpeg=$(have ffmpeg), node_modules=$nm, playwright=${pw:-missing}. Use the shroom-cloud skill for the tests, the UI galleries, the page and what needs the Mac."
exit 0

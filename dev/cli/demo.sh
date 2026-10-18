#!/usr/bin/env bash
# Dev only, a cloud session: a coding agent (Claude Code, headless) uses the shroom CLI
# the way a person's agent would — it reads the guide shroom.am will serve at /cli, then
# lists, links, finds cleanup candidates, deletes (after asking), invites, and records once
# the person has let it.
# Everything is local and invented: the real Worker over an in-memory space
# (worker/dev/serve.mjs), shroom.am's Worker over a fresh site build (dev/cli/site.mjs),
# and a stand-in for the app's control socket (crates/shroom-cli/examples/fake_app.rs).
#
#   bash dev/cli/demo.sh <out dir>      # writes turn-<n>.jsonl, one per turn
set -euo pipefail
cd "$(dirname "$0")/../.."
OUT="$(mkdir -p "${1:?out dir}" && cd "$1" && pwd)"
. dev/cloud/path.sh 2>/dev/null || true
cargo build -q -p shroom-cli --bins --examples
# A short folder: a Unix socket's path has to fit in 104 bytes.
W="$(mktemp -d /tmp/shroom-demo.XXXX)"; mkdir -p "$W/bin" "$W/app" "$W/home" "$W/cwd"
cp target/debug/shroom-cli "$W/bin/shroom"
cp target/debug/examples/fake_app "$W/app/shroom"   # the seam wants a process named shroom

exec 3> >(sleep 3600 | node worker/dev/serve.mjs --port 8787 --space v.myshroom.workers.dev=Ann >"$W/serve.out" 2>&1)
exec 4> >(sleep 3600 | node dev/cli/site.mjs --port 8788 >"$W/site.out" 2>&1)
trap 'trap - EXIT; kill 0 2>/dev/null' EXIT   # the stand-ins go with the script
sleep 2
target/debug/examples/demo_space http://127.0.0.1:8787 "$W/home" >/dev/null
(sleep 3600 | "$W/app/shroom" http://127.0.0.1:8787 "$W/home" >"$W/app.out" 2>&1) &
sleep 1

export PATH="$W/bin:$PATH" SHROOM_HOME="$W/home" SHROOM_SPACE_BASE=http://127.0.0.1:8787
cd "$W/cwd"
TOOLS=(--allowedTools "Bash(shroom:*)" "Bash(curl:*)" "Bash(sleep:*)" --disallowedTools "Read" "Edit" "Write")
turn() { # turn <n> <prompt> [--continue]
  local n="$1" p="$2"; shift 2
  printf '%s\n' "$p" >"$OUT/prompt-$n.txt"
  claude -p "$p" "$@" "${TOOLS[@]}" --output-format stream-json --verbose >"$OUT/turn-$n.jsonl"
}
turn 1 "Use shroom on my Mac: read http://127.0.0.1:8788/cli and follow it. (That address stands in for https://shroom.am/cli.) Which of my videos are old enough to clean up? And send me the link to the login bug repro."
turn 2 "Delete the old demo for the prospect." --continue
turn 3 "Yes, delete it." --continue
turn 4 "Invite Sam Lee to my space." --continue
turn 5 "Now record my screen for about five seconds, then stop and give me the link." --continue
# The person turns on Settings → General's “Let it start and stop recordings” (off by
# default): the stand-in app reads it from prefs.json, as the app keeps it.
printf '{"agent_recording":true}\n' >"$W/home/prefs.json"
turn 6 "I turned it on. Try again, then show me my three newest videos." --continue
echo "done: $OUT"

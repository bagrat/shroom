#!/usr/bin/env bash
# Dev only, a cloud session: a team lead's coding agent (Claude Code, headless) does things
# many at once with the shroom CLI (Bagrat, 2026-10-18: batch commands) — it reads the guide
# shroom.am will serve at /cli, invites a team in one call and mails each person their own
# link with the person's mail tool (a stand-in, dev/cli/send-mail, that only writes an
# outbox), makes a digest of the week's links, and cleans up old videos after one yes.
# Everything is local and invented: the real Worker over an in-memory space
# (worker/dev/serve.mjs), shroom.am's Worker over a fresh site build (dev/cli/site.mjs),
# and a stand-in for the app's control socket (crates/shroom-cli/examples/fake_app.rs).
#
#   bash dev/cli/demo-batch.sh <out dir>   # turn-<n>.jsonl per turn, outbox.jsonl
set -euo pipefail
cd "$(dirname "$0")/../.."
OUT="$(mkdir -p "${1:?out dir}" && cd "$1" && pwd)"
. dev/cloud/path.sh 2>/dev/null || true
cargo build -q -p shroom-cli --bins --examples
# A short folder: a Unix socket's path has to fit in 104 bytes.
W="$(mktemp -d /tmp/shroom-demo.XXXX)"; mkdir -p "$W/bin" "$W/app" "$W/home" "$W/cwd"
cp target/debug/shroom "$W/bin/shroom"
cp dev/cli/send-mail "$W/bin/send-mail"
cp target/debug/examples/fake_app "$W/app/shroom"   # the seam wants a process named shroom

exec 3> >(sleep 3600 | node worker/dev/serve.mjs --port 8787 --space v.myshroom.workers.dev=Ann >"$W/serve.out" 2>&1)
exec 4> >(sleep 3600 | node dev/cli/site.mjs --port 8788 >"$W/site.out" 2>&1)
trap 'trap - EXIT; kill 0 2>/dev/null' EXIT   # the stand-ins go with the script
sleep 2
target/debug/examples/demo_space http://127.0.0.1:8787 "$W/home" >/dev/null
(sleep 3600 | "$W/app/shroom" http://127.0.0.1:8787 "$W/home" >"$W/app.out" 2>&1) &
sleep 1

export PATH="$W/bin:$PATH" SHROOM_HOME="$W/home" SHROOM_SPACE_BASE=http://127.0.0.1:8787 OUTBOX="$OUT/outbox.jsonl"
cd "$W/cwd"
TOOLS=(--allowedTools "Bash(shroom:*)" "Bash(curl:*)" "Bash(send-mail:*)" --disallowedTools "Read" "Edit" "Write")
turn() { # turn <n> <prompt> [--continue]
  local n="$1" p="$2"; shift 2
  printf '%s\n' "$p" >"$OUT/prompt-$n.txt"
  claude -p "$p" "$@" "${TOOLS[@]}" --output-format stream-json --verbose >"$OUT/turn-$n.jsonl"
}
turn 1 "Use shroom on my Mac: read http://127.0.0.1:8788/cli and follow it. (That address stands in for https://shroom.am/cli.) Send shroom invitations to the engineering team: Priya Shah (priya@example.com), Tom Becker (tom@example.com), Lena Ortiz (lena@example.com) and Mher (mher@example.com). Email each of them their own invite with my mail tool, send-mail (send-mail --to <address> --subject <subject> --body <text>)."
turn 2 "Yes, go ahead." --continue
turn 3 "Now make me a short digest of what we recorded in the last 10 days, with links." --continue
turn 4 "And clean up: delete everything older than 200 days." --continue
turn 5 "Yes, delete them." --continue
echo "done: $OUT"

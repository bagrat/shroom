#!/bin/bash
# The Mac in the loop, from a session — a cloud session, or a Mac: ask a GitHub-hosted Mac
# to try this branch's dev app, and bring back what it saw.
#
#   loop.sh run [--hold <minutes>] [--by-push] <scenario…>
#       Starts .github/workflows/mac.yml for the branch you are on (push it first), with the
#       scenarios named (dev/mac/scenarios/<name>.sh), waits, and fetches the evidence:
#       summary.md, and per scenario video.mp4, output.log and windows.log. With --hold the
#       Mac is kept for that long after the scenarios, and this returns as soon as its SSH
#       door is open. --by-push asks through a commit to dev/mac/request instead of
#       starting the workflow — for a session that may not start one. On a session's own
#       branch only: the ask is a commit and a push, so on main it is refused.
#   loop.sh ssh <command…>     a command on the held Mac, from the repo's root there
#   loop.sh shot               the held Mac's screen now, as a local PNG — prints its path
#   loop.sh get <path>         a file from the held Mac — prints the local copy's path
#   loop.sh release            lets the held Mac go, waits, and fetches the evidence
#   loop.sh fetch [<run>]      the evidence of a finished run (the latest by default)
#
# On the held Mac, dev/mac/mac.sh's `run <scenario…>` runs scenarios again; cliclick,
# osascript and `open` drive it; `screencapture` is what `shot` uses.
#
# Needs gh (signed in — a cloud session's is), git, ssh and node ≥ 22. State and evidence
# go under $MAC_LOOP_DIR (default: ${TMPDIR:-/tmp}/shroom-mac); `latest` there is the run
# the other commands mean. Nothing here touches this machine beyond that folder — and,
# with --by-push, one commit on the branch.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
STATE=${MAC_LOOP_DIR:-${TMPDIR:-/tmp}/shroom-mac}
mkdir -p "$STATE"
# owner/name from the remote's address, whatever carries it (https, ssh, a session's proxy).
REPO=$(git -C "$ROOT" remote get-url origin | sed -E 's#\.git$##; s#^.*[:/]([^/:]+/[^/:]+)$#\1#')

fail() { echo "loop: $*" >&2; exit 1; }
latest() { [ -e "$STATE/latest/run" ] || fail "no run yet — loop.sh run …"; (cd "$STATE/latest" && pwd -P); }
field() { node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))[process.argv[2]]' "$1/connect/connect.json" "$2"; }
attempt() { gh api "repos/$REPO/actions/runs/$1" --jq .run_attempt; }

fetch() { # <dir> <run>
  local dir=$1 run=$2
  gh run watch "$run" --repo "$REPO" --interval 15 >/dev/null 2>&1 || true
  rm -rf "$dir/evidence"
  gh run download "$run" --repo "$REPO" --name "mac-evidence-$run-$(attempt "$run")" --dir "$dir/evidence" \
    || fail "run $run left no evidence — https://github.com/$REPO/actions/runs/$run"
  # The job is green whenever the Mac ran what was asked; the table is the verdict.
  echo "run $run (the job: $(gh api "repos/$REPO/actions/runs/$run" --jq .conclusion), which is not the verdict) — https://github.com/$REPO/actions/runs/$run"
  cat "$dir/evidence/summary.md" 2>/dev/null || true
  echo
  if [ ! -s "$dir/evidence/results.tsv" ]; then
    echo "verdict: none — nothing reported; the job's log says why"
  elif grep -q '^fail' "$dir/evidence/results.tsv"; then
    echo "verdict: NO — $(awk -F'\t' '$1 == "fail" { printf "%s%s", sep, $2; sep = ", " }' "$dir/evidence/results.tsv")"
  else
    echo "verdict: all passed"
  fi
  echo
  echo "evidence: $dir/evidence"
  find "$dir/evidence" -name '*.mp4' | sort
}

# Waits for the held Mac's door, then writes what ssh needs to walk through it.
connect() { # <dir> <run>
  local dir=$1 run=$2 tries=0 name
  name="mac-connect-$run-$(attempt "$run")"
  until gh run download "$run" --repo "$REPO" --name "$name" --dir "$dir/connect" >/dev/null 2>&1; do
    tries=$((tries + 1))
    [ "$tries" -lt 120 ] || fail "run $run never opened its door"
    [ "$(gh api "repos/$REPO/actions/runs/$run" --jq .status)" != completed ] || fail "run $run ended without opening its door — loop.sh fetch"
    sleep 10
  done
  printf 'mac %s\n' "$(field "$dir" host_key)" >"$dir/known_hosts"
  cat >"$dir/ssh_config" <<EOF
Host mac
  HostName $(field "$dir" host)
  HostKeyAlias mac
  User $(field "$dir" user)
  IdentityFile $dir/key
  IdentitiesOnly yes
  UserKnownHostsFile $dir/known_hosts
  StrictHostKeyChecking yes
  ProxyCommand env NODE_USE_ENV_PROXY=1 node "$ROOT/dev/mac/ws-proxy.mjs" %h
  ServerAliveInterval 20
  LogLevel ERROR
EOF
}

on_mac() { # <dir> <command…>
  local dir=$1
  shift
  ssh -F "$dir/ssh_config" mac "cd '$(field "$dir" workspace)' && $*"
}

run() {
  local hold=0 by_push=no branch dir key="" since run=""
  while [ $# -gt 0 ]; do
    case $1 in
      --hold) hold=$2; shift 2 ;;
      --by-push) by_push=yes; shift ;;
      *) break ;;
    esac
  done
  [ $# -gt 0 ] || fail "which scenarios? (ls dev/mac/scenarios)"
  branch=$(git -C "$ROOT" rev-parse --abbrev-ref HEAD)
  [ "$by_push" = no ] || [ "$branch" != main ] || fail "--by-push commits and pushes: not on main — ask from a session's branch, or start it without --by-push"
  dir=$(mktemp -d "$STATE/run.XXXXXX")
  if [ "$hold" != 0 ]; then
    ssh-keygen -q -t ed25519 -N '' -C mac-loop -f "$dir/key"
    key=$(cat "$dir/key.pub")
  fi
  since=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  if [ "$by_push" = yes ]; then
    printf 'scenario=%s\nhold=%s\nssh_key=%s\nasked=%s\n' "$*" "$hold" "$key" "$since" >"$ROOT/dev/mac/request"
    git -C "$ROOT" add dev/mac/request
    # MAC_LOOP_TRAILER, when set, closes the commit's message (an author's attribution line).
    git -C "$ROOT" commit -q -m "mac: $*" ${MAC_LOOP_TRAILER:+-m "$MAC_LOOP_TRAILER"} -- dev/mac/request
    git -C "$ROOT" push -q origin "$branch"
  else
    [ "$(git -C "$ROOT" rev-parse HEAD)" = "$(git -C "$ROOT" rev-parse "origin/$branch" 2>/dev/null)" ] || fail "push $branch first: the Mac tries what is on origin"
    gh workflow run mac.yml --repo "$REPO" --ref "$branch" -f scenario="$*" -f hold="$hold" -f ssh_key="$key" >/dev/null
  fi
  for _ in $(seq 1 30); do
    # By the workflow's name, not its file: gh looks a file up on the default branch, where
    # a workflow that lives on one branch isn't.
    run=$(gh run list --repo "$REPO" --branch "$branch" --limit 10 --json databaseId,createdAt,workflowName \
      --jq "[.[] | select(.workflowName == \"Mac\" and .createdAt >= \"$since\")][0].databaseId // empty")
    [ -z "$run" ] || break
    sleep 4
  done
  [ -n "$run" ] || fail "no run started for $branch"
  echo "$run" >"$dir/run"
  ln -sfn "$dir" "$STATE/latest"
  echo "run $run — https://github.com/$REPO/actions/runs/$run"
  if [ "$hold" != 0 ]; then
    connect "$dir" "$run"
    on_mac "$dir" true || fail "the door is open but ssh didn't get through — $dir/ssh_config"
    echo "the Mac is held for $hold minutes: loop.sh ssh … | shot | get <path> | release"
  else
    fetch "$dir" "$run"
  fi
}

case ${1:-} in
  run) shift; run "$@" ;;
  ssh) shift; on_mac "$(latest)" "$@" ;;
  shot)
    dir=$(latest)
    there=$(on_mac "$dir" dev/mac/mac.sh shot)
    mkdir -p "$dir/shots"
    scp -q -F "$dir/ssh_config" "mac:$there" "$dir/shots/"
    echo "$dir/shots/$(basename "$there")"
    ;;
  get)
    dir=$(latest)
    mkdir -p "$dir/got"
    scp -q -F "$dir/ssh_config" "mac:$2" "$dir/got/"
    echo "$dir/got/$(basename "$2")"
    ;;
  release)
    dir=$(latest)
    on_mac "$dir" dev/mac/mac.sh release
    fetch "$dir" "$(cat "$dir/run")"
    ;;
  fetch)
    if [ -n "${2:-}" ]; then
      dir=$(mktemp -d "$STATE/run.XXXXXX")
      echo "$2" >"$dir/run"
      ln -sfn "$dir" "$STATE/latest"
    else
      dir=$(latest)
    fi
    fetch "$dir" "$(cat "$dir/run")"
    ;;
  *) sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'; exit 64 ;;
esac

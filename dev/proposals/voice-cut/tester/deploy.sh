#!/usr/bin/env bash
# Deploys the "shroom cut" tester to the Cloudflare TEST account as the Worker
# poc-voice-cut-tester, under a new throwaway key (or TESTER_KEY's, so a link already
# handed out keeps working), and prints the link to open. Needs SHROOM_E2E_TOKEN and
# SHROOM_E2E_ACCOUNT_ID in the environment (never on a command line). `deploy.sh down`
# deletes it.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
name=poc-voice-cut-tester
api="https://api.cloudflare.com/client/v4/accounts/$SHROOM_E2E_ACCOUNT_ID/workers"
auth="Authorization: Bearer $SHROOM_E2E_TOKEN"
ok() { python3 -c 'import json,sys; d=json.load(sys.stdin); sys.exit(0 if d.get("success") else (print(d.get("errors"), file=sys.stderr) or 1))'; }

if [ "${1:-}" = down ]; then
  curl -s -X DELETE -H "$auth" "$api/scripts/$name?force=true" | ok && echo "deleted $name"
  exit 0
fi

(cd "$here/wasm" && cargo build -q --release --target wasm32-unknown-unknown)
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
cp "$here/wasm/target/wasm32-unknown-unknown/release/shroom_cut_wasm.wasm" "$work/planner.wasm"
key=${TESTER_KEY:-$(python3 -c 'import secrets; print(secrets.token_urlsafe(18))')}
KEY="$key" python3 - "$work/meta.json" <<'PY'
import json, os, sys
json.dump({"main_module": "worker.js", "compatibility_date": "2026-09-01",
           "bindings": [{"type": "ai", "name": "AI"}, {"type": "secret_text", "name": "KEY", "text": os.environ["KEY"]}]},
          open(sys.argv[1], "w"))
PY
curl -s -X PUT -H "$auth" \
  -F "metadata=@$work/meta.json;type=application/json" \
  -F "worker.js=@$here/worker.js;type=application/javascript+module" \
  -F "page.html=@$here/page.html;type=text/plain" \
  -F "planner.wasm=@$work/planner.wasm;type=application/octet-stream" \
  "$api/scripts/$name" | ok
curl -s -X POST -H "$auth" -H 'content-type: application/json' -d '{"enabled":true}' "$api/scripts/$name/subdomain" | ok
sub=$(curl -s -H "$auth" "$api/subdomain" | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["subdomain"])')
echo "https://$name.$sub.workers.dev/#k=$key"

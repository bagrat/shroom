# scripts/runtime

Shared runtime plumbing for the commands and skills.

## `run-node`

A thin Bash dispatcher: `run-node <script.mjs> [args…]` runs a shroom Node script
with a Node that's new enough (`>= 22`, which gives a global `fetch`), regardless of
how Node is installed on the machine.

**Why it exists.** The `node` on the agent's non-interactive `PATH` is often the old
system Node (e.g. 16, no `fetch`), and a version selected via nvm isn't sourced into
that shell. Without this, every command/skill would have to carry a verbose
`export NVM_DIR=…; . nvm.sh && nvm use 22 && node …` prefix on *every* call. The
wrapper does that selection once and dispatches — the determinism boundary: exact,
repeatable plumbing lives in a script, not in a command's prose.

So commands/skills invoke `"${CLAUDE_PLUGIN_ROOT}/scripts/runtime/run-node" "…/foo.mjs"`
and grant `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/runtime/run-node:*)`.

**Selection order:** a Node `>= 22` already on `PATH` → nvm's `>= 22` → a Homebrew
`node@22` keg. If none is found it exits non-zero with a product-voiced hint pointing
at `/shroom:setup` (which installs one — see `scripts/setup/lib/node-detect.mjs` for
the install side). `SHROOM_RUN_NODE_KEGS` overrides the keg candidates (tests only).

Tests: `node scripts/runtime/test/run-node.test.mjs`.

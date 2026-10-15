# Sourced (never run) by setup.sh, playwright.sh and the SessionStart hook: puts the tools
# a fresh cloud shell may not have on PATH in front of it. Rust lands in ~/.cargo/bin when
# setup.sh installs it; the image keeps Node 22 in /opt/node22/bin when the default is older.
# Leaves what it prepended in SHROOM_CLOUD_PATH_ADD, so the hook can persist it for the
# session's own commands.
SHROOM_CLOUD_PATH_ADD=""
if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  SHROOM_CLOUD_PATH_ADD="$HOME/.cargo/bin:"
fi
_shroom_node_major=$(node -v 2>/dev/null | sed 's/^v\([0-9]*\).*/\1/')
if [ "${_shroom_node_major:-0}" -lt 22 ] 2>/dev/null && [ -x /opt/node22/bin/node ]; then
  SHROOM_CLOUD_PATH_ADD="/opt/node22/bin:$SHROOM_CLOUD_PATH_ADD"
fi
unset _shroom_node_major
PATH="$SHROOM_CLOUD_PATH_ADD$PATH"
export PATH

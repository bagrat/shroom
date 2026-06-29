#!/usr/bin/env node
// SessionStart hook — nudge the user to run /shroom:setup when shroom isn't
// configured yet. The plugin can be installed long before it's set up; this is
// the first-use detection pattern (there's no onInstall hook). It only ever
// *suggests*: a one-line note injected as session context, never an action
// (working agreement — never silently mutate, and here we don't even prompt a
// command, just remind).
//
// Behaviour:
//   - Silent once ~/.shroom/credentials.json exists (configured → no output).
//   - Only on a fresh session entry (source startup/resume); never on
//     clear/compact, so it can't re-nag mid-session.
//   - Fail-soft: ANY error exits 0 with no output, so a hook hiccup can never
//     block or slow a session (this runs at the start of EVERY session).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

function main() {
  // Read the hook payload from stdin (best-effort; we only need `source`).
  let source = 'startup';
  try {
    const raw = fs.readFileSync(0, 'utf8');
    if (raw.trim()) source = JSON.parse(raw).source || source;
  } catch {
    /* no / malformed stdin — treat as a fresh startup */
  }

  // Nudge only when a session genuinely begins. clear/compact happen *within* a
  // session the user is already in — re-nudging there would be noise.
  if (source !== 'startup' && source !== 'resume') return;

  const credsPath = path.join(os.homedir(), '.shroom', 'credentials.json');
  if (fs.existsSync(credsPath)) return; // already set up → stay quiet

  const note =
    'The shroom screen-recording plugin is installed but not set up yet ' +
    '(no ~/.shroom/credentials.json). At a natural moment — or if the user ' +
    'mentions recording, a demo, or shroom — let them know they can run ' +
    '/shroom:setup once (~5–10 min) to enable screen recording with permanent ' +
    'shareable links. Mention this at most once, briefly; do not nag.';

  process.stdout.write(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: 'SessionStart',
        additionalContext: note,
      },
    }),
  );
}

try {
  main();
} catch {
  /* fail-soft */
}
process.exit(0);

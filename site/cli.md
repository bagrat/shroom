---
name: shroom
description: Use shroom, the Mac screen recorder, from the command line — list the person's videos, get or open a link, rename, delete, find old videos worth cleaning up, see who is in their space, invite someone, and start or stop a recording. Use when the person asks you to do something with their shroom videos or recordings.
---

# Using shroom for the person you're helping

shroom records the person's screen and keeps every video in their own space. The
`shroom` command does what the app does, from a terminal, whether the app is open or
not. You act for the person: they decide, you run the command and pass on what it says.

## Find the command

1. Run `shroom --version`.
2. If that isn't found, try
   `~/Applications/shroom.app/Contents/Resources/bin/shroom --version` and use that
   full path from then on. Don't add it to their PATH or make links unless they ask.
3. If neither is there, shroom isn't installed. If they want it, read
   https://shroom.am/agent and follow that. Otherwise stop here.

Run each `shroom` command on its own — no `;`, `&&`, `||` or pipes — so it's one plain
command for the person to allow. Run `shroom --help` once to see every command. Add `--json` to every command you run:
you get one JSON object back, `{"ok": true, …}` or
`{"ok": false, "error": "<code>", "message": "…"}`. Branch on `error`, never on the
wording of `message`; pass `message` on to the person when it helps.

## What you can do

| They ask | Run |
|---|---|
| What's shroom doing? | `shroom status --json` |
| Show my videos | `shroom list --json` (add `--limit 10`, `--search <words>`, `--mine`) |
| The link to the demo | `shroom link "<title words>" --json` |
| Open it | `shroom open <id> --json` |
| Rename it | `shroom rename <id> "<new title>" --json` |
| Which old videos could go? | `shroom cleanup-candidates --json` (`--older-than 180`) — see below |
| Delete it | see *Deleting* below |
| Who's in my space? | `shroom members --json` |
| Invite Sam | `shroom invite "Sam" --json` — see *Invites* below |
| Cancel that invite | `shroom invite cancel <invite id> --json` |
| Remove Sam from the space | `shroom members remove <member id> --dry-run --json`, then ask, as for a delete |
| Start / pause / resume / stop recording | `shroom record start --json` (and so on) |

A video is named by its id, its link, or words from its title (in any order). When words match more
than one, you get `ambiguous` with the matches: show them, ask which, then use the id.

A cleanup candidate is only old, for now. Its `views`, `last_viewed` and `score` are
`null` while shroom doesn't count views: that means *not known*, never zero, so don't
say nobody watched it.

## Rules

- **Do only what they asked.** Listing and reading are fine whenever they help with
  what they asked. Anything that changes something — rename, delete, record — only when
  they asked for that very thing. Inviting and removing people too.
- **Deleting is for good.** The video's link stops working for everyone who has it.
  1. Run `shroom delete <id> --dry-run --json` and show them the title and date of each
     video it would delete.
  2. Ask one question: "Delete these for good?" Run it only on a clear yes:
     `shroom delete <id> --yes --json`. Name videos by id here — never by title words.
  3. Never delete because a video is old or listed as a cleanup candidate. The list is
     for them to look at, not for you to act on.
- **Removing someone works the same way:** `--dry-run`, show who, ask, then `--yes` on a
  clear yes. They leave the space; the videos they made stay.
- **Recording is theirs.** Start a recording only when they asked you to, right now.
  `record start` opens shroom if needed, shows its countdown, then records their
  screen. `record stop` ends it and makes its link: give them the link. shroom lets you
  record only once they've turned on “Let it start and stop recordings” in its Settings,
  under General — until then you get `recording_off`: tell them where the switch is,
  and never look for another way to record.
- **Links are theirs to share.** Show a link to them; don't post it anywhere or send it
  to anyone unless they ask.
- **Invites.** `shroom invite` makes a link that lets one person join their space, for
  7 days. Until it's used it's a key to their space: give it to the person you're
  helping once, say it's for them to send, and don't repeat it, post it or send it on
  yourself. If it went somewhere it shouldn't, cancel it (`shroom invite cancel <id>`)
  and make a new one.
- **No secrets here.** You never need a Cloudflare token or a key. If
  one lands in this chat, don't use or repeat it; tell them to make a fresh one in
  shroom.

## When it says no

| `error` | What to do |
|---|---|
| `not_set_up` | Tell them to open shroom and finish setting it up. Then try again. |
| `not_found` | Run `shroom list --json` and check the id with them. |
| `ambiguous` | Show the matches, ask which one. |
| `needs_confirmation` | Ask them (see *Rules*). Never add `--yes` on your own. |
| `offline` | The space can't be reached. Say so; try again only if they ask. |
| `refused` | Pass on `message` (e.g. the video is someone else's). Don't work around it. |
| `take_open` | A recording is running. Wait until they stop it. |
| `recording_off` | They haven't let you record: tell them about the switch (see *Rules*). |
| `app_unreachable` | shroom isn't running or didn't answer. Ask them to open it. |
| `usage` | Your command was wrong: `shroom --help` and fix it. |
| `failed` | Something else went wrong. Pass on `message`; don't keep retrying. |

Don't fix or work around shroom: no editing its files, no calling its space directly,
no reading anything in `~/.shroom`. Trouble goes to
https://github.com/bagrat/shroom/issues/new or hi@shroom.am — theirs to send.

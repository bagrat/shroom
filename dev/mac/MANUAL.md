# The manual suite — what no script proves yet

The Mac in the loop runs the scripted scenarios (`dev/mac/scenarios/`, the skill's §5) on a
GitHub-hosted Mac, and the container runs the rest (the skill's §1–§2). What is below is
what still needs hands: a session's on a held hosted Mac (§1), or Bagrat's on his own Mac
(§2). §3 is what is red on a hosted runner for a reason that isn't the app, so nobody
chases it twice. §4 is the page a run's results go to.

When a check here gets a script, it becomes a scenario and leaves this file. When a check is
run by hand, update its "last seen".

## 1. By hand on a held hosted Mac

Hold a Mac (`loop.sh run --hold 30 surfaces` — any scenario first, so the app is built,
granted and warm) and drive it with `dev/mac/hand.sh` over `loop.sh ssh` — the held Mac's
copy, from the branch it built. The dev app runs set up under a throwaway home against the
Cloudflare fake, with TextEdit as the other app:

```bash
L="bash dev/mac/loop.sh"
$L ssh 'dev/mac/hand.sh fake && dev/mac/hand.sh seed hand && dev/mac/hand.sh textedit && dev/mac/hand.sh start hand'
$L shot                                   # look before every click: the popover's rows move
$L ssh 'dev/mac/hand.sh click 775 12'     # the mushroom (dev/mac/hand.sh tray says where)
$L ssh 'dev/mac/hand.sh still c-count'    # a still mid-move; then $L get <the path it printed>
$L release                                # fetches the evidence and the hold's whole video
```

| Check | How | Passes when | Last seen |
|---|---|---|---|
| a. The mushroom over another app | TextEdit in front; click the mushroom; click a control once (Quality's 2K); Esc | the popover opens, the first click acts, Esc closes it, TextEdit stays in front (`hand.sh front` before and after) | ✅ 2026-10-13, fc8e3b9, run 38384769274 |
| b. What closes the popover | with it open: a click in TextEdit · a click on a Dock icon · a window picked in Mission Control (`key missioncontrol`) · Cmd-Tab · a click on bare desktop | the first and the last close it; the three switches don't (`hand.sh wins` shows the panel or not) | ✅ 2026-10-13, fc8e3b9 |
| c. Over a full-screen app | `fullscreen on`; move to the top, then click the mushroom; Cmd-Tab out and back; Start; a still at 1 s; Esc | the popover opens over it and survives both switches; the count plays over the app with no slide to a desktop; Esc cancels (`core cancel-countdown` in the home's `logs/run.log`) | ✅ 2026-10-13, fc8e3b9 — after Esc the popover came back in the top-right corner, not under the hidden mushroom |
| d. Typed fields | the Library from the popover; click a title, `type "renamed take"`, Enter; click the search, type a long string fast; `key selectall`, type | the whole rename stays; the whole search string stays; ⌘A then a key replaces it | ✅ 2026-10-13, fc8e3b9 |
| e. No Dock icon | `quit`, then `watch 8 &` and `start` together; the Library and Settings from the popover under `watch`; then `start` a fresh home | every look `UIElement`, the other app in front throughout; a fresh home turns `Foreground` with the setup window in front and a Dock icon | ✅ 2026-10-13, fc8e3b9 |
| f. Setup | a fresh home on `fake fresh`: Set up my space, I have one, `paste-token`, then the name step | the field shows dots while shroom holds the token; the name row claims nothing was taken | ◐ 2026-10-13: name row ✅; the dots went by unseen — the fake checks a token in under 0.2 s (§2) |
| g. A take by hand | Start, the count to its end, ~10 s, the mushroom mid-take, Stop; `get` the take's `preview.mp4` and pull frames | the file plays, and no frame holds the popover or the count | ✅ 2026-10-13, fc8e3b9 |

What a held Mac taught (2026-10-13):

- **Look before you click.** The popover's buttons move as its rows change: after a take a
  status line's row pushes the footer down 36 px while it shows (since DEVIATIONS #52 it goes
  when the upload finishes, so the footer moves back). A window's first paint takes 2–4 s.
- **A click on bare desktop** also sets off macOS 26's "click wallpaper to reveal desktop":
  every window sweeps aside. Click the wallpaper again to bring them back.
- **Full screen** goes through `hand.sh fullscreen` (System Events' `AXFullScreen`), never
  ⌃⌘F, which goes to whichever app is in front. In full screen the menu bar hides: move to
  the top edge, wait a second and a half, then click the mushroom.
- **The Dock** is read from LaunchServices (`hand.sh dock`, `watch`): `UIElement` is no
  Dock icon, `Foreground` is one. The window list can't tell.
- **The fake answers in milliseconds**, so anything shroom shows only while it waits on
  Cloudflare goes by too fast to see here.
- **macOS has no `timeout`**; the runner-side scripts use `lib.sh`'s `limit`.
- **Nothing real goes in.** The pasted token is random and the clipboard is cleared after;
  the runner's screen, its log and its evidence are public for three days.

## 2. Only on Bagrat's Mac

The hosted Mac can't settle these: it has no camera, its sound is a virtual device, it has
one 1024×768 display, its grants are written rather than asked for, and it reaches only the
fake. Each is in Bagrat's checkpoint already; this is the list of what it is still needed
for.

| Check | Why not on a hosted Mac | Where |
|---|---|---|
| The camera: the bubble, Camera as the source, `take-signals`' camera step | no camera | CHECKPOINT §7 |
| Sound by ear: the claps land with the hands | a virtual sound device, nobody to hear it | CHECKPOINT §7 (B59 e) |
| A second display, the notch, the lid | one display, no notch, no lid | CHECKPOINT §8 |
| Timing bars: `record`'s repeated and skipped frames, the A/V offset, `shroom-record`'s media tests | three loaded cores (§3) | a Mac decides |
| First-run permission prompts | the workflow writes the grants; nothing is ever asked | CHECKPOINT §5, TESTING-PLAN §8 |
| Real Cloudflare, and the token's dots while a real check runs (B104) | the drives run against the fake | CHECKPOINT §4–§6; the live tests (the skill's §3) |
| A real login launch: log out, log back in | the drive only stands in for it | CHECKPOINT §8 |
| Safari playback; how it feels at speed (the count, the popover) | no Safari drive here; the runner is slow | CHECKPOINT §6–§7 |
| The update over a real release; a Mac that has never seen shroom | stand-ins only | CHECKPOINT §9–§10 |

## 3. Red on a hosted runner, and why

| What | Red because | Status |
|---|---|---|
| `selftest` | ScreenCaptureKit sends a frame when the screen changes, and no Dock icon bounces since B113 | the bar changed in `fc8e3b9` |
| `record` ends with 2 | timing bars on three virtual cores (0.55 % repeated and skipped against 0.5 %) | by design: a Mac decides |
| `take-signals` ends with 1 | the camera step: the runner has the grant but no camera | issue #8 |
| `take-left-out` ends with 1 | a 1-px rim around the pattern window lights 1.1 % of the dark field at 1024×768; the take itself is clean | issue #4 |
| `library`, sometimes | the ⌘A check types a key a fixed 200 ms after ⌘A (2 of 5) | issue #3 |
| `onboarding`, once | the `found` path's app quit without a report (1 of 4), and the drive threw | issue #5 |
| CI's Tests job | `shroom-record`'s media tests time takes with fixed sleeps against tight bars | issue #7 |
| CI's Tests job, sometimes | `shroom-upload`'s tombstone test races its own churn thread | issue #6 |

## 4. The report page

After a run, its results go to one private page — every scenario's verdict, the checks that
said no and its video, then whatever was checked by hand — built from what `loop.sh`
brought back and published as an artifact, its media beside it:

```bash
node dev/mac/report.mjs --out <dir> --notes <notes.json> <run dir>…
```

A run dir is the folder `loop.sh run` printed (its `run` file and `evidence/`). The notes
are the session's own words — a note and issue links per scenario, sections for the
checks made by hand with their stills, and what is still owed; `report.mjs`'s header has
the shape. Publish `<dir>/index.html` with `media/` as its files. The first one was for
2026-10-13 (private).

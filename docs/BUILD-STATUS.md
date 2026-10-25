# shroom v1 — build status

> Regenerated at each milestone (TESTING-PLAN §6). **Milestone 5 — every region is built
> and the whole-spec audit is closed** (2026-10-03, at `47dfaa3`); the build's own in-frame
> list is empty, and what's left is Bagrat's (“Waiting for Bagrat”, below).
>
> **Where it stands (2026-10-14):** the code lives on `main` (`build/v1` merged and
> retired). **0.1.7 is released** (tag `v0.1.7`, 2026-10-14 — the first release cut by
> **Actions → Release → Run workflow** with Publish, B126); 0.1.5 before it. Since the
> milestone: the from-scratch run's fixes and the hosted Mac's pass (B104–B118, PR #11 —
> their batch logs are in [`history/build-log/2026-10-13.md`](../history/build-log/2026-10-13.md)),
> the popover opening at once (B119, PR #13), the Mac job's verdict table (PR #14), a
> camera for the hosted Mac (B120, B121, PR #12), the design canvas (B122, PR #16), the
> repo audit's licences, security fixes and cleanup (B123–B125, PR #15,
> [`history/audits/2026-10-13-repo-audit.md`](../history/audits/2026-10-13-repo-audit.md)), and the
> release as a workflow (B126, PR #17).
>
> Earlier milestones' records (3: the install line to a first recording; 4: the Library,
> Settings and updates) are in this file's git history, last at `32807c8`.


> **The record** — what milestone 5 meant, the test runs, the heartbeat, incidents, media
> truth, spec coverage, the scaffold flags: [`history/build-status/milestone-5.md`](../history/build-status/milestone-5.md).
> Earlier batches: [`history/build-log/`](../history/build-log).

## Next

1. **0.1.7 is out** (2026-10-14, run 38528266164): installed 0.1.5s offer it as an update.
   The heartbeat ran on Bagrat's Mac the same day, on `main` after the release — green,
   exit 0 (the Heartbeat table). Still open: the update from 0.1.5 on a real install is
   the first real test of B125's signer check (`grep 'updater not applied'` should find
   nothing). The next release: bump (`tools/bump-version.sh`), merge, then **Actions →
   Release → Run workflow** on `main` with Publish (RELEASING.md).
2. The checkpoint sitting (`CHECKPOINT.md`, items 20–25 and the by-hand checks of
   28–35, plus what came since — B119, DEVIATIONS #42, B125), then the ratifications
   (1–6).
3. “How to set up and use shroom” plays on the landing (17, B194). Intel is decided: Apple
   silicon only (7, B167). Identity and signing (8) are done: 0.1.5
   shipped signed.
4. Send feedback from the popover is built (N60, B193), the waving hand his pick.
5. **The editor is built** ([#61](https://github.com/bagrat/shroom/pull/61) *(the editor:
   trim, cut, undo, markers, and edits on the share page)*; N63 and N64, ratified 2026-10-18;
   B195–B199). Edit… in a live Library row opens it in a window of its own: trim, cut,
   a cut's edges dragged to adjust it (B195 (g), Bagrat's ask once he'd seen it), Undo and
   Redo, Revert. Done saves the edit into the record, and the link, unchanged,
   plays it at once. The Mac then makes the final video and asks “Delete the original?”:
   Yes, delete makes the edit the video now; otherwise the original goes two days on, by
   the first of the space's Macs to run then (B197). Markers dropped while paused, or with
   ⌥⌘M while recording, show as flags on its timeline (N64). What waits on Bagrat: items
   37–39.
6. Voice cuts (N65, provisional) are built: the owner page's strip, its press, the
   cuts' word in the status and the page's one finishing line, which stays while a take is heard (B200), and the Mac's lane that plans a take once the space has
   heard it and lands its cuts in the record (B201), how far back a line said again in other
   words reaches weighed by Clef on the space's own account (B202); the cue is “shroom cut”, his pick (B203). They sit on the editor's edit list (N63) and come alive with N66's
   transcript (the AI build); the editor's final copy lets Download follow them. Built ahead of the release call, which is Bagrat's.
   Nothing else in-frame is queued: the build resumes on his answers.
5. **Names, chapters and captions** (N66, N67, B205–B209) are built on their branch, off
   by default, waiting for his review and the release call: the space names its videos,
   places chapters in the seek bar and writes captions through the owner's own Cloudflare
   account; a switch in Settings · Space on the setup Mac and a one-time card in its
   Library turn it on. Checked live on the free-plan test account (B206).
5. **The `shroom` command for coding agents is built and ratified** (N62, B215–B219;
   [#58](https://github.com/bagrat/shroom/pull/58) *(the shroom command for coding agents)*):
   the command inside the app and on PATH, many at once, the app's control socket behind
   General's switch — its refusal carrying the link that opens Settings at the switch — the
   start without the count and the agent label, and the guide at shroom.am/cli. Shipped in
   0.1.25 (2026-10-19). Then `shroom edit` (N72, B225, item 42), ratified 2026-10-19: what a video's edit
   is, one change a call saved as the editor's Done saves, and the app making its final
   video.


## Waiting for Bagrat

Gathered from DECISIONS, DEVIATIONS, WORKER-INTERFACE §9 and ARCHITECTURE §16. None blocks
the build; the ones marked **release** block shipping. The numbers are the ones
`CHECKPOINT.md` points at (1–6, 20–25), so new items are appended, not slotted in.

**Ratify**
1. **DEVIATIONS 1–32 and 36** (#7 not adopted). New since milestone 4: 24 (the installer's
   named failures), 25 (`wizard-unexpected`), 26 (a verify-rejected token reads garbled),
   27 (exits wait for a copy to Movies), 28 (Members writes that fail say so), 29 (the
   landing's demo held back; closed by B190), 30 (the needs-update line in the invite window), 31 (the
   join page's cookie faces best-effort), 32 (the repair mood's foreign-Worker rest), 36
   ((a) Return copies the link on `stopped-offline`; (b) the tray's face and the bubble
   during `stopping` — the spec is silent on both).
2. **N29–N42**, built as provisional — from the rendered galleries.
3. **The copy written during the build** (DECISIONS' last section), now with the
   landing's system line, title, description and footer, and FFmpeg's notice in the
   release bundle (`tools/ffmpeg-NOTICE.txt`, B82).
4. **WORKER-INTERFACE §9's 21 questions** — each carries its recommendation, and each is
   built as recommended except #16 (the pending own-Mac row, left out as recommended) and
   #19 (the broken roster's reseed: only the set-aside is built; recommended: ask before
   reseeding, in the repair mood).
5. **“Disconnect this Mac…” as red text** (law 9, as built) against the spec's quiet text.
6. **r5's `update failed` id** has a space where the laws want kebab-case; the core keeps
   the spec's id until he renames it (B57 f).

**Decide**
7. **Intel** (§16 #27) — **decided** (B167, ratifies B7): Apple silicon only for v1,
   as built and as the landing and join page say; revisit when people ask.
   **The floor** is measured now (B83): 14.0 stands; 13.0 passes the probe but has no
   Ventura VM to be tested on (item 27).
8. **Release identity** (§16 #26, B66) — **release.** The `am.shroom` identifier (reuse on
   POC machines) and the release signing key's custody; recommended: probe the reuse on a
   POC machine first. `tauri.release.conf.json` carries placeholders. The bundled encoder
   is signed with the bundle (B82), so it rides this decision too. **Settled by the
   first release:** 0.1.5 shipped signed by the pinned identity (RELEASING.md); the POC
   reuse question stays open.
9. **Transcription** (§16 #29) — **decided for v1** (B168): none. After v1 it's for
   automatic naming and chapters; still under discussion.
10. **Pruning local copies** (§16 #37): recommended only behind upload confirmation,
    always keeping the single-file copy (law 2).
11. **The POC space** (§16 #48): migrate, retire, or accept the `shroom` label the wizard
    would take beside the POC's `v`.
12. **Which page the 1 s bar means** (B59): the owner's keyed page (~0.7–0.9 s, once
    1.04 s at `47dfaa3` — thin) or the link itself (~0.1–0.2 s).
13. **The Worker module's exported constants** (B63): split them out so a stricter loader
    can never refuse a deploy? A change to what the app uploads.
14. ~~**A Worker-held deletion record** (B38): closes the window where an already-issued
    upload credential can rewrite a tombstone (≤ 7 days). New design.~~ Built, B125
    (2026-10-13): `.shroom/gone/<id>`, sealed by the Mac that deletes.
15. **The two resting spinners with no face** (DEVIATIONS 11 and 32): every label taken,
    and a foreign Worker under the space's own label — each needs a state and a line.
16. **Publishing the BYO-agent skill** (ARCHITECTURE §16 #40): **retired** (2026-10-14,
    DECISIONS B132, DEVIATIONS #50): shroom.am's `/agent` page is how an agent installs
    shroom, its copy ratified the same day. Later, his idea: a doctor served like
    `/install`, for an agent to fix a broken shroom. The landing and `/install` at
    shroom.am are live, and 0.1.5 is the first GitHub release — **done.**

**Supply**
17. **The landing's video** (N7, DEVIATIONS 29) — **done**: the 15-minute take recorded on
    0.1.22 (2026-10-17, N58, B190) gave way on 2026-10-18 to “How to set up and use shroom”,
    four minutes in his own voice, served by shroom.am at `/how-to.mp4` (N61, B194).
18. **og:description** for a video page's link previews (B27, the copy table) — none is
    invented; R5.1's Teams card wants one. Whether “shroom” belongs on cards as a site name.
    The Teams rich-card probe needs a Teams tenant. (shroom.am's own pages have theirs,
    since [#8](https://github.com/bagrat/shroom/pull/8) *(FAQ, privacy and terms pages)*.)
19. **Recorded REST responses** (B63): the real account's answers, scrubbed, for L1 — his
    review that no account id, token id or name survives.

**Checkpoint sitting (his hands)** — scripted in [`CHECKPOINT.md`](CHECKPOINT.md): about
45 minutes when it was written, longer now that steps 7 and 8 carry items 28–35's checks,
in twelve parts, from scratch — on the official release from shroom.am: the landing, its
install line, the real installer, his own account. (The preview build it once ran on is
retired, B148.)
20. **The wizard end to end** on his account (CHECKPOINT §5), and whether the pre-filled
    token page's permission keys (`workers_scripts`, `workers_r2`) fill in (§16 #31).
    **Not in the sitting — a fresh Cloudflare account, another day:** the card gate's real
    code (B26 reads 10042, unobserved) and a first workers.dev name claim.
21. **A real login launch** carries the `lgit` mark (B45 a); **⌘Q mid-take with the
    Library in front** (B45 b — with the Library open the Dock's Quit is there too, and
    mid-take it does nothing either, item 28); **the behind-the-notch anchor** on a crowded
    menu bar (B45 c, B46) — CHECKPOINT §8. **A real lid-close on battery mid-take** (B92): does will-sleep come
    early enough for the pause to land inside the 2 s hold? — CHECKPOINT §8.
22. **The installer's update from Terminal.app:** does an Automation prompt (“Terminal
    wants to control shroom”) appear for the quit (B44)? — CHECKPOINT §10, step 3.
23. **The clean-VM permission clicks** as a fresh principal (TESTING-PLAN §8), and a
    `shroom://join/…` link opened there by a person — CHECKPOINT §10 (Tart clones he
    starts by hand). *Invitations: confirmed working
    (Bagrat, 2026-10-14) — by hand with the first release, 0.1.5, and a clean Tahoe clone
    (RELEASING, “What has not been proven”). The permission clicks: not reported.*
24. **A physical microphone's delay**, by ear: the clap in view of the camera —
    CHECKPOINT §7 (B59 e).
25. **Feel:** countdown timing, popover snappiness, the bubble — CHECKPOINT §7. With them,
    by hand (no machine can click from another app, or press a real Esc, without a
    permission): the popover's first click, the popover and bubble following to a
    full-screen app — and through a click on the Dock or in Mission Control — the count
    over a full-screen app, and with the Library open behind another app (DEVIATIONS #39,
    #40; B106).

*The sitting of 2026-10-14/15, on the official release — what it settled:*
- **0.1.10 (§4–8).** Passed: the landing's "Read it first", the installer's lines, the token
  page's pre-filled permissions (item 20), the wiring rows finding `v.myshroom.workers.dev`,
  the permission asks, record → page → private window, Library rename/delete, Members'
  invite, the countdown, popover and bubble, the first click, Return, the mushroom twice,
  the menus and No mic, following to full screen (⌘-Tab, Dock, Mission Control), Restart,
  ⌘H, the count over full screen and with the Library behind, the clap in sync (item 24),
  Spotlight's one shroom with no Dock flash, the Library's keys, the windows over full
  screen, ⌘Q mid-take and while paused (item 21). His rulings: the app switcher keeps the
  popover, Show Desktop leaves it (item 25); the Dock icon while a window is open (B138).
  Found and fixed since: Camera only lost its take (#73), the lid dropped to the rescue
  path (#79), the popover's corner on Spotlight (#78, #90); copy and design asks on the
  landing, Members (B137), Space (B139) and the removed link (B141). Not reached: Invite
  someone at the wizard's end, two displays, the notch reopen.
- **0.1.12 (§9 and the fixes).** Passed: the update (staged, signature matched), Camera only
  (#73), the lid's pause and Resume (#79, item 21), ⌘Q with nothing recording (#81), Ready ✓
  and the removed link's not-found page (#72), Settings → Uninstall and the new permissions
  chapter (#76). Failed, fixed in 0.1.13: the popover's corner on a mirrored display (#90).
- **0.1.13** (all five checks passed). Passed: the popover opening by itself after the
  update (B155), What's new on shroom.am, the popover under the mushroom on a mirrored
  display (#90), System default following AirPods (B149), the mushroom on a dark menu bar
  (B151), and a mic disconnect mid-take pausing with its line (judged good: it tells you rather
  than switching silently).
- **0.1.15.** Passed: the update found by a click on the mushroom, the popover opening by
  itself on “Updated to 0.1.15 ✓” (B154, B155: all passed); the pause popover over a
  full-screen app when the mic is unplugged, placed under the mushroom's spot, parked over
  a hidden menu bar (B161: passed). Copy thumbnail (B164) copies and pastes, with
  three findings sent for fixing: right after upload the paste is a still JPEG and later
  copies the GIF; the GIF's quality is low; the length chip shows only after a rename. His
  suggestion: Copy thumbnail disabled until the picture is ready.
- **§10 on 0.1.15** (two Tart clones, a brand-new Cloudflare account, 2026-10-15/16): the
  stranger's run passes end to end — items 20's fresh-account half, 22 and 23. Every earlier
  miss re-checked and fixed: the email-first line; the card step's line and Add credit card
  (about 30 s of Cloudflare's own lag before the card counted); the token box still while
  checked; permissions before wiring (wiring finished underneath, 65 s after the relaunch);
  no camera ask without a camera; “Invites will say they're from …” with Change; no
  invite-link field; a removed member's Settings; Uninstall at the welcome's foot; the
  install line's “up to date — opening it now”; the uninstall line's last words; Copy
  thumbnail and the viewer's buttons. Not watched: whether a false offline line or “Almost
  there…” showed during wiring. Two minor findings sent for fixing: the card step flashes
  for about 6 s after the permission relaunch, and the popover sits on a see-through
  square-cornered backing that shows past its rounded edge (on his own Mac too).
- **Nothing left** in this sitting.

**Machine changes (his yes — outside TESTING-PLAN §9's list)**
26. **gpg**, to check the FFmpeg tarball's own signature (B82). Until then the pinned
    SHA-256 (two witnesses) is the gate, and a mismatch deletes the download.
27. **A Ventura VM** beside the Sequoia and Tahoe goldens — only if he wants the floor at
    13.0 (B83).

**From his from-scratch run (2026-10-12)**
28. ~~**No Dock icon once shroom is set up** (DEVIATIONS #38 — his ask, built, awaiting his
    read; law 6 as ratified gave the Library and Settings one).~~ **Settled:** #38 was
    rejected (2026-10-13) and law 6 restored; at the 0.1.10 checkpoint he confirmed it:
    shroom is in the Dock while any of its windows is open (B138). The history below is
    kept as it was. What a Dock icon gave for
    free is proven without it on a clean VM (B105, `tests/l5/dock-vm.sh`). **His hands,
    CHECKPOINT §8:** the Library with no Dock icon — back from the mushroom's popover,
    typed into, ⌘C and ⌘V, ⌘W — and ⌘Q mid-take. **All-set's Invite someone handing over
    to Settings** had not run anywhere at the close, and could not have: the drive's flag
    was the install script's own, and the app ended the launch (B117). Mended, and seen
    since on clean VMs of both macOS versions (`tests/l5/setup-vm.sh`): the Dock icon
    goes, Settings is in front with the keyboard and keeps it, and a name typed straight
    away lands in “Who's this for?”. **His hands, CHECKPOINT §5:** press Invite someone at
    the last step and type a name without clicking first — a hand on the button and a
    real keyboard are the part no machine did. **At launch too (B113):** a Dock icon did
    still
    show for a moment while shroom started — measured: every start, 105–241 ms, and a
    start by a plain open put shroom in front for 75–140 ms — and no longer does: macOS
    now hears of shroom as a menu-bar app from the first moment. **His eye, CHECKPOINT §8
    and §9:** the Dock while a set-up shroom starts, and while an update comes back.
    **His to weigh:** a first run's Dock icon now arrives with the welcome, about half a
    second in, instead of from the first moment.
29. **Ratify DEVIATIONS #41** — switched off, the mark leaves the owner's own page too, the
    switch staying alone bottom-right — **and rule on B107, B109, B110 and B111:** every
    typed field counts its edits, and an edit the core never took gives way 2 s after the
    core has next spoken (B107); the tab icon is always there, whatever the switch says
    (B109); the page's switch saves one flip at a time, so a tab closed while a write is on
    its way keeps the earlier flip (B110); the drives' half-second hold once the keyboard
    has left (three seconds at first; measured since, B111 c), and a failing drive keeping
    its throwaway home (B111).
30. **By hand, in a 0.1.4 build:** in the Library, click a title, type over the selected
    one, Enter; again, typing in the middle; Esc; a click away. In setup, a pasted token
    shows as dots; a link name typed quickly with an uppercase letter or “!” in it.
31. **By hand, once 0.1.4 is on his space:** untick “made with shroom” — the logo goes at
    once and stays gone after a refresh, the switch still there; a private window shows
    none; two quick clicks, then a reload, come back as he left it. A browser tab shows the
    icon on a video's page and on a join link (no headless browser draws a tab).
32. **With this Mac unlocked:** `node app/test/launch-drive.mjs` (run here once since,
    unlocked: 8 of 8, nothing unseen — Incidents),
    `node app/test/onboarding-drive.mjs all`, the take drives and the heartbeat at 0.1.4 —
    none could run behind the lock, and none was run at the close with him at the Mac:
    they are his to start. (As the close left it, `onboarding-drive.mjs all` would have
    failed in his hands whatever the screen — B117. Its seven paths have run green on
    clean VMs since; on this Mac only `doors` has, behind the lock.) The heartbeat first
    wants the dev app back in
    `~/Applications` (B4): there is none there now, so its preflight would stop, and the
    update drive's “installed app untouched” had nothing to compare.
33. **The popover, the count and the bubble on a real desktop** (DEVIATIONS #39, #40 — to
    ratify; B106, B112). **First, with this Mac unlocked, the takes:**
    `node app/test/take-drive.mjs left-out`, then `signals`, then `movies`. A take since the
    popover and the count became panels has been recorded and read in clean VMs on both
    macOS versions (B114) — it starts, and leaves them out — but not on this Mac (the
    lock), whose screen has what a clone's hasn't: a notch, a wide display, a camera, a
    microphone. `left-out` covers the screen with the pattern for about fifteen seconds
    and exits 0 only when every frame is the pattern alone. **By hand, CHECKPOINT §7:**
    the first click and Esc with another
    app in front; the popover and the bubble following to a full-screen app, by ⌘-Tab, by a
    click on the Dock, by Mission Control; the count over a full-screen app, and over the
    Library behind another app; ⌘H with the popover and the bubble up, and with the bubble
    in a take. **His to rule:** a popover that opened by itself and that he hasn't touched
    is not closed by a click elsewhere; a click in the Library or Settings closes an asked
    popover; a popover with the keyboard takes ⌘H, ⌘W and ⌘Q for shroom while his own app
    looks in front — ⌘H now hides the Library and leaves the popover up; both overlay pages
    load when shroom starts — measured again (B106; debug builds, a 26.6.2 clone): a shroom
    nobody has clicked since it started holds about 145 MB more than before (about 50 MB
    as Activity Monitor counts) and four helper processes; from the first click to the
    first count about 40 MB more; after that nothing — the build before made the same
    pages at the first click and the first count, and kept them.
34. **From the reviewers' notes (B115).** **By hand, CHECKPOINT §7 and §8** — the new
    lines there: Return in the popover; a second click on the mushroom (on macOS 26 the
    mushroom on screen is the menu bar's own window, and no machine here can click it);
    the Record and Microphone drop-downs with another app in front; a click on the other
    display; a click in the app switcher (never measured — either answer is news); the
    first Start since shroom opened with the Library behind another app; the Dock as the
    desktop comes up after a login. **His to rule:** with the windows swept aside by Show
    Desktop, or in App Exposé, a click on the bare desktop leaves the popover open — the
    click watch reads both as Mission Control (DEVIATIONS #39 now says so); and whether a
    shroom nobody has clicked should carry both pages from the start (B106's price).
35. **The close (B116) — the batch's rulings, all ruled.** B106 ruled as built 2026-10-15 (“happy wit hthe popover behavior btw”). The rest
    is ruled (2026-10-14, “ratifications are all good, only B106 is questionable, remind to
    discuss it later”): DEVIATIONS #39, #40 and #41 ratified; B104, B105, B107–B115 and
    B117 as built; #38 rejected and B116's CI question ruled (item 36). **B106** —
    the popover and the count made as panels at start-up, at about 145 MB and four helper
    processes until the first click — ruled as built. Each ruling in one line:
    [`history/build-status/waiting-35-36.md`](../history/build-status/waiting-35-36.md).

36. **After the close (B118) — what is left of the hosted Mac's pass.** B118 accepted;
    DEVIATIONS #42 (the camera preview stays for the take) proven on his Mac; CI's Tests
    job ruled and green. Still his: **(a)** the consent-dialog proposal for the take and
    setup lanes (B114, B117) — probe once in a clone, both images, before the lanes rely
    on it; **(b)** the four L5 lanes the branch touched, both images (“After the close”
    in [`history/build-log/2026-10-13.md`](../history/build-log/2026-10-13.md)). The full
    item: [`history/build-status/waiting-35-36.md`](../history/build-status/waiting-35-36.md).

**The editor ([#61](https://github.com/bagrat/shroom/pull/61) *(trim, cut, undo, markers,
and edits on the share page)*)**
37. **Its words, from use.** N63's, N64's and B197's drafts, with the build's two new
    lines (a save the space refused, and the time left by Revert counted by the hour; B199
    (k), (n)), were **ratified by Bagrat as built** (2026-10-18), his feedback to come from
    using them.
38. **N63 and N64, and B195–B199, ratified by Bagrat as built** (2026-10-18), the cut-edge
    drag (B195 (g)) built as the board he approved that day. DEVIATIONS #55 (Yes, delete
    filled and leading, but never on Return) is ratified too.
39. **Edit in shroom on the owner page** (N69, B221; Bagrat's ask and his approval of the
    boards as drawn, option A and the tooltip, 2026-10-18): the scissors open the video in
    the editor through `shroom://edit/…`, or the popover says, once, why this Mac can't.
    He ratified its spec row and its four lines as built on 2026-10-24. Since 2026-10-19
    each video opens in an editor window of its own (N73, B227, ratified by Bagrat as
    drawn), so its line for another video is gone; and ✕ with changes not saved asks first
    (N71, B223).
40. **Known gaps**, none blocking:
    - The final video's bar counts the upload by file, not by byte: once the cut is done it
      holds at 80% while the video goes up (minutes, for a long one), then 95, 97, 99, 100.
    - After a bake, the Library row on the Mac that holds the take may show the take's own
      poster until the Library's next refresh.
    - After an edit, a new Copy thumbnail picture is made the next time the owner opens the
      page; until then the Library's Copy thumbnail falls back to the page's poster, or to
      the plain link.
    - The upkeep on a Mac without the token (the space's records over the Library API, a
      15-minute `manage` credential for each bake and copy) is untested on a real second
      Mac: the hosted Mac's editor drive runs as a token Mac against the fake space.

40. **N68 — the page counts plays, and the Library suggests videos to clean up** (built;
    B210–B214). He ratified it as built on 2026-10-18: its words and rules, the privacy
    line (the page carries it in the same PR, so it ships with counting), each row's cost
    past the free 10 GB, and the 1¢ size gate (B211). Proved on the hosted Mac (the cleanup
    drive, 22 checks). Waits only on his timing of the release, with the other four builds.

41. **The `shroom` command (N62, B215–B219), built in full on his ask and ratified
    (2026-10-18) — shipped in 0.1.25 (2026-10-19).** shroom.am/cli is live, and the install
    line and the cask link the command (the hosted Mac's install scenarios on v0.1.25). He ratified N62 and its decisions, the
    Settings card's words as built (he'll tweak them once he uses it), and the command's
    words and the guide as built; the refusal's three lines, the switch's label and the
    agent label's board he had approved as drawn. The security review's one open point, the
    switch being consent rather than a wall against code already running as the person, he
    accepted as a risk (B216). Proven: CI, the hosted Mac's `cli-socket`, `settings-link`,
    `settings`, `cli-label` and `take-left-out` drives, and the full e2e's command leg on
    the test account.

42. **`shroom edit` (N72, B225), ratified by Bagrat as built** (2026-10-19; the
    orchestrator, 2026-10-19: build it now the editor has shipped). The command says what a video's edit
    is and makes one change a call — trim, cut, put-back, apply, resize, revert — the
    editor's own steps, saved as its Done saves, through the space's page; the running app
    is asked whether the video is open in an editor window, and to make the final video. Ratified with it: its
    words (`crates/shroom-cli/src/say.rs`, the edit section), the guide's *Editing* section,
    and B225's calls — the command saves itself rather than through the app, refuses while
    the video is open in an editor window, and never asks “Delete the original?” (Keep for 2 days).

43. **N75 — Pin, Delete all, and auto-cleanup run by the space's page** (B232–B234; Bagrat
    approved every board and its words as drawn, 2026-10-24, relayed by the orchestrator).
    Pin is the record's `keep`, from a row's menu and `shroom pin`/`unpin`; Delete all… asks
    once and holds one Undo; auto-cleanup (off by default; 15, 30 or 60 days unwatched, or a year since N77, B236) is
    set on the Mac that set up the space and run by the space's page on the one schedule AI
    naming uses, three days' warning first in the Library, the popover and the command.
    Proved in CI (the core walkthroughs, the Worker's `autoclean.test.js`, the command
    against the real page, the window gallery). Ships with 0.1.29. On the hosted Mac since
    2026-10-25: the `cleanup` scenario walks it in the real app — Delete all… and its one
    Undo, Pin and Unpin from a row's menu, the Settings pop-up (Off as it ships, needs Count
    plays), the warning in the Library and the popover banner, the page deleting on its own
    schedule with every window closed, and `shroom cleanup-candidates` matching the Library.

44. **N80 — Turning AI on or off shows its parts as they happen** (B249; Bagrat approved the
    boards with the plainer words, 2026-10-25, relayed by the orchestrator). The switch in
    Settings · Space moves at once and lists Checking your space · Connecting (or
    Disconnecting) AI · Switching AI on (or off) for new videos, the one underway with its
    clock, done once the page says so; the banner's Turn off lists the same parts. Measured
    on the test account (2–8 s a turn). Proved in CI (the core tests, the cloud fake, the
    window gallery). Known gap: the app's lane (`runtime/cloud.rs`) is compiled and run on
    the Mac only; no hosted-Mac drive turns the switch yet.

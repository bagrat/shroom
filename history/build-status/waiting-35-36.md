# Waiting items 35 and 36, in full

Moved out of BUILD-STATUS.md's waiting list on 2026-10-14; the list keeps a short
form of each under the same number. Frozen.

35. **The close (B116) — and every ruling the batch's rows await, in one place.**
    **To ratify, DEVIATIONS:**
    - **#38** — no Dock icon once shroom is set up: the reading built (the setup window
      alone has one), or the other — no icon ever.
    - **#39** — the popover follows you; from outside only a click somewhere else closes
      it. Inside it: a click on the Dock or in Mission Control closes nothing; with Show
      Desktop or App Exposé up, neither does a click on the bare desktop; a popover that
      opened by itself and that he hasn't touched is not closed by a click elsewhere; a
      click in the Library or Settings closes one he asked for; ⌘H dismisses nothing.
    - **#40** — every overlay on any Space and above a full-screen app; the count plays
      where he is and leaves him there; a first click acts.
    - **#41** — switched off, the mark leaves the owner's own page too, the switch alone
      bottom-right.

    **To rule, DECISIONS:**
    - **B104** — a pasted token, and a link pasted in Settings · Space, show as a fixed
      row of sixteen dots while shroom holds them; the name row says “v” was taken only
      where one was seen.
    - **B105** — with no Dock icon, the keyboard goes back to the app he was in when the
      last window closes.
    - **B106** — the popover and the count are panels made as shroom starts. Its price:
      about 145 MB more (about 50 as Activity Monitor counts) and four helper processes
      until the first click.
    - **B107** — an edit the core never took gives way 2 s after the core has next spoken.
    - **B108** — the mark's off look as built for #41: the logo gone at once, no reload.
    - **B109** — the tab icon is always there, whatever the switch says.
    - **B110** — the switch saves one flip at a time: a tab closed while a write is on its
      way keeps the earlier flip.
    - **B111** — the drives hold half a second once the keyboard has left; a failing drive
      keeps its throwaway home.
    - **B112** — ⌘H with the popover open hides the Library and leaves the popover, the
      bubble and a playing count; a popover opened while shroom is hidden no longer brings
      the Library back.
    - **B113** — shroom is born a menu-bar app: a first run's Dock icon arrives with the
      welcome, about half a second in; with the mushroom truly hidden, a launch's popover
      reaches its fallback corner about 0.3 s later than before.
    - **B114** — the take lane writes a clone's own privacy database and ends macOS's
      consent dialog by its PID; the pattern guard looks again for up to 8 s; one drive
      records the count on purpose, as the reader's proof.
    - **B115** — the launch drive counts as green behind a lock with one thing named
      unseen; the mushroom's place stays the mushroom's (the dead zone, left as built);
      where shroom may not record, Return's check accepts “recording is off”.
    - **B116** — the update lane writes a clone's privacy database too; the Library's
      delete test's stand-in changed and its assertion didn't; **and whether GitHub's
      runner is held to the recorder's real-time tests** — CI's Tests job has never been
      green on `main`, before this batch or with it. *That question ruled (2026-10-13):
      as item 36 recommends.*
    - **B117** — the setup lane writes a clone's privacy databases too, the microphone's
      row beside the screen's, and ends macOS's consent dialog by its PID as the take lane
      does; the onboarding drive's Invite someone flag is `--invite-someone`; after
      Invite someone the drive types a name; the checkpoint asks him to press Invite
      someone at the end of setup.

    **To know before a push or a tag:** `main` has the batch as far as `decdafd`; CI's
    release build and signing rehearsal are green there with it; its Tests job is red, as
    it was before the batch (RELEASING, “What has not been proven”). 0.1.4 has not been
    released — no tag exists, and RELEASING's routine now starts from that. **Not done in
    the batch:** a take on this Mac, the heartbeat, and a preview built from this tree
    (CHECKPOINT step 2 would be its first).

36. **After the close (B118) — the hosted Mac's pass and issues #3–#8.**
    **B118 accepted** (Bagrat, 2026-10-13).
    **By hand, on a 0.1.4 build (Bagrat, 2026-10-13):** the update works; the popover
    always on top, and the Dock, work; the keyboard goes where it should; the Library's
    rename works; the bubble works; a camera-only take records. One thing wrong: the
    camera's preview went away once the take began — it should stay. Built as
    **DEVIATIONS #42** (his words, ratified in the asking): the preview stays for the whole
    take. **Owed to his Mac** — no runner has a camera: a camera-only take with the preview
    up, watched back (the picture whole, the right size, smooth), and
    `node app/test/take-drive.mjs signals`, which now also checks the take keeps getting
    the camera's frames beside the preview.
    **His second round, by hand on 0.1.4 (2026-10-13):** a screen take's sound in sync by
    ear; full screen, the popover's small things, ⌘Q mid-take and a login all as they
    should be; the app switcher leaves the popover where it was (the never-measured case,
    now known). Two changes, both built and ratified in the asking:
    - **DEVIATIONS #38 rejected → law 6 as ratified:** the Dock icon while any window of
      shroom's is open. The Library and Settings, asked for from a full-screen app, open on
      a desktop with the icon, stay there when he goes back, and the icon goes when they
      close — proven on the hosted Mac ([38444269493](https://github.com/bagrat/shroom-archive/actions/runs/38444269493):
      surfaces 102 checks, library, settings, update, onboarding, launch all green).
    - **DEVIATIONS #41 changed:** switched off, the owner's corner is empty too — the
      switch, its words and the logo; Settings turns it back on. The page changed, so the
      tree is **0.1.5** (`tools/bump-version.sh`); Worker 439/439, workerd 14/14, the page
      drive 33/33 in Chromium and in WebKit twice running (one run before them ended not
      ok, its failing check not captured; every mark check passed in it).
    **His ruling on proof (2026-10-13):** “the video evidence is enough for the fixes as
    long as you added tests for them” — the hosted Mac's runs and videos stand for the L5
    lanes on this branch's fixes; each fix has its own test (the drives' checks above,
    the core's walkthroughs, the Worker's suites).
    **The signals drive on his Mac (2026-10-13), twice.** First on a bundle built before
    DEVIATIONS #42 (842fbfd) — green, holding the preview to the old rule. Then on this
    branch at `d841bf2`, rebuilt (`npm run build:dev`): exit 0, `"ok": true`, nothing
    skipped, on the FaceTime camera with an iPhone beside it — the camera take ran from
    the count through `recording` to `camera-lost`, so “the preview stays while the take
    records, live, on the recorded camera” and “…and the take keeps getting the camera's
    frames beside it” were both made and held (the drive's `ok` is every check). Read from
    his Mac's session's transcript; the report is `~/signals.json` there. **DEVIATIONS
    #42 is proven on a real camera.**
    **CI's Tests job: green for the first time**, on PR #11
    ([38440862125](https://github.com/bagrat/shroom-archive/actions/runs/38440862125)) — the three
    wall-clock tests passed; `beeps_land_on_their_seconds` missed its bar there and was
    reported as the agreed warning.
    **CI's Tests job (issue #7, B116) — ruled: agreed, all three (Bagrat, 2026-10-13);
    built on the branch.** What was recommended: The failures are
    all timing, and every take in them came out whole and readable. Two kinds:
    - *Wall-clock* (`synthetic.rs`: the steady source, the frozen screen, the fast
      source — “take 0 lasted 1.7666 s, wanted ~1.5”): each test sleeps 1.0–1.5 s
      between `start_take` and `end_take`, then wants `seconds` within ±0.15 s of the sleep.
      Under load, starting ffmpeg and stopping the take add 0.2–0.27 s the test never
      measured. **Recommended:** hold `seconds` between the sleep less a frame and the
      window the test actually held the take open (an `Instant` before `start_take`, one
      after `end_take`) plus a frame. On a quiet Mac that is as tight as now; it stops
      failing a runner for time the test itself spent.
    - *A/V sync* (`av_sync.rs`: drift ≤ 0.015 s, beeps on their seconds; `interruptions.rs`:
      picture vs sound ≤ 0.1 s): real bars on the media, missed when the runner starves the
      recorder. **Recommended:** keep the bars. On CI only, run these two tests in a step
      of their own that reports a miss as a warning (“timing bars: a Mac decides”), the way
      `record` ends with 2 on the hosted Mac; your Mac and the heartbeat keep them strict.
    - And run `shroom-record`'s test files one at a time on CI (`--test-threads=1`): four
      encoders on three cores is the load that brings this on.
    With all three, a red Tests job on `main` would mean something again.
    **The take and setup lanes end macOS's consent dialog by name (B114, B117) — a
    proposal, not done.** Each kills every `UserNotificationCenter` the clone has after
    the self-test, whatever it was showing. The hosted Mac never raises that dialog: before
    anything captures it writes the window-picker approval into
    `~/Library/Group Containers/group.com.apple.replayd/ScreenCaptureApprovals.plist` and
    restarts `replayd` (`dev/mac/lib.sh`, `approve_picker`). The lanes could do the same in
    the clone and then fail if the dialog shows up anyway. Not built here: it can only be
    proven in your VMs, and on macOS 15+ writing another app's group container from the
    guest agent might raise a prompt of its own and hang the lane. Probe once in a clone
    (both images) before the lanes rely on it.
    **To run on your Mac:** the four L5 lanes the branch touches, both images (the section
    “After the close”, in `history/build-log/2026-10-13.md`).

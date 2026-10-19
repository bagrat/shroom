# The checkpoint — from scratch, on the official release

The build proves itself on fakes, a fenced test account and a headless VM. A few things
only your hands and your own account can tell us. This is one sitting for all of them
(BUILD-STATUS, "Checkpoint sitting", items 20–25, and the by-hand checks of items 28–35),
walked the way a stranger meets shroom: the landing page, the install line, the
installer, the first launch. Allow about 45 minutes; you can stop after any step.

Your Mac carries only the official release (CLAUDE.md), so the sitting runs on it: the
one from shroom.am. Nothing here builds, serves or launches anything from the repo.

## 0. What this sitting proves, and what it can't

It proves the product end to end on your real Cloudflare account: the landing and its
install line, the installer, the wizard, a recording and its page, the Library, Settings
and Members, how it feels, a real login launch, an update, and a Mac that has never seen
shroom.

What it needs: your Mac, a browser signed in to Cloudflare (step 5 makes a token), and
for some of step 7 and 8 a full-screen app, a second display and a MacBook on battery.

Two things need a brand-new Cloudflare account and can't be shown on yours: the card gate
(your account already has a card), and claiming a workers.dev name for the first time
(yours is `myshroom`, and stays so). Step 10 shows them, on a new account in a VM.

## 1. Start clean

If shroom is on this Mac, take it off — the app, its folder and its permissions; never
your space, your videos or your links. In shroom: Settings · General · Uninstall shroom….
On a shroom without it, the install line does the same:

```bash
curl -fsSL https://shroom.am/install | sh -s -- --uninstall
```

(It stops, changing nothing, mid-take or while a recording hasn't reached your space yet.)
Then, in the Cloudflare dashboard under API Tokens, delete the token shroom made last
time, so step 5 meets the token page fresh.

Your space stays on your account, so setup finds it again: that is the path most people
coming back will walk.

## 2. Which release

The sitting runs on the latest release at shroom.am. Step 9 updates it to the next one,
so it waits until a newer release is out — do steps 1–8 now and step 9 when it is.

## 3. The site

The landing is the real one: **https://shroom.am**.

## 4. The landing, and the install line

- Read the page as someone who has never heard of shroom. Note anything that reads wrong.
- "Read it first" opens the installer as plain text — the same script the line runs.
- Press Copy beside the install line, open Terminal, paste, and press Return.

The installer says "Getting shroom…", "verified ✓", then "Installed — opening shroom
now." — and shroom opens.

## 5. First launch: the wizard and the permissions

- The welcome window is up, and shroom's icon came into the Dock with it. Walk it.
- When it opens Cloudflare's token page, check the permissions come pre-filled: Workers
  Scripts and Workers R2 Storage, both Edit (§16 #31). Note what you see.
- Create the token, then paste it into shroom — only into shroom, never into a chat.
- Watch the wiring rows tick, one by one. Your space is still there, so the name row
  finds it: "✓ Your link name — v.myshroom.workers.dev".
- Then the permissions chapter. The uninstall reset them, so macOS asks for real: the
  microphone, the screen, and the camera — all three during setup. Click through them
  yourself — the first-time yes. On the screen ask, only macOS's own alert should show,
  nothing of shroom's over it (#76).
  If shroom reopens itself during permissions, it comes back as the same app, where you
  left off.
- At the last step press **Invite someone**, not Close. Settings opens on Members, in
  front, and shroom's icon stays in the Dock while it's open (B138). Don't click
  anything: just type a name. It should land in "Who's this for?".

## 6. Record, share, tidy

- Record a short take, and stop — the bubble should be on at your first Start (#76).
  Your page opens in the browser: once the upload is done
  it shows "Ready ✓" briefly (#72). Give the take a title there, and Copy link.
- Open the link in a private window (Safari: ⇧⌘N), as someone you sent it to would.
- In the Library: rename a take, then delete one. Open the deleted one's link again, and
  reload it: a plain not-found page, the same as a link that never existed (#72).
- Settings · Space: read what it says about your space.
- Settings · Members: type a name in "Who's this for?", Invite, and Copy link. The link
  shows only while Settings stays open — afterwards just the pending row is left
  (Remove… it if you like).

Disconnect is real here: it makes this Mac forget your space (the space itself stays on
your account). Leave it be.

## 7. Feel

- The countdown: does it feel right before a take starts?
- The popover: is it quick to open and to answer? With a camera picked in its camera
  picker, it should open at once, the bubble showing a spinner until the camera
  delivers (B119).
- The bubble: pick your camera in the popover's camera picker, and see how it sits and moves. Pick
  Camera off: it goes. Pick it again: it comes back on the camera you picked.
- The first click: with another app in front, click the mushroom, then click the camera
  row once. Its list of cameras should open on that first click; Esc should
  close the list, and Esc again the popover, without you clicking in it first.
- Following you: open the popover on a desktop with a camera picked in the camera picker,
  then ⌘-Tab to an app that is full screen. The popover and the bubble should both be
  there with you. Click anywhere in that app: the popover closes, and the bubble goes
  with it.
- Following you by a click: open the popover again, then switch apps with a click on the
  Dock — and once more by picking a window in Mission Control. Both times the popover and
  the bubble should still be there when you arrive.
- With the Library open: quit shroom and open it again, open the Library, click into
  another app so the Library sits behind it, then click the mushroom and press Start — the
  first count since shroom opened. The count should show and Esc should cancel it, and the
  Library should still be where you left it.
- Return: click the mushroom and press Return, touching nothing else. The count should
  start; Esc cancels it.
- The mushroom, twice: with the popover open, click the mushroom. The popover should close
  and stay closed.
- The menus: with another app in front, click the mushroom, then pick another display, and
  another microphone. Each should take your pick, and the popover should stay open
  through both. Pick No mic: a take then records without sound. Pick Camera only: Show
  yourself stays, as the camera the take records, and has no Camera off (DEVIATIONS #43).
  Move the camera's frame, and start a take on Camera only: the frame should stay where
  you put it, and the camera's preview on screen while it records (DEVIATIONS #42).
  Stop it: the take should be in the Library (#73).
- Restart: start a take, pause it, and click Restart. It should ask first ("Start over?");
  Back keeps the take, paused; Restart starts the count, and Esc during the count gives the
  take back (DEVIATIONS #44).
- Two displays, the clicked one (N56): with “Displays have separate Spaces” on and a camera
  picked, click the mushroom in the MacBook's menu bar. Record should show the MacBook's
  display, and the bubble should come up there. Close the popover and click the mushroom on
  the external display: both should be on the external one now, and so should the count
  after Start. Then pick the MacBook's display in Record by hand and reopen from the
  external display: the pick should hold until that take ends.
- Two displays, the popover's display (N57): from the MacBook's popover, open Settings and
  the Library. Both should open on the MacBook. After an update, the popover that opens by
  itself should hang under the mushroom, not in a corner.
- Two displays: with both displays awake, open the popover and click on the other display
  — once in a window, once on bare desktop. The popover should close both times. If it
  ever stays, note which display had the Dock.
- The app switcher: open the popover, hold ⌘, press Tab, and click an app's icon in the
  switcher. The popover should still be there when you arrive.
- Show Desktop: with the popover open, sweep the windows aside (Show Desktop) and click the
  bare desktop. The popover stays.
- Hidden: open the Library, pick a camera in the camera picker, click the mushroom, and
  press ⌘H. The Library should go; the popover and the bubble should stay. Click the
  mushroom to close the popover, click it again, and open the Library from the popover: it
  should come back in front. Then start a take, click the Library, and press ⌘H: the
  bubble should stay on screen — and be in the take when you watch it back.
- Over full screen: in a full-screen app, click the mushroom (move the pointer to the top
  for the menu bar) and press Start. The count should play right there, Esc should cancel
  it, and at no point should macOS slide you to a desktop. Start again and let it run: you
  should still be in the full-screen app when the dot turns red.
- The microphone, by ear: with a camera picked in the camera picker, clap three times in
  view of the camera. Watch the take back — does each clap's sound land with your hands?
  (B59 e)

## 8. The same app, every time

- Quit shroom from its popover, then open it from Spotlight: type "shroom" and press
  Return. There should be one shroom to pick, and it should be the same app with the same
  setup — its popover opens, ready to record. Watch the Dock while it opens: no shroom
  icon should show there, not even for a moment, the app you were in should stay in front,
  and the popover should hang right under the mushroom (#78) — with the Mac mirrored to
  an external display too, if you have one to hand. If it doesn't, send `run.log`: each
  popover's line there says where it went and why.
- Settings · General: make sure "Start shroom when you log in" is ticked. Log out, and
  log back in. Nothing should open but the mushroom — watch the Dock as the desktop comes
  up: no shroom icon there, not even for a moment — and your space is still there. Then
  check it knew it was a login launch:

```bash
grep 'boot: launch login' ~/.shroom/logs/run.log
```

- The Dock icon: shroom is in the Dock only while one of its windows is open. Open the
  Library — shroom should appear in the Dock and in ⌘-Tab. Click another app so the Library
  goes behind it, then bring the Library back from the mushroom's popover: it should come
  to the front. Click its search field and type — it should land there, and ⌘C and ⌘V
  should work. Press ⌘W: the Library closes, the Dock icon goes, and what you type next
  goes to the app you were in before.
- Over full screen, the windows: in a full-screen app, open the Library from the popover.
  It should open on a desktop, not over the full-screen app. ⌘-Tab back to the full-screen
  app, then close the Library and open Settings from there: it should open on a desktop
  too.
- ⌘Q, nothing recording: open the Library and, with it in front, press ⌘Q. The Library
  closes, and shroom stays in the menu bar (#81).
- ⌘Q, mid-take: open the Library, start a take, click the Library so it's in front, and
  press ⌘Q. Nothing should happen: no question, and the take keeps recording (B45 b).
  Then click the mushroom to pause, click the Library again, and press ⌘Q once more — the
  paused popover should come back; Stop the take there. (shroom only asks before quitting
  when nothing is recording and a video is still uploading.)
- The notch: crowd your menu bar until the mushroom hides behind the notch. With shroom
  running, open it again:

```bash
open ~/Applications/shroom.app
```

  The popover should land at the top right of the screen your pointer is on (B45 c, B46).
- The lid, on a MacBook running on battery: start a take, close the lid, wait ten seconds,
  and open it. The take should have paused the moment the lid closed, and the paused
  popover should be there when you're back, with nothing lost (B92); Resume carries the
  take on (#79). Then check macOS
  waited for the pause:

```bash
grep 'power: macOS will sleep' ~/.shroom/logs/run.log
```

## 9. The update

Once a newer release is out (one patch above the tree's: shroom 0.1.26), shroom finds it
by itself — it looks each time you open the popover (B154) — so click the mushroom.
Within a few seconds the mushroom gets its small Frost dot. Click it: a slim banner reads
"shroom 0.1.26 is ready." with Update beside it. ("What's new" opens the release notes.)

Press Update. It says "Updating — back in a few seconds.", closes, and comes back by
itself, with no window open, so no icon in the Dock as it comes back. Its popover opens by
itself under the mushroom and says "Updated to 0.1.26 ✓". The app you're in stays in front,
and what you type still goes there. Click anywhere else and the popover goes; the line
shows only once (B155). Your
space, your takes and your permissions are all as they were. Settings · General should
now read "shroom 0.1.26 — up to date."

To see it found the update — and that the update's signature matched the copy it
replaced (B125), which the second line shows by finding nothing:

```bash
grep 'updater staged' ~/.shroom/logs/run.log
```

```bash
grep 'updater not applied' ~/.shroom/logs/run.log
```

Then update once more from Terminal, with shroom running — the install line again. Note
whether macOS asks "Terminal wants to control shroom" (B44).

## 10. Two strangers: Macs and a Cloudflare account that have never seen shroom

Two macOS VMs on your Mac and a brand-new Cloudflare account you sign up for inside the
first — a real stranger's first run, and the friend they invite (your go, 2026-10-14: “for
clean mac we can actually try in a vm and with a completely new cloudflare account”; “also
… spawn second vm and test the invite from wizard, and the invitation itself, and how the
members look on inviter vm”). It is the one place the two things §0 can't show on your
account get shown: the card gate and claiming a workers.dev name for the first time. Only
the official release goes in; nothing from the repo runs.

- **Owner** — `shroom-owner`: signs up for Cloudflare, sets up, and invites from the
  wizard's last step.
- **Friend** — `shroom-friend`: opens the invite link and joins as a member.

**What it needs.** Tart, and its clean Tahoe image `shroom-clean-tahoe` — both already on
your Mac (TESTING-PLAN §4, M4); the image is never run itself, only cloned. Each clone is
copy-on-write, so it starts at almost nothing and grows by a few GB; each takes 6 GB of
memory while it runs (two at once is macOS's limit for macOS guests). A fresh email
address the new Cloudflare account can use, and your card (storage needs one on file; a
few takes cost nothing).

**What the VMs can't show.** They have no camera, so the camera picker, the bubble and camera
at setup can only be seen saying there's no camera; their sound may be thin or missing.
Those stay with steps 5–7 on your own Mac. Timing in a VM is no measure of feel either.

1. Make both, with a shared folder to pass the invite link through — on your Mac:

```bash
tart list | grep shroom-clean-tahoe
mkdir -p ~/shroom-handoff
tart clone shroom-clean-tahoe shroom-owner
tart clone shroom-clean-tahoe shroom-friend
tart set shroom-owner --memory 6144
tart set shroom-friend --memory 6144
```

   (If the first line prints nothing, the image is gone: clone
   `ghcr.io/cirruslabs/macos-tahoe-vanilla:latest` instead — a ~25 GB download, once,
   same login.) Then each in a Terminal tab of its own. `tart run` holds its tab until the
   VM stops, so give each its own tab. (`$HOME`, not `~`: zsh leaves a `~` after `=` as it
   is.)

```bash
tart run shroom-owner --dir=handoff:$HOME/shroom-handoff
```

```bash
tart run shroom-friend --dir=handoff:$HOME/shroom-handoff
```

   Log in to each as `admin`, password `admin`. The shared folder shows in each one's
   Finder under Locations, and at `/Volumes/My Shared Files/handoff`.

**On the owner:**

2. In Safari, open **https://shroom.am** and read it as a stranger. Copy the install line
   into Terminal and run it.
3. Walk the welcome window. When it sends you to Cloudflare, **sign up** with the fresh
   email — inside the VM, as a stranger would. Note each place the wizard waits on you and
   what it says: confirming the email, adding the card, creating the token (Workers Scripts
   and Workers R2 Storage, both Edit, pre-filled), and **claiming your workers.dev name**
   — pick one, and the link name row should end on it. A brand-new account has to confirm
   its email before Cloudflare will make the token, and a workers.dev name claimed for the
   first time takes a few minutes to go live, so expect a wait there.
4. The permissions chapter: macOS asks for real, for the first time, in this Mac — click
   through each yourself (TESTING-PLAN §8).
5. At the last step press **Invite someone**. Settings should open on Members, in front;
   type the friend's name without clicking — it should land in “Who's this for?” — and
   Invite. Read the new row and its line, then **Copy link** and save it into the shared
   folder:

```bash
pbpaste > "/Volumes/My Shared Files/handoff/invite.txt"
```

6. Record a short take and stop. The page opens; open its link in Safari's private window.

**On the friend:**

7. Open the link as a person would: in Terminal, `cat "/Volumes/My Shared
   Files/handoff/invite.txt"`, then paste it into Safari. Read the join page as someone
   who was sent it: who invited them, what they're asked to do. Follow it — install,
   join, and click through the permission asks yourself.
8. Record a short take into the owner's space. Settings has no Members for a member: note
   what it does show.

**Back on the owner:**

9. Settings · Members: the invite's row should have become the friend, joined. Note how
   the list reads: you and the friend. Macs show only once someone has a second one. In the Library, the friend's take
   should be there too.
10. Run the install line again, with shroom running. Already the latest, it should say
    “shroom … is up to date — opening it now.” and download nothing; with a
    newer release out, “Updated shroom to …”, and come back (B156). Note whether macOS
    asks “Terminal wants to control shroom” (B44).
11. Remove the friend in Members (Remove…). On the friend, note what shroom says and does
    next. Then take shroom off the friend: Settings · General · Uninstall shroom… (or step
    1's line).

12. Clear it all away, on your Mac:

```bash
tart stop shroom-owner
tart stop shroom-friend
tart delete shroom-owner
tart delete shroom-friend
rm -rf ~/shroom-handoff
```

   The new Cloudflare account is yours to delete or keep (its dashboard, Manage
   Account); its worker and storage go with it.

## 11. Keep it, or clear it away

shroom and your space are yours to keep — nothing below is needed. To take shroom off this
Mac, the step 1 line again:

```bash
curl -fsSL https://shroom.am/install | sh -s -- --uninstall
```

It never touches your space: your videos and links stay. Anything you saved to Movies
stays too.

## What to send back

- What you saw in steps 4–10: the landing, the installer's lines, the token page's
  permissions, the first-time permission asks, the feel — each line of step 7 — the clap,
  Spotlight and the login line, the Dock while shroom starts, ⌘Q mid-take, the notch, the
  lid, the update, the Terminal prompt; the stranger's sign-up, card and name claim, their
  permission clicks, the invite from the wizard, the friend's join page and join, Members
  on both sides, the removal and the uninstall.
- Your answers to the "Ratify" list in BUILD-STATUS (items 1–6), and whichever of
  "Decide" you're ready for.

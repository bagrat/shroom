# Draft decision row: auto-cleanup runs in the space's page (for review; becomes a B-row in docs/DECISIONS.md)

**Auto-cleanup's deletes run in the space's own page, on its one schedule** (Bagrat, 2026-10-24: off by default; Off / 15 / 30 / 60 days from the last watch, or from the upload if never played; a pinned video is never deleted; a warning a few days ahead with a one-tap Pin; and the page, not a Mac, does the deleting, so a closed Mac never misses one and the bill never grows for it).

- **The setting.** `.shroom/autoclean.json` = `{days: 0|15|30|60, at, since}` in the space's bucket. The Mac that holds the setup key sets it from Settings · General, because turning it on gives the page its schedule, the way turning AI on does (B206 a). Every other Mac reads it (`GET /_shroom/api/autoclean`, device key). It needs Count plays: with counting off, nothing is due.
- **One schedule, shared.** The page keeps the single Cron Trigger B206 already uses, every minute, wired while AI *or* auto-cleanup is on. Turning auto-cleanup on re-wires the page with it from the setup Mac (`wire::set_schedule`), once; turning both off takes it away. A second expression would cost the account another of its Cron Triggers for nothing.
- **What's due.** A video is due `days` after its last counted play (the owner's own never count, B210), or after `createdAt` if never played, and never before counting began (`views/since.json`). The page keeps `.shroom/autoclean/state.json` = `{scanAt, cursor, going: {<id>: {warnedAt, deletesAt}}}`. Once a day it walks the records a few at a time, one page per scheduled run (CPU stays small), adding each video that will be due within three days with `deletesAt` = the later of its due date and `warnedAt` + 3 days, and dropping any that were watched, pinned or deleted. So nothing is deleted without three days' warning, also when it's turned on or shortened. `POST /_shroom/api/cleanup` gains `deletesAt` per item, which the Library, the popover and the command read.
- **The delete.** Each scheduled run deletes at most one video whose `deletesAt` has passed, after reading it fresh. It skips the video when:
  - it's pinned (`keep`);
  - it isn't live, or its upload hasn't finished (no `drained`/`bytes`);
  - a fresh "being edited" mark sits beside it: `.shroom/editing/<id>.json` = `{until}`, written by the Mac with the editor open (`POST /_shroom/api/editing`, device key, maker or owner) every 5 minutes and good for 15;
  - its play record shows a play since `warnedAt`.

  A skipped video is looked at again on the next run. The delete goes exactly as a Delete does: the tombstone first, then the bytes under `rec/<id>/` (paged, 1,000 keys a call), then the seal (`gone/<id>`, B125). The record gains `by: "auto"` so the Macs can say it once. **Timing:** on the date, or within a day of it.
- **The Macs** keep only the warning, Pin, Review…, the popover banner and the line once after a delete.

**Cloudflare's limits (Workers limits page, read 2026-10-24).** Free plan: 5 Cron Triggers per account (Paid 250); 10 ms CPU per scheduled run; 50 subrequests per run, R2 binding calls included; 15 minutes of wall time per scheduled run. One shared trigger uses one of the 5. One delete a run is about 5–10 R2 calls, inside 50. The daily walk goes a page at a time to stay inside 10 ms. The page doesn't say whether "Cron Triggers" counts expressions or Workers; one expression on one Worker is safe either way.

**Rejected:**
- **The token Mac's cleanup sweep** (B91, every 6 h of awake time), the first draw. A deletion only happens while that Mac is open and awake, so a closed laptop misses it and the space keeps paying for videos the person asked to have gone. A member's videos would wait on the owner's Mac too.
- **A second Cron Trigger for auto-cleanup alone.** It spends another of the free account's five, for no gain.
- **Scanning every record each minute.** It goes past 10 ms of CPU and 50 subrequests on a real space.
- **R2 lifecycle rules.** They expire by object age, not by the last watch, and can't skip a pinned video.

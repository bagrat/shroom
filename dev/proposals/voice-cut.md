# Editing: a trim timeline, markers, and “shroom cut”

*Proposal and POC, 2026-10-18. Draft PR, not for merging: the feature, its words and its
spec rows are Bagrat's call. The words on the boards are placeholders.*

> **Built, 2026-10-18** (Bagrat: build everything now, decide the release later; relayed
> by the orchestrator). The voice cuts are spec N65 (provisional) and DECISIONS
> [B200](../../docs/DECISIONS.md) (the owner page) and B201 (the Mac). Where the build
> parts from what follows:
>
> - **Reviewed on the owner page, not in a window after Stop.** An owner-only strip under
>   the player lists each cut with Put back, and an offered line with Cut it too
>   (`OwnerPageCuts`, `OwnerPageCutRows` on the canvas).
> - **No switch of its own.** The space's listening (N66, the AI build) is the one switch.
> - **The words are the space's**, heard as the take uploads (N66's transcript). The Mac
>   that made the take plans the cues; nothing new goes up.
> - **The quiet is read in Rust** from the take's own sound: the bundled encoder has no
>   `silencedetect`. It matches ffmpeg's on the same speech to a constant 21 ms.
> - **The cuts land in the editor's list** (N63's `meta.edits`): ids `v1`…, the plan's mark
>   set once, so a cut put back stays put back.
> - **No Clef gate yet.** The rules alone made no wrong cut; full Clef stays the option
>   measured below.
> - The trim timeline and markers are the editor's build (N63, N64), not this PR's.

> **Working rule** (Bagrat, 2026-10-18, relayed by the orchestrator). These POCs run on
> best judgment, without stopping to ask, and may use the Cloudflare token for POC work. The
> calls below are written as “decided X because Y” for him to overturn.

**Asked** (Bagrat, 2026-10-18, relayed by the orchestrator). Three things, in order:

1. A plain timeline editor: drag to trim the ends, cut a part out. Probably nothing more.
2. Markers dropped while recording, as the POC once had.
3. A spoken cue. You slip, say a code word such as “shroom cut”, and say the line again.
   shroom finds the cue in the transcript, works out where the retake starts by how much of
   what you said after the cue repeats what you said before it, and cuts the slip
   automatically. He'd like this one ready for launch day (Tue 27 Oct) if it can be.

**In short.** The cue works. A pure-Rust planner (`crates/shroom-edit`) takes a transcript
with word times and returns an edit list. The cloud harness speaks seven invented takes with
four stock voices, transcribes them with Whisper and plans them, then scores the cuts
against the script.

- **A real voice passed.** Bagrat tried the tester (below) with his own voice on 2026-10-18,
  and the auto cut worked.
- **On real Workers AI**, through a Worker deployed with a shroom-shaped token on the test
  account, **27 of 28 takes passed**, both with and without the code word in Whisper's
  prompt. **No run cut a word that should stay.**
- **Ten minutes in 60 s chunks** (52 cues) got 49 of 52 cues cut, and again nothing that
  should stay.
- **The two blockers turned out not to be blockers.** The setup token's existing powers can
  deploy a Worker that calls Workers AI, and one request took 10 minutes of audio.
- **Cuts live on an edit list.** Nothing is re-encoded, and any cut can be undone.
- **A working editor prototype** plays the takes with their cuts applied.

## Boards

On the [shroom UI canvas](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg), in a new
column, **Editing (a proposal)**:

- **“Editor — voice cuts”** ([`EditorVoiceCuts.png`](voice-cut/EditorVoiceCuts.png)). The
  editor after a take with three cues:
  - Two slips are cut. They show red and hatched on the timeline and as cards listing what
    went and what plays next.
  - One cue had no retake after it. Only “shroom cut” went, and the line before it is a
    dashed amber suggestion with “Cut it too”.
  - A cut made by hand sits in the same list.
  - Every card has Undo.
- **“Editor — trim”** ([`EditorTrim.png`](voice-cut/EditorTrim.png)). Yellow ends trim the
  take. Dragging across the sound selects a part, and “Cut 12.0 s” removes it. Markers show
  as flags. The footer reads “Your link stays the same.”
- **“Editor — the ways in”** ([`EditorDoors.png`](voice-cut/EditorDoors.png)). Four places:
  - **Edit…** in a Library row's menu.
  - **Drop a marker** back in the paused popover. B143 hid it “until the editor ships”, and
    the core still has it.
  - A line under “Link copied” after Finish & Share: “Cut 2 slips where you said ‘shroom
    cut’. [Review]”.
  - One switch in Settings › General.

The boards come from [`voice-cut/make-boards.py`](voice-cut/make-boards.py).

## The POC

### What was run

- **Takes.** Seven invented scripts ([`poc/scenarios.json`](voice-cut/poc/scenarios.json)),
  spoken by four stock [Piper](https://github.com/rhasspy/piper) voices: two US, one GB, one
  US female. No one's own voice was cloned.
- **Pipeline** ([`poc/run.py`](voice-cut/poc/run.py)), for each take:
  1. Transcribe with Whisper (`faster-whisper`, word timestamps).
  2. Find the silences (ffmpeg `silencedetect`).
  3. Plan with `shroom-cut`.
  4. Score the cuts against the script.
  5. Render a before and an after video and a picture of the cuts.
- **The scripts:**
  1. The brief's own example: “Hello, that is shroom. Shroom cut. Hello, this is shroom.”
  2. A slip in the middle of a paragraph, retaken with “So, then you…”.
  3. A slip in the second half of a sentence. The retake starts at that half, not at the
     sentence start.
  4. A cue with no retake after it.
  5. Two slips in a row, each followed by a cue.
  6. “cut” and “shroom cut” said as ordinary speech. Nothing should be cut.
  7. A fluffed sign-off that's retaken, then a cue as the very last words.
- **Passing** means three things:
  - Every cue is cut.
  - No kept or merely mentioned word is cut.
  - Every slip is cut, or offered when no retake followed it.

| Transcriber | Prompt | Passed |
|---|---|---|
| **Workers AI** `@cf/openai/whisper-large-v3-turbo` (real, the test account) | “Shroom cut.” plus a disfluent sample | **27/28** |
| **Workers AI**, the same | none | **27/28** |
| Whisper large-v3-turbo, local (int8, CPU) | the prompt | 27/28 |
| Whisper small.en, local | the prompt | 27/28 |
| Whisper small.en, local | none | 19/28: the cue is heard as “Trume cut”, “Trumcut”, “Trung cut” or “drum cut” |

**The misses.** None of them cut a kept word.

- **Workers AI wrote one line the way it was meant.** It gave “Then you pick the window
  you want to share.” and left out both the slip and the cue. Nothing was heard, so nothing
  was cut. This is the tidying the research warned about. It hit 1 take in 28.
- **Local turbo left the first 8 s out of one transcript**: the slip, the first cue and
  the retake.
- **On one small.en take, Whisper wrote “hmm” as “him.”** The offered line before a cue
  with no retake then started one clause late.

The fixtures keep each miss as a known one, with its reason
(`crates/shroom-edit/tests/takes.rs`). The test holds the other 81 transcripts to their
scripts.

**Ten minutes at once.** All the takes were run together into a 10-minute take with 52
cues, then transcribed by Workers AI:

| How it was sent | Cues cut | Slips cut or offered | Kept lines cut |
|---|---|---|---|
| One request, 10 min (19 MB WAV, or 2.4 MB MP3; 16–23 s) | 45/52 | 46/52 | **0/98** |
| 60 s chunks with 2 s overlap | **49/52** | **49/52** | **0/98** |

- On long audio, Whisper's word times drift by a few tenths of a second. A silence near a
  gap now counts even if the word times say it's slightly off.
- The test `ten_minutes` holds both fixtures to a floor.

**What the runs taught:**

- The code word needs biasing. Whisper's `initial_prompt` does it, and Cloudflare's Whisper
  takes the same option. Deepgram takes `keyterm`.
  - Even with the prompt, one voice came out as “Troom cut”.
  - So a looser sound-alike (“troom”, “drum”, “trung”) also counts, but only when it stands
    as its own sentence: after a sentence end, and followed by a full stop or a pause.
    “The drum cut was loud” stays speech.
- **The prompt has a cost**, and Whisper sometimes skips speech. Across the runs, two takes
  lost their first 8–10 s: one on small.en in an earlier run, one on local turbo.
  - The planner now refuses to treat a long gap with sound in it as silence, so that take
    lost only the cue, not the missed speech.
  - The real fix is to transcribe in chunks of about 60 s with a little overlap. On the
    10-minute take, chunks beat one request: 49 cues cut against 45.
  - The app could also flag a take whose transcript has a long stretch of sound with no
    words in it.
- **Whisper mostly kept the repeated words and the fillers** (“the, the window”, “um”). It
  tidied one take in 28 on Workers AI. These are clean synthetic voices. The first real one
  (below) came out cut right too.
- Two bugs came out of the runs:
  - A sentence ending in “called.” made the cue look like a mention.
  - “uh” transcribed as “the”, as in “one more thing, the shroom cut.”, did the same.
  Both are fixed, and the unit tests now cover them.

### How a cue becomes a cut

`crates/shroom-edit` (pure Rust, ~500 lines, serde only):

1. **Find the cue** (`cue.rs`). “shroom” is matched by sound, so “Shroom”, “Schroom”,
   “shrewm”, “shrooms”, “Shroom-cut” and “shroomcut” all count. It must be followed by
   “cut”. A *mention* is skipped: “you say shroom cut and…”, “the shroom cut feature”. The
   exception is a cue that stands as its own sentence or is followed by a pause.
2. **Find the restart** (`align.rs`).
   - The words after the cue are aligned against every possible starting point among the
     words before it. Both starts are pinned, both ends are free, and near-miss words count
     (“recording”/“recordings”).
   - The highest score wins. On a tie, the latest start wins, because people retake the line
     they just fluffed.
   - A leading “so”, “okay” or “um” in the retake is allowed. “so” and “okay” stay in the
     video; “um” and “uh” go with the cue.
3. **Plan the cut** (`plan.rs`), in three cases:
   - **A retake was found.** The cut runs from the restart through the end of the cue.
     Each edge is snapped into the detected silence beside it, keeping up to 0.12 s of quiet
     each side, so the join sounds natural. It is *applied*, at high confidence, when it
     meets all of these:
     - at least 3 words are said again and they cover half of the slip, or at least 2 words
       cover two-thirds of it;
     - the cue was said apart from the speech;
     - the slip is under 40 s.
     A weaker match is offered, not applied.
   - **No retake, but the cue was said apart** (a pause before or after it). Only the cue
     goes. The line before it, back to a sentence end or a 0.6 s pause, is offered unapplied.
   - **Neither.** It's speech, and nothing happens.
4. **Several cues.** Each cue only looks back as far as the previous cue, so two slips in a
   row each get their own cut, and the clean third take stays.

Run it: `cargo test -p shroom-edit`. The binary `shroom-cut` turns a transcript (JSON on
stdin) into the edit list (stdout).

### The edit list

```json
{ "version": 1, "source_duration": 13.1, "in": 0.0, "out": 13.1,
  "cuts": [ { "start": 3.87, "end": 7.82, "kind": "retake", "confidence": "high", "applied": true,
              "removed": "Then you pick the, the window you um, shroom cut.",
              "resumes": "So, then you pick the window" } ] }
```

- Times are in seconds on the recording's own timeline (the joined takes).
- `in`/`out` is the trim.
- `kind` is one of `retake`, `cue-only`, `line-before-cue` or `manual`. Hand cuts use the
  same list.
- **Undo means `applied: false`.** Revert to original means no applied cuts and the full
  trim.
- What plays is `keep()`: the trim minus the applied cuts, with overlapping cuts merged.

### The prototype

The prototype is [`prototype/index.html`](voice-cut/prototype/index.html), and
[`prototype/build.py`](voice-cut/prototype/build.py) packs a run into it.
- It plays each take with its edit list applied as it plays, skipping the red parts.
- It shows the transcript with the cut words struck through.
- Each cut can be undone, and each suggestion accepted.
- You can drag the ends to trim, or drag across the sound to cut a part.
- An Edited / Original switch, and the live edit list underneath.

Bagrat has it as a private page, and the link is in the report to him. A before-and-after
video of four takes sits beside it (also private).

## Trying it with a real voice

**Now, with nothing to install: the tester** ([`voice-cut/tester/`](voice-cut/tester/README.md)).

- It's a private page on the Cloudflare test account, behind a throwaway key, and it works
  from a phone or a laptop.
- Record (or pick an audio or video file), slip, say “shroom cut”, say the line again, and
  press Stop.
- Whisper on Workers AI hears it, and the real planner runs in the page as WebAssembly.
- The page shows the transcript with the cuts struck through and the cuts themselves, and
  plays the take cut.
- Checked in headless Chromium, both ways:
  - an uploaded take: one cut, the right one;
  - a recording through a fake microphone: two cuts, the right ones. This one turned up a
    planner bug, below.

**The real-voice probe passed** (Bagrat, 2026-10-18). He tried the tester with his own
voice, and the auto cut worked.

- The tester stores nothing, so there's no transcript to look at.
- Cloudflare's analytics for its Worker show that hour's whole traffic:
  - 16 requests and no errors;
  - 5 calls to Workers AI.
- 12 of those requests, and 4 of the AI calls, were the headless checks above. So about one
  take of his went through, and it went cleanly.

**The full path, recording in the app to cuts on the share page, can't be tested yet.**
The app, the Worker and the page don't do any of this until the integration is built
(about 7.5 days, above). Then:

- the hosted Mac runs the app side, recording a take with a spoken “shroom cut” played in
  through its audio loopback, as the system-audio scenario does today (B172);
- CI and the cloud fakes run the Worker and the page;
- a Tart VM on Bagrat's Mac stays the last resort, and isn't needed for this.

**The bug the tester found.** A line said correctly earlier in the take, then slipped and
retaken, made the aligner pick the earlier, exact repeat. That made the “slip” the whole
stretch in between, so it was too long to cut. Each candidate restart now pays for the
words between it and the cue that the retake doesn't say again. A unit test holds this,
and every fixture still passes.

## How editing fits the link and the storage

What's true today (ARCHITECTURE §4, §5, §14; WORKER-INTERFACE):

- The take is HLS fMP4 in R2, at `rec/<id>/v0/…`, with 6 s segments.
- `video.mp4` is built after go-live and serves both download and playback.
- The link `v.<name>.workers.dev/<id>` never changes. Meta only grows.
- `meta.markers` already exists and nothing shows it. `captions` and `rec/<id>/tx/` are
  reserved.

**Recommended: two steps, the link untouched.**

1. **Applied as it plays (the launch step).**
   - The app saves the edit list (above) in the recording's record, as `meta.edits`
     (below: the editor build's choice). This is additive: an old page ignores it.
   - The page's player skips cut ranges, holds to the trim, and shows the edited length and
     edited time. It works the same over HLS and over `video.mp4`.
   - Saving and undo are instant and cost nothing. The prototype is exactly this.
   - **Costs:**
     - The cut parts are still in R2, so someone who reads the playlist could hear the slip.
     - `/d/<id>` would still download the uncut file.
     - The poster and the 15 s teaser may show a cut moment.
2. **Published (the follow-up, or at launch if time allows).**
   - The app renders the edit locally from the take it still holds. This is ffmpeg: copy
     what lies between keyframes, re-encode only around each cut, and re-cut `video.mp4`.
   - It uploads the result under a new prefix, `rec/<id>/v1/`, and the new `video.mp4`, then
     flips `meta.media`. The page follows meta, so the link is the same and nothing has to
     reload by hand.
   - The old prefix is deleted afterwards.
   - Undo after publishing re-renders from the local take, which shroom keeps (ARCHITECTURE
     §16 #37: local pruning is still open).

The Worker gains only reading the edits and the skip in its player.

**As the editor build does it** (`claude/editor`, 2026-10-18): the edit list goes inline in
the record, as `meta.edits`, not in a separate `edits.json`.

- The Worker reads the record on every view anyway, so this saves a read and leaves no
  second object to keep consistent.
- The page gets only the computed keep ranges, so a voice cut's `removed` text (what was
  said in the slip) never reaches a viewer.
- Saving goes through the same read-modify-write under If-Match as every other write to
  the record (ARCHITECTURE §5).
- This proposal follows that. The app does the
render; it already owns ffmpeg and the takes.

**Markers** land in the same file (`"markers": [seconds]`) as flags on the timeline, and
playback can jump to them. B143's hidden **Drop a marker** comes back while paused, and
⌥⌘M drops one while recording (call 7 below).

## Where the words are heard

The app needs word times for one take. The options, from research (sources in the PR
thread):

| Option | Word times | Repeats kept | Cost | macOS | New prompt | Effort |
|---|---|---|---|---|---|---|
| **Workers AI whisper-large-v3-turbo**, from the person's own Worker | yes (`segments[].words`): **probed** | 27 of 28 takes | ~$0.0005/min; ~214 free min/day | any | none | low: one Worker route plus chunking |
| Workers AI Deepgram nova-3 | yes, with confidence | yes (`filler_words`), `keyterm` biasing | ~$0.005/min; ~21 free min/day | any | none | low |
| Apple SpeechAnalyzer (on device) | yes (`audioTimeRange`) | unknown | free | **26+** (the floor is 14) | likely none; to probe | medium: a Swift shim |
| whisper.cpp via `whisper-rs` (on device) | yes (DTW) | as Whisper | free; a 150 MB–1.6 GB model to download | 14 | none | medium: a C++ build and a model download |
| SFSpeechRecognizer | yes, flaky on long files | unknown | free | 14 | **yes**, the Speech TCC prompt | medium |

**Recommendation: Workers AI whisper-large-v3-turbo**, from the person's own Worker.

- It fits BYO: it runs in their account, under their free allowance, with no model download
  and no new macOS prompt. It is the model behind the 27/28 above.
- **How it would work.** After Finish & Share, the app extracts the take's audio (MP3 at
  32 kb/s, 60 s chunks with 2 s of overlap). It posts each chunk to a new Worker route,
  `POST /_shroom/listen/<id>`, under the app's existing key. The Worker runs
  `env.AI.run('@cf/openai/whisper-large-v3-turbo', {audio, initial_prompt, vad_filter:
  true, condition_on_previous_text: false})` and returns the words.
- A 5-minute take is five chunks, about 3 s each, and they can run at the same time. The
  cuts are normally in place before anyone opens the link.
- **Probed, 2026-10-18,** on the dedicated test account, with its token, which has a real
  setup token's powers (Workers Scripts Edit + R2 Edit):
  - **That token can't call Workers AI's REST API** (“Authentication error”).
  - **But it can deploy a Worker with an `ai` binding**, and that Worker can call Whisper.
    So the setup wizard and the token need no change: the app already deploys the Worker.
  - **There is no size wall through the binding.** 1, 3, 5 and 10 minutes all came back,
    as WAV (up to 19 MB) or MP3 (up to 2.4 MB). Time: 3–6 s for a minute, 16–23 s for ten.
  - **`segments[].words` was filled every time**, with `start` and `end` in seconds.
  - **Cost.** The whole probe used roughly 2,000 neurons, within the free 10,000 a day.
  - **What was created:** one Worker, `poc-voice-cut-ai`, guarded by a throwaway key,
    deleted after the probe (the API answers 404 for it now). Nothing else was touched.
- SpeechAnalyzer stays the on-device option for macOS 26 and later.
- **Reading the audio from the take itself.** The sibling Cloudflare AI POC (PR #60)
  already pulls only the audio out of a take's fMP4 pieces in R2, with ranged reads. Its
  `fmp4.js` does that; its `windows()` and `hear()` turn the pieces into ~60 s Whisper
  windows and give the words on the take's own clock, with none doubled at a window's edge.
  - **Decided:** the listen route reuses that code, and the Worker reads the pieces it
    already stores. The app then uploads nothing new; it only asks the Worker to listen.
    This replaces “the app extracts and posts chunks” above.

### A second opinion on each cue: Clef

Cloudflare's decision model, Clef, answers typed questions about a text with a
probability. It was asked whether each candidate is “the speaker telling the editor to cut
out a mistake, rather than talking about the phrase, a feature, or cutting something else”.

- **The candidates** were all 133 places in the fixtures where the code word, or a
  sound-alike, met “cut”, labelled by the script. To those were added 15 hand-written
  sentences, such as “just say shroom cut and…”, “Shroom, cut the long silence…” and “drum
  cut loops”.
- **The state** explained the convention in two sentences
  ([`poc/clef.py`](voice-cut/poc/clef.py)).

| Model | Cues (99): lowest | Non-cues (34): highest | At 0.3: cues kept / non-cues through | Time |
|---|---|---|---|---|
| **`@cf/cloudflare/clef`** (27B) | **0.20** (median 0.83) | **0.08** | **98/99 · 0/34** | 2.5 s for all, 32 a call |
| `@cf/cloudflare/clef-flash` (9B) | 0.04 (median 0.15) | 0.91 | 15/99 · 2/34 | 5 s; one per call didn't help (46/99 · 9/34) |

- **Decided: Clef, not Clef-flash, as a gate on each cue the rules find.** A cue counts
  when Clef gives it at least 0.15, which separates every case above.
- **Why a gate and not a replacement.** The rules (sound match, mention words, the pause,
  the retake alignment) already got every fixture right. Clef covers phrasings the rules
  have never seen, like “We added shroom cut in the last release”.
- **The cost** is one call per take, about 30 neurons, a fraction of a cent.
- **What Clef can't fix.** It reads text, so it can't help when Whisper wrote the line as
  meant and dropped the cue. Clef-omni hears audio and might; it wasn't tried.

## Spec sketch (provisional, not in `machine.json` yet)

The editor and markers are now being built in their own session (branch `claude/editor`),
which holds **N63 (the editor)** and **N64 (markers)**, plus decision rows B195–B199, and
takes this crate's edit list unchanged. N62 is held elsewhere. What follows is this
proposal's own row, whose decision rows would start at B200, and its sketch of those two
as first written here, for the record.

- **N65 Voice cuts** (provisional; this proposal).
  - Settings › General gets a switch, off by default (call 3 below). Copy is for Bagrat.
  - After Finish & Share, the take is transcribed in the person's own Cloudflare account.
  - Each cue becomes a cut, applied or offered as above.
  - When cues were heard, the stop popover adds a line with Review, which opens the editor.
  - New states: `editor`, `editor-saving`.
  - New edges: `library → editor` (Edit…), `stopped → editor` (Review), `editor → library`
    (Done).
  - Amends `absent_from_v1` (“captions and transcript search (P8)”: the transcript is used,
    not shown) and BUILD-STATUS item 9 (B168: no transcription in v1).
- **The editor** (now N63, `claude/editor`), as sketched here.
  - Edit… in a Library row's menu opens an `editor` window.
  - It holds a preview, a timeline with trim ends, drag-to-cut, a list of cuts, Undo per cut,
    Revert to original, and Done.
  - Edits are an edit list beside the recording, applied as it plays. The link never
    changes.
  - Amends `absent_from_v1` (“trim, the editor, marker surfacing (P11)”) and `library`
    (a fifth row action).
- **Markers surface** (now N64, `claude/editor`), as sketched here.
  - Drop a marker comes back in `paused`, reversing B143's hiding.
  - ⌥⌘M drops a marker in `recording`, held only while a take is open, like N54's ⌥⌘P.
    Amends `absent_from_v1`'s global hotkeys.
  - Markers show as flags in the editor.
- **Laws touched.**
  - The Worker gains a route that calls AI on the person's own account, a new power for
    WORKER-INTERFACE §3.
  - The privacy page's facts change: the voice is transcribed in *your* Cloudflare account.

## Effort to launch day (Tue 27 Oct)

Today is Sun 18 Oct, which leaves 7 working days. The estimates are honest, for one session,
with tests:

| Piece | Days |
|---|---|
| Probes: AI binding vs the token, size limit, a real human take (**all done**) | 0 |
| Worker: `/_shroom/listen/<id>`, the `ai` binding at deploy, voice cuts into `meta.edits`, cloudfake twin | 1.5 |
| App: ask the Worker to listen after stop, merge the words, plan, save into `meta.edits` | 1 |
| `shroom-edit` hardening plus spec tests from real takes | 1 |
| Page player: skip cuts, edited time and length, the trim | 1 |
| Re-cut `video.mp4` and the teaser so download and preview match | 1 |
| A review list: the stop popover's line, a cuts window with Undo and Play (no timeline) | 1.5 |
| Spec rows, boards approved, hosted-Mac pass | 0.5 |
| **Voice cuts at launch, playback-applied, with a review list** | **~7.5** |
| The full timeline editor (trim, drag-to-cut, preview) | +3–4 |
| Publishing the edit (new HLS prefix, delete the cut parts) | +1.5–2 |

**My read.**
- **Voice cuts for launch is possible but tight.**
  - The probes are clean, so the platform risk is gone.
  - It needs Bagrat's yes by about Tue 20 Oct and two sessions working in parallel: Worker
    and page in one, the app in the other.
  - Ship it opt-in, labelled new, with the cuts list as the review.
- **The timeline editor and publishing the edit are realistic the week after launch.**
- The prototype here is close to the editor's front end and would cut the UI time.
- **The real-voice risk is down, not gone.** One take of Bagrat's own voice, on one device,
  cut right (2026-10-18). Before it's on by default, a few more takes are worth it: another
  mic, a noisy room, a longer take, another accent. The tester stays up for that.

## Calls made, for Bagrat to overturn

Under the working rule above, each is decided, with why:

1. **The code word: kept “shroom cut”,** because it worked on real Workers AI without a
   prompt (27/28). It's two words and rarely said. Its loose sound-alikes (“Troom cut”) are
   caught only when said alone. Alternatives, if he wants one: “shroom redo”, “cut cut”. The
   word is product copy, so it's still his.
2. **Confident cuts are applied with Undo; unsure ones are offered.** Across 140 transcripts
   (28 takes × 5 runs) and both 10-minute takes, no applied cut touched a kept line.
   Offering everything would add a review step to every take for little safety.
3. **Off by default for launch, opt-in from Settings.** It sends the voice to the person's
   own Cloudflare account, and the privacy page's facts must change first. A real voice has
   now been through it once (2026-10-18). Turn it on by default after a few more takes,
   once the privacy page says so.
4. **Launch scope: voice cuts with a review list by the 27th; the timeline editor the week
   after.** This is the 7.5 days above. The editor adds 3–4 days and isn't needed for the
   differentiator.
5. **Playback-skip at launch, publishing the edit right after.** Cut parts stay in R2 until
   published, and the review window says so. Publishing is 1.5–2 days and protects the
   download and the teaser.
6. **A real take: done, and it passed.** Bagrat tried the tester with his own voice
   (2026-10-18) and the auto cut worked. More takes would still help: other mics, noise,
   accents, longer takes. The tester stays up for them.
7. **Markers: Drop a marker comes back while paused, and ⌥⌘M drops one while
   recording.** Bagrat asked for markers dropped *during* recording. A marker that only
   drops while paused sits on a take boundary, which says nothing a pause doesn't. ⌥⌘M sits
   beside N54's ⌥⌘P and is held only while a take is open, the same way.
   - The core's `DropMarker` and the recorder's `Marker` already exist (B143 kept them).
   - Rejected: no shortcut, waiting for demand. That leaves markers worth little.

## Running the POC

```sh
pip install piper-tts faster-whisper pillow
python3 -m piper.download_voices en_US-ryan-medium   # into a voices dir; any stock voice
cargo build -p shroom-edit --release
python3 dev/proposals/voice-cut/poc/run.py --voices <dir> --out <dir> --render [--model large-v3-turbo]
WAI_URL=… WAI_KEY=… python3 dev/proposals/voice-cut/poc/run.py --voices <dir> --out <dir> --model workers-ai
#   (a Worker with an `ai` binding that runs Whisper on the posted body: see the doc's probe)
python3 dev/proposals/voice-cut/poc/run.py --voices <dir> --out <dir> --wav my-take.m4a --render   # a real take
python3 dev/proposals/voice-cut/prototype/build.py <out>/<voice> <site dir>   # then serve <site dir> with prototype/index.html
```

Each take's folder holds:
- `take.wav`;
- `transcript.json`;
- `edit-list.json`;
- `strip.png`: the sound, the script's slips and cues, and the cuts;
- `before.mp4` and `after.mp4`, with the source time burned in, so every jump in the after
  video is visible.

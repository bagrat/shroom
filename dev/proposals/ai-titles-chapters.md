# AI titles and chapters, through the owner's own Cloudflare

*Proposal and POC, 2026-10-18; **built the same day** (spec N66, N67, provisional;
DECISIONS B205–B209; §10), off by default until Bagrat's release call. Nothing here is decided. It follows B171 (AI titles and
chapters after v1, boards first, following B168) and B170 (the pasted key stays). The
product shape, the words and the spec rows are Bagrat's call. The POC is
[`ai-titles-chapters/poc/`](ai-titles-chapters/poc/); the boards are on the canvas
(§8).*

## In short

1. **It works, live, end to end.** A Worker route reads a take's pieces from the bucket,
   transcribes them with Whisper, and writes a title and chapters beside the take, all on
   the owner's own account. It ran on the test account against three takes: two invented
   ones, and the public 4-minute setup video on shroom.am (real voice, with music under
   it). Examples are in §4.
2. **Cost: about 4¢ per recorded hour on Cloudflare's paid plan**, measured on a real
   36-minute recording and confirmed by Cloudflare's own meter. The free allowance covers
   about **2.7 hours of video a day** per space, shared by everyone in it. The transcript
   is ~85% of the cost, naming the rest; captions add nothing (§5).
3. **Speed: the title and chapters land about 10–20 s after Stop**, if the take is
   transcribed while it records (each ~60 s window as its pieces upload). A first title
   can show before Stop.
4. **The "System 1" models are real.** They are decision models: they answer typed
   questions (yes/no, pick one, a score) with probabilities, in one fast pass, and write
   no text. **Jev** (TypeSafe AI, mid-September 2026) started the class. **Cloudflare's
   Clef** (1 October 2026) is open-weight and runs on Workers AI. There are three
   models: `clef`, `clef-flash`, and `clef-omni`, which also listens to audio and watches
   video. **They can't write a title.** As chapter-cut finders they were fast (0.5 s) and
   cheap, but **less accurate than a small LLM on real speech** (§4.3). They pay off for
   shroom where the answer is a choice: the cleanup suggestions, voice cues, "is anyone
   talking?", and checks on a title. Jev is a third-party API, so it is out (BYO).
5. **Recommendation:** Whisper turbo (`@cf/openai/whisper-large-v3-turbo`) to hear, and
   **`@cf/openai/gpt-oss-120b`** to name, with `@cf/google/gemma-4-26b-a4b-it` as the
   fallback. Hard rules sit on top in code: chapters snap to sentence starts, are ≥ 20 s
   long, start at 0:00, and appear only on videos of 90 s or more. A title someone types
   always wins. Chapters live **in the seek bar**, split into one section per chapter
   and named in a tip on hover, as Bagrat chose on 2026-10-18 (§9.1).
6. **Nothing new on Cloudflare's side** (B171, live check 2). Turning it on redeploys the
   space's Worker with an `ai` binding. Members hold no AI key and never see a cost.

## 1. What Cloudflare offers (read 2026-10-18)

### 1.1 Workers AI, and the free day

- **One price unit, the neuron.** Every account gets **10,000 free neurons a day**, on the
  free and the paid Workers plan alike. They reset at 00:00 UTC. On the free plan, calls
  past that fail until the reset. On the paid plan ($5 a month) they cost **$0.011 per
  1,000** ([pricing](https://developers.cloudflare.com/workers-ai/platform/pricing/)).
- **Called from the space's Worker** through an `ai` binding (`env.AI.run(model, input)`),
  or by REST at `/accounts/{id}/ai/run/{model}`. The binding needs no token. B171's live
  check 2 showed that today's two-power key deploys a Worker with that binding.
- **Rate limits** ([limits](https://developers.cloudflare.com/workers-ai/platform/limits/)):
  720 requests a minute for speech recognition and 300 for text generation, per account.
  A space hearing a 60 s window per take is far below either.

### 1.2 Transcription

| Model | $ / audio min | Neurons / min | What it gives |
|---|---|---|---|
| `@cf/openai/whisper-large-v3-turbo` | 0.0005 | 46.63 | text, **segments with per-word start/end**, WebVTT, detected language; `initial_prompt` (vocabulary), `language`, `task: translate` (to English), VAD; audio as base64 |
| `@cf/openai/whisper` | 0.0005 | 41.14 | text, words, VTT (older, weaker) |
| `@cf/deepgram/nova-3` | 0.0052 | 472.73 | Deepgram's model: smart format, diarization, key terms, paragraphs, utterances. 10× the price |
| `@cf/deepgram/flux` (WebSocket) | 0.0077 | 700 | streaming, turn detection (voice agents) |

**Word timestamps are there in Whisper turbo** (`segments[].words[]`, `{word, start,
end}`), confirmed live. That is what the voice-cue editor needs (§2).

### 1.3 Text models that name

Measured on the same takes (§4). Prices are per million tokens, in and out.

| Model | $ in / out | Naming a 4-min take | Took | Verdict |
|---|---|---|---|---|
| `@cf/openai/gpt-oss-120b` | 0.35 / 0.75 | 114 neurons | 7–18 s | best titles in 3 of 3; balanced chapters |
| `@cf/openai/gpt-oss-20b` | 0.20 / 0.30 | 36–55 | 6–10 s | good; chapters a little fine-grained |
| `@cf/google/gemma-4-26b-a4b-it` | 0.10 / 0.30 | 14 | 3 s | good; fewer, broader chapters |
| `@cf/zai-org/glm-4.7-flash` | 0.06 / 0.40 | 13 | 8 s | good chapters; writes in lower case |
| `@cf/meta/llama-3.1-8b-instruct-fp8-fast` | 0.045 / 0.38 | 16 | 2 s | Title Case, splits too finely |
| `@cf/meta/llama-3.2-3b-instruct` | 0.051 / 0.34 | 14 | 1.2 s | Title Case, repeats chapters |
| `@cf/ibm-granite/granite-4.0-h-micro` | 0.017 / 0.11 | 3.5 | 5–11 s | generic titles ("App Updates Overview") |
| `@cf/qwen/qwen3-30b-a3b-fp8` | 0.051 / 0.34 | — | — | puts its answer in its reasoning field; not used |

Thinking models (gpt-oss, glm, gemma-4, qwen3) spend output tokens on reasoning before
they answer. The POC allows 4,000 output tokens and turns thinking off where the model
takes `chat_template_kwargs`.

### 1.4 The "System 1" class: decision models

- **What they are.** You give one *state* (text; for Clef, images; for Clef-omni, audio
  and video too) and up to 64 typed *questions*. Each question is a `noul` (the chance
  that a statement is true), a `choice` (one of 2–255 named options, each with its
  probability) or a `score` (an ordered rubric of 2–10 levels). The model reads once
  and scores every answer in parallel. It writes no text, so **output is free** and the
  latency doesn't grow with the answer. The API shape is called the System One API.
- **Jev**, by **TypeSafe AI**, launched 15 September 2026. It is text only, hosted by
  TypeSafe (and on OpenRouter), at $0.042 per million input tokens. Its architecture is
  undisclosed. Using it would make owners sign up with a third party, which breaks BYO,
  so it is out for shroom.
- **Clef**, by **Cloudflare**, launched 1 October 2026
  ([changelog](https://developers.cloudflare.com/changelog/post/2026-10-01-clef-workers-ai/)).
  The weights are Apache-2.0, built on frozen Qwen backbones; the training data is not
  public. Its API is compatible with Jev's. Context is 64K tokens. Prices are from the
  pricing page:

  | Model | Size | $ / M input | Notes |
  |---|---|---|---|
  | `@cf/cloudflare/clef` | 27B | 0.24 | the most accurate; images (≤ 4) |
  | `@cf/cloudflare/clef-flash` | 9B | 0.038 | median 39 ms in Cloudflare's own figures; **0.5–0.8 s measured** through the Worker with 60 questions |
  | `@cf/cloudflare/clef-omni` | 30B MoE, 3B active | 0.15 | **audio** (≤ 4 clips of ≤ 300 s, ~780 tokens a minute), **video** (≤ 2 clips of ≤ 60 s), images |

  Secondary reports gave other numbers for Clef-flash ($0.09). The pricing page is the
  source used here. The benchmarks are Cloudflare's own and not yet reproduced
  independently.
- **What they're good for in shroom:** any per-sentence or per-clip judgment, made in
  bulk. Clef-omni could answer "is anyone talking in this clip?" from the audio alone,
  for about $0.0001 a minute.

### 1.5 AI Gateway

AI Gateway is a proxy in front of the AI calls: logs, caching, rate limits, retries and
model fallback, analytics with cost. It's free on every plan. A Worker uses it by adding
`{ gateway: { id } }` to `env.AI.run`. **Not needed for v-next**, and making a gateway
would need a new power on the owner's key. It is worth a later look for one thing: a
per-space cost view the owner could open in Cloudflare.

### 1.6 What stays BYO

Everything above runs **in the space's own Worker, on the owner's own account**. Whisper,
the naming model and Clef are Workers AI models, billed to the owner. Nothing runs on
shroom.am, and members hold nothing. Jev, OpenAI and the like are outside: they'd need
an account with a third party.

### 1.7 Long videos and the Worker's limits

- **Whisper:** there's no documented length cap, but Cloudflare's own tutorial splits
  long audio into ~1 MB pieces. The POC sends **~60 s windows** (≈ 1 MB of AAC). Each
  window, apart from the first, starts with one extra piece of the window before it, so
  a word cut at a window's edge is heard whole once. The words are then merged on the
  take's own clock.
- **Workers' limits** ([limits](https://developers.cloudflare.com/workers/platform/limits/)):
  on the free plan, **10 ms of CPU** and 1,000 calls to Cloudflare services (R2, …) per
  invocation; on the paid plan, 30 s of CPU and 10,000 calls. So the work is cut into
  **one invocation per window**: about 10 pieces × 3–4 ranged reads plus one AI call. The
  POC reads only each piece's audio bytes (the `moof` boxes, then one span of samples):
  **~96 KB of a 2.6 MB piece**. A 4-minute take reads in 1.3–4.9 s instead of 17 s.
- **Naming a long take:** one hour of talk is ~15–17K tokens of transcript, well inside
  gpt-oss's window (128K) and Clef's (64K).
- **Not measured yet:** CPU per invocation on a free-plan account. Workers time CPU only
  between I/O, so it can't be read from inside. Reading it needs Workers analytics during
  a run (§9.7). The likely cost
  is the base64 of ~1 MB of audio, a few ms. If that's too much, 30 s windows halve it.

## 2. Where each one helps shroom

| Feature | What does it | Fit | Notes |
|---|---|---|---|
| **Transcript** (stored beside the take) | Whisper turbo | **core** | `rec/<id>/ai/transcript.json`, words with times; ~50 KB per 10 min |
| **Auto-title** | Whisper + LLM | **core** (B171) | provisional from the first ~2 min, final after Stop |
| **Chapters** | Whisper + LLM; code holds the rules | **core** (B171) | WebVTT chapters; see §4.3 for Clef here |
| **Captions for viewers** | Whisper's VTT | cheap add-on | the same transcript; a player track; translate-to-English is a flag |
| **Summary** (a few lines under the video) | LLM | cheap add-on | same input as naming, ~+20% of naming's cost |
| **Search** (Library, by what was said) | the transcript + text search | later | plain text search over transcripts is enough; embeddings (`bge-m3`, $0.012/M) only if needed |
| **Cleanup suggestions** (sibling: `claude/cleanup-suggestions`) | **Clef** (27B) per sentence (`choice`: filler / false start / off-topic / keep) | **strong for System 1** | 64 sentences a call; thresholds on probabilities. Validate Clef-flash on real speech before using the cheaper model (see the note under this table) |
| **Voice-cue editing** (sibling: `claude/voice-cut-editor`) | Whisper turbo's **word times** + **Clef** (27B) (`noul`: "is this an instruction to cut?") | **strong** | word starts/ends confirmed live; tested live by the editor's POC: full Clef at a 0.3 threshold kept 98 of 99 cues and let through 0 of 34 non-cues (148 sentences in 2.5 s); Clef-flash was weak (median 0.15 on real cues) |
| **"Is anyone talking?"** | Whisper's `word_count`, or Clef-omni on audio | gate | skip naming silent or music-only takes, and keep the date |
| **Title check** | Clef `score` | optional | e.g. pick the best of 3 LLM titles, or flag a title that leaks a password |

**Which Clef.** Two live tests now point the same way: Clef-flash missed topic changes on
real speech here (§4.3), and missed spoken cut cues in the editor's POC. Full Clef (27B,
$0.24/M input, ~6× Clef-flash) separated the cues cleanly. So for judgments on real
speech, **use full Clef**. It is still a few neurons per minute of talk.

## 3. The POC

Files under `ai-titles-chapters/poc/`, all dependency-free, like the space's Worker:

- `fmp4.js` reads the take's AAC track from `init.mp4`, then each piece's audio samples
  from its `moof` boxes, and rewraps them as ADTS frames, a stream Whisper takes as it
  is. `pieceAudioRanged` reads only the box headers, the `moof`s and the audio span. A
  piece's start comes from its `tfdt`, so words carry the take's clock.
- `ai.js` holds the windows, the call to Whisper, the word merge at window edges, and
  sentences (a full stop, a pause, or a run-on cut at a comma after 15 s). Then naming,
  done one of two ways:
  - **`llm`**: one call names the video and places the chapters.
  - **`clef`**: Clef-flash scores each block edge "does a new topic start here?", the
    code picks the cuts, and the LLM only names them.

  Then the chapter rules, a title fit to show, and the WebVTT output.
- `worker.js` is the route, `POST /_shroom/ai/<id>`. It reads
  `rec/<id>/v0/{init.mp4, seg_*.m4s}` and writes `rec/<id>/ai/transcript.json`,
  `title.json` and `chapters.vtt`. A stored transcript is reused, so a take is heard once
  and named as often as needed. The POC's gate is a secret in the path; the product's
  would be the owner cookie and the Mac's upload pass.
- `test/`: 13 tests, run with `npm test` in that folder. They cover the fMP4 parsing on a
  real ffmpeg-made take in shroom's encoding, ranged reads matching whole reads, edge
  words heard exactly once, the chapter rules, both naming ways against a fake AI held
  to the documented answer shapes, and the route against an in-memory bucket.
- `live.mjs` runs the POC live, on the **test account** only (TESTING-PLAN D3). It makes
  a bucket and the Worker (`shroom-probe-ai`), uploads the takes, calls the route for
  each way and model, and deletes everything on the way out:

  ```sh
  cd dev/proposals/ai-titles-chapters/poc
  sample/make-sample.sh /tmp/walkthrough                      # invented take, flite voice
  sample/make-sample.sh /tmp/onboarding sample/script2.txt
  SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node live.mjs /tmp/walkthrough,/tmp/onboarding out.json \
    llm:@cf/openai/gpt-oss-120b clef:@cf/openai/gpt-oss-20b
  ```
- `results/` holds every live run of 2026-10-18 (titles, chapters, timings, neurons,
  Clef's cut chances), and the transcripts of the two invented takes.

## 4. What it made

### 4.1 The setup video (real voice over music, 4:09)

Whisper turbo heard it almost word for word: "Shroom", "R2", "three hours … nothing",
"$1.50 a month". It got "Cloudflare" wrong once ("cloudfair").

- **gpt-oss-120b:** *How to set up and use Shroom*, which is the video's real title.
  Chapters: 0:00 Introduction and overview · 0:28 Installing Shroom and creating account ·
  1:09 Connecting to Cloudflare and setting token · 1:51 Pricing, naming links, and
  permissions · 2:23 Using Shroom to record and share · 3:01 Team collaboration and final
  notes.
- **gpt-oss-20b:** *Shroom screen recorder setup and usage*, with 7 chapters along the
  same lines.
- **gemma-4:** *Setting up and using Shroom*. Chapters: 0:00 Shroom setup and overview ·
  0:22 Installing and connecting Cloudflare · 1:51 Permissions and recording process ·
  2:57 Sharing and team management.
- **The Clef way (clef-flash + gpt-oss-20b):** 3 chapters, at 0:00, 2:08 and 3:10.

### 4.2 The invented takes (synthetic voice)

*Walkthrough* (3:03, a sign-up screen, an invoice bug and trial numbers):

- gpt-oss-120b: *Sign up screen, invoice bug, trial numbers*. Chapters: 0:00 Sign up
  screen changes · 1:08 Invoice page bug details · 1:54 Trial results and metrics · 2:36
  Next steps and wrap up. That is right to the sentence.
- llama-3.2-3b: *Quick Walk Before Thursday Review*. It made six chapters and repeated
  one: the small models name the occasion and over-split.

*Onboarding* (4:10, the repo layout, tests, deploys, on-call):

- gpt-oss-120b: *Onboarding: code layout, testing, deployment, on-call*.
- gpt-oss-20b: 0:00 Welcome and overview · 0:21 Repository layout · 1:10 Running tests ·
  2:01 Change deployment process · 3:10 On-call responsibilities.

### 4.3 Clef for chapter cuts: fast, but not the best tool here

On the invented takes, which have clean pauses between topics, clef-flash put the cuts
right: chance 0.92 at the invoice bug, 0.76 at the trial, 0.83–0.87 at each new topic of
the onboarding. On the real narrated video its chances were flatter (at most 0.65), and
it found 2 of ~5 topic changes. Its call took 0.5–0.8 s for 5–9 neurons. The naming
call after it was half the LLM-only one (22–25 neurons, 1.7–2.7 s against 36–55 and
6–10 s). Over an hour, though, the LLM still reads the whole transcript to name the
sections, so the saving is small (§5). **Verdict:** for chapters the LLM way is better;
Clef's place is in the sibling features (§2).

### 4.4 What the synthetic voice did, and what wasn't tried

The flite voice clips its first word after each pause ("Okay" came out as "K", "Right"
as "GHT"). The real voice didn't. Not tried yet:

- **non-English speech.** Whisper detects the language; the prompt asks for titles in
  the language spoken.
- **a real shroom take with pauses** (a paused take has several `init_<k>.mp4` tracks;
  the POC handles one).
- **a take over 10 minutes.**

## 5. Cost per recorded hour, and speed

**Measured on a real 35:39 recording**, read back from Cloudflare's meter (2026-10-18). The
recording is a human voice reading a book (LibriVox, public domain: four chapters of *The
Art of War*). It was cut into 357 pieces in shroom's own encoding and run on the test
account the way the product would: one ~60 s window per Worker invocation, then one naming
call (`poc/measure.mjs`; `poc/results/measure-2026-10-18-36min.json`). The meter was read
with a one-hour Account Analytics Read token, deleted after use. It showed exactly the
computed figures: Whisper 1,825.80 neurons over 36 calls, gpt-oss-120b 351.96, gemma-4
87.35. The earlier, shorter runs agreed too: Clef-flash billed 49.5 neurons over 6 calls
as computed. Two gpt-oss-120b calls that ran out of tokens while reasoning still billed
~140 each, so the build allows 4,000 output tokens.

| Per recorded hour | Neurons | $ on the paid plan |
|---|---|---|
| Transcript (Whisper turbo; the 6 s overlap between windows adds ~10% of audio) | 3,072 | $0.034 |
| Title and chapters with gpt-oss-120b (8,053 tokens in and 1,404 out for 35:39; scaled linearly, an upper bound) | ~590 | $0.0065 |
| … or with gemma-4 (8,634 in, 325 out) | ~150 | $0.0016 |
| Captions (written from the same words: no second hearing) | 0 | $0 |
| **Total, recommended** | **~3,660** | **~$0.040** |
| *For scale: Deepgram nova-3 instead of Whisper* | *~31,000* | *$0.34* |

- **The free day (10,000 neurons) covers about 2.7 hours of video a day** for the whole
  space (3.1 hours with gemma-4). Past it, on the free plan, naming waits until 00:00
  UTC and catches up. The paid plan gets the same 10,000 a day, then continues at
  ~4¢ an hour.
- **A provisional title before Stop** (from the first ~2 minutes heard) costs ~20
  neurons.
- **Speed, on the 35:39 take:** one window was heard in a median of 4.3 s (at most
  11.9 s). Naming took 16 s with gpt-oss-120b and 7 s with gemma-4. In the product the
  hearing runs while recording, so **after Stop: the last window (~5 s) plus naming
  (~7–16 s)**. On a 4-minute take the naming took 3–10 s.
- **Quality on the long take:** gpt-oss-120b put all 8 chapter starts within seconds of
  where the book's chapters begin. gemma-4 split finer and hit the 12-chapter cap, which
  in the POC then dropped the last 14 minutes. The rule now folds the shortest chapters
  away instead, so a long video keeps chapters to its end (§3).
- **Size:** 35:39 of talk made 5,371 words, a transcript of ~140 KB and 503 caption cues
  (48 KB of WebVTT).
- **Later saving:** transcribing on the Mac (macOS 26's SpeechAnalyzer) would remove the
  transcript line, ~85% of the cost (as noted under B171).

## 6. Recommendation

1. **Hear with Whisper turbo, in 60 s windows while the take uploads.** Start each
   window, apart from the first, with the last piece of the window before. Read only the
   audio bytes.
2. **Name with gpt-oss-120b**, with gemma-4 as the fallback on an error or an empty
   answer. Use one call: title plus chapters. Code holds the rules (§3).
3. **Show chapters in the seek bar** (Bagrat, 2026-10-18): one section per chapter, a
   tip with the time and name on hover, and on a phone the same above the thumb while a
   finger is on the bar. Only when there are 2 or more chapters.
4. **Store everything beside the take** (`rec/<id>/ai/…`), deleted with it. The record
   keeps today's `titled` flag: an AI title fills the title only while `titled` is
   false, and typing one sets it as today.
5. **Off by default; the owner turns it on once.** One card in the Library and a switch in
   Settings · Space, each with one plain cost line. Members never see a cost.
6. **Hand the sibling features full Clef** (not Clef-flash) for per-sentence judgments, and Whisper
   turbo's word times to the voice-cue editor.
7. **Captions ship with it:** `rec/<id>/ai/captions.vtt`, from the same transcript, as a
   track viewers can turn on in the player (§9.6).

## 7. Spec sketch (provisional, numbers at ratification)

Rows `N-ai-1…6` stand in for ledger numbers, so they don't collide with other proposals.
Said-lines are drafts, copy for Bagrat.

- **N-ai-1 — Turning naming on.** Owner-class only. **Turn on** (the Library card or the
  Settings · Space switch) redeploys the space's Worker with an `ai` binding. That is a
  re-wire with no new power (B171), shown as today's wiring. **Not now** hides the card
  for good, and the switch stays. Off again: no new naming. Titles and chapters already
  made stay. *Said:* `library.ai-offer` "shroom can name your videos and add chapters";
  `library.ai-offer-cost` "From what's said in them, by your own Cloudflare account. Free
  for about 3 hours of video a day; past that, about 4¢ an hour."; `settings-space.name-videos`
  "Name videos and add chapters".
- **N-ai-2 — Hearing as it uploads.** When a window's pieces have settled, the Mac calls
  `POST /_shroom/ai/<id>/hear?through=<seg>` with the take's upload pass. It is one
  invocation per window (the free plan's limits, §1.7). The Worker appends the window's
  words to `rec/<id>/ai/transcript.json`. Members' takes go the same way, on the owner's
  account. **Edge:** no AI binding (naming off) → 404, and the Mac stops asking for that
  take.
- **N-ai-3 — The title.** After ~2 minutes heard, and again after Stop, the Worker
  writes the title into the record **only while `titled` is false**, as `aiTitle` with
  the record's `title` following it. A title typed on the page or in the Library (today's
  `POST /_shroom/title`, `titled: true`) always wins and is never written over. Until a
  title comes, the date shows, as today. *Said (owner's page, until edited):*
  `page-owner.named-from-what-you-said` "Named from what you said. Click the title to
  change it."
- **N-ai-4 — Chapters.** After Stop, on a take of 90 s or more with 2 or more chapters,
  the Worker writes `rec/<id>/ai/chapters.vtt`. The page splits its seek bar into one
  section per chapter, 2 px apart.
  - **Hover** (laptop): a tip at the pointer gives the time and the chapter's name, and
    the section under the pointer grows a little.
  - **Touch** (phone): while a finger drags the thumb, and for ~1.5 s after a tap, the
    same tip sits above the thumb, and the thumb grows. Lifting seeks. The bar's touch
    area is 44 px tall.
  - **Always:** the chapter now playing is named after the time (`1:20 / 3:03 · Invoice
    page bug details`, cut short with … when long).
  - **Keyboard and VoiceOver:** the bar's spoken value names the chapter.

  There is no chapter list. Viewers need no account, as today. A take with fewer than 2
  chapters keeps today's whole bar. The same step
  writes `rec/<id>/ai/captions.vtt`, a captions track the player offers (off until a
  viewer turns it on).
- **N-ai-5 — The free day runs out.** A Workers AI refusal for the daily allocation marks
  the take *naming later*. A Cron Trigger on the space's Worker (free on both plans)
  catches up after 00:00 UTC, oldest first. No error shows anywhere; the date stays until
  then.
- **N-ai-6 — Removal.** Deleting a take deletes `rec/<id>/ai/` with it, under today's
  delete and seal rules (§4.20).
- **Laws kept:** nothing turns on by itself (Turn on is the owner's act). The words say
  what it does, not how. Members never see a cost. Capture and upload are untouched: if
  naming fails, nothing else does.

## 8. Boards

Added to the [shroom UI canvas](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg), in a
new column right of the editor's boards (x = 11600). PNGs are in
`ai-titles-chapters/boards/`, beside the generator (`make-boards.py`):

- **Library — the one-time offer** (`AiOffer`)
- **Settings — Space: Name videos and add chapters, off and on** (`AiSettings`)
- **The page — the owner's, named for you** (`AiOwnerNamed`)
- **The page — chapters in the seek bar, named on hover** (`AiPageMarks`), Bagrat's pick, approved 2026-10-18
- **The page on a phone — chapters in the seek bar, named while a finger is on it** (`AiPageMarksPhone`), approved 2026-10-18
- **The page — chapters, A: listed under the player** (`AiPageChapters`), not chosen, kept for the record

The titles and chapters on the boards are the POC's real output on the invented
walkthrough.

## 9. Calls made, and what's left for Bagrat

The POC needed an answer to each question below, so each one is decided here, with its
alternative, as a recommendation. All of them are Bagrat's to ratify or overturn.

1. **Chapters live in the seek bar: ruled by Bagrat on 2026-10-18.** He chose the bar
   split into chapter sections, with a tip naming the chapter on hover while the video
   plays. That is option B; option A (a list under the player) is dropped. For phones,
   which have no hover, the call made here: a finger on the bar shows the same tip above
   the thumb (while dragging, and briefly after a tap). The current chapter is always
   named after the time, so a phone viewer knows where they are without touching. No list
   anywhere. The alternative was a list on phones only; it was left out to keep one
   design everywhere. **Seen and approved:** Bagrat looked at both boards, the laptop
   (`AiPageMarks`) and the phone (`AiPageMarksPhone`), and approved the chapter design
   as drawn on them (Bagrat, 2026-10-18).
2. **Name with gpt-oss-120b, falling back to gemma-4.** Decided because it gave the best
   titles in 3 of 3 takes, at ~$0.007 an hour, under a fifth of the total. gemma-4 is
   ~4× cheaper and faster, and nearly as good: it is the fallback, and the obvious
   switch if cost ever matters.
3. **An optional monthly limit, off by default.** It shows only on Cloudflare's paid
   plan, where spend can grow, as drawn ("Set a monthly limit…"). On the free plan the
   daily allowance already caps it. Decided because owners on the paid plan may want a
   ceiling, while a forced default limit would quietly stop naming for busy teams.
   Alternative: no limit at all.
4. **Past videos: offered once.** Turn on is followed by one line, "Name my past videos
   too", with its cost stated, run at the free day's pace. Decided because the owner's
   Library otherwise stays half dated, half named. The alternative was leaving past
   videos alone. (Not drawn yet: a follow-up board.)
5. **Chapters will be editable**, but not in the first cut. The page's owner could rename
   or remove a chapter later; v-next ships them read-only. Decided because typed edits
   need their own rules (like `titled`) and boards, and the AI chapters were good enough
   in testing to ship read-only first.
6. **Captions: yes, in the same first cut.** They are a WebVTT track from the transcript
   that already exists, at no extra AI cost. **Summary: later.** It's new copy on the page
   and the least needed. Decided because captions are nearly free and help viewers
   (sound off, accessibility).
7. **The metered check is done** (§5). Left open: CPU per invocation on a free-plan
   account. The next live run should read it from the Worker's own analytics, or
   `wrangler tail`, during the run rather than after.

## 10. Built (2026-10-18)

The POC became the product on this branch, at Bagrat's ask the same day (build now, decide
the release later). What it is, in one place; the rows are DECISIONS B205–B209.

- **The space's page** (`worker/src/worker.js`, its last section): `POST /_shroom/api/ai`
  takes one step a call — hears the next minute or names the done take — and says when
  there's more; the transcript grows at `rec/<id>/ai/transcript.json`; the record gets the
  title (while untitled), `named` and `captions`. A schedule each minute takes the oldest
  waiting take one step: a used-up free day (3036) waits for 00:00 UTC and the owner page
  says so, a busy model five minutes. Captions are written per view through the saved edit
  (`GET /_shroom/cc/<id>`); chapters show as sections of the seek bar with a tip, and the
  embed has both. Without the binding nothing runs and every page is as it was.
- **The setup Mac** turns it on: a switch in Settings · Space, and once a card in its
  Library (Turn on · Not now); on re-wires the page with the AI and its schedule
  (`shroom_cloud::wire::set_ai`), and a refresh after an update keeps it.
- **Every Mac** tells its space, from the uploader, each ten pieces and once the take is
  live, through a lane of its own (`shroom_cloud::member::ai_signal`); a done that went
  unanswered is said again from launch.
- **Checked live on the test account, which is on Cloudflare's free plan**
  (`worker/dev/ai-live.mjs`): two takes heard while "recording", named and captioned, the
  second finished by the schedule alone once its trigger reached the network (about five
  minutes the first time) — *The Art of War Laying Plans*, three chapters, 43 caption
  cues, the page with chapters, captions and its button. About 330 neurons, inside the
  free day.
- **CPU at the edge** (`worker/dev/ai-cpu.mjs`, one invocation a minute): hearing a minute
  20–61 ms, naming 12 ms, a page with chapters 10 ms, captions 9.5 ms. The free plan's limit
  is 10 ms with some give for overruns; every step ran on the free test account, as all 36
  windows of the 36-minute take did (§5). Reading a window's sound is most of it (22
  ranged reads, about 1 ms each); a Mac-side audio file per minute would make it one.


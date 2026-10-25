<script lang="ts">
  // The editor (N63) renders the slice it's handed — final copy included — and plays the
  // edit. It composes no sentences and decides nothing (ARCHITECTURE §3.5): every change
  // is a step sent to the core (`edit`), which takes it into the edit's history or refuses
  // it, and the next slice is what the edit is. Its own are the measuring and the playing:
  // the playhead, a part being selected, an end being dragged (played as it would be until
  // the core answers), and the skip loop that plays the slice's `keep` — the cuts jumped,
  // held to the trim — as the share page does. The numbers it fills into the Scene's
  // templates are the clock's and a selection's length.
  import { onMount, untrack } from "svelte";
  import { act, edit, key, onScene, ownLabel, sceneGet, type Action, type EditStep } from "./lib/ipc";
  import Btn from "./onboarding/Btn.svelte";
  import Sheet from "./library/Sheet.svelte";
  import type { Sheet as SheetView } from "./library/types";
  import Timeline, { type Selection, type Trim } from "./editor/Timeline.svelte";
  import { clock, fill } from "./editor/clock";
  import { END_SLACK, editedAt, gain, keep as keepOf, length, nextPlay, snap, type Keep } from "./editor/keep";
  import { isEditor, type EditorWindow } from "./editor/types";

  /** Where the shell serves the editor's media (runtime/media.rs); a gallery points it
   * elsewhere. */
  const base = window.__shroomMediaBase ?? "shroom-media://localhost";

  let w = $state.raw<EditorWindow | null>(null);
  let video: HTMLVideoElement | undefined = $state();
  /** The playhead, in seconds on the recording. */
  let pos = $state(0);
  let playing = $state(false);
  /** Paused at the edit's end: the time reads the whole length, and Play starts over. */
  let ended = $state(false);
  let peaks = $state.raw<Uint8Array | null>(null);
  let peaksFor = "";
  let selection = $state<Selection | null>(null);
  /** An end being dragged, then sent: what plays until the core's slice says otherwise. */
  let trim = $state<Trim | null>(null);
  let trimSent: { list: string; at: number } | null = null;
  /** Each time a video becomes ready in this window — every open, the same video's too (a
   * baked one is another file under the same name) — its media are read afresh. */
  let opened = $state(0);
  let lastMedia = "";

  const send = (a: Action) => act("editor", a);
  const step = (s: EditStep) => void edit(s);

  const take = (s: unknown) => {
    if (!isEditor(s)) return;
    if (!s) {
      // Put away: the window is only hidden, so the video stops and is let go of here, or
      // it would go on playing behind it.
      unload();
      w = null;
      lastMedia = "";
      return;
    }
    w = s;
    // A video that has just become ready: a new open, starting at the top, paused.
    if (s.body.media.state === "ready" && lastMedia !== "ready") {
      opened += 1;
      pos = 0;
      playing = false;
      ended = false;
      selection = null;
      trim = null;
      trimSent = null;
    }
    lastMedia = s.body.media.state;
    // A trim sent is answered once the list changes (refused, nothing changes: a moment
    // later the ends go back to the slice's, `trimmed`).
    if (trimSent && JSON.stringify(s.body.list) !== trimSent.list) {
      trimSent = null;
      trim = null;
    }
    // A picked cut is found again by its times; a part being selected lasts while editing does.
    const sel = selection;
    if (!s.body.editable || !s.body.list) selection = null;
    else if (sel?.kind === "cut") {
      const i = s.body.list.cuts.findIndex((c) => c.start === sel.start && c.end === sel.end);
      selection = i >= 0 ? { ...sel, index: i } : null;
    }
  };

  let b = $derived(w?.body);
  /** The window's question, as a sheet over it (Bagrat, 2026-10-25: the footer's place was
   * too easy to miss): “Delete the original?” once the final video is up (B197), or ✕'s
   * “Save your edit?” (N71). The window behind is held: inert, and Space plays nothing. */
  let ask = $derived.by((): SheetView | null => {
    if (!w || !b) return null;
    if (b.after?.state === "asking") {
      const a = b.after;
      return { id: w.id, faces: [], title: a.title, lines: [a.line], blocked: null, buttons: [a.keep, a.delete] };
    }
    const c = b.closing;
    return c ? { id: w.id, faces: [], title: c.title, lines: [c.line], blocked: null, buttons: [c.discard, c.keep, c.save] } : null;
  });
  let media = $derived(b?.media.state ?? "opening");
  let ready = $derived(media === "ready");
  let src = $derived(b && ready ? `${base}/${encodeURIComponent(b.rec)}/video?open=${opened}` : null);
  /** The timeline's scale: the recording's length. */
  let duration = $derived(b ? (b.source_duration > 0 ? b.source_duration : b.media.state === "ready" ? b.media.duration : 0) : 0);
  /** What plays: the slice's keep, or the ends being dragged over the slice's cuts. */
  let playKeep: Keep = $derived(b?.list && trim ? keepOf({ in: trim.in, out: trim.out, cuts: b.list.cuts }) : (b?.keep ?? []));
  let len = $derived(b ? (b.list && trim ? length(playKeep) : b.edited_duration > 0 || b.list ? b.edited_duration : b.source_duration) : 0);
  let now = $derived(ended ? len : ready ? editedAt(playKeep, pos) : 0);
  // The Scene's “{} / {}”: the edited position, then the edited length.
  let time = $derived(fill(fill(b?.labels.time ?? "{} / {}", clock(now, ended)), clock(len, true)));
  let was = $derived(b?.list && clock(len, true) !== clock(b.source_duration, true) ? fill(b.labels.was, clock(b.source_duration, true)) : null);

  $effect(() => {
    if (w) document.title = w.body.title;
  });

  // The peaks, once per open, as soon as the video is here.
  $effect(() => {
    const rec = b?.rec;
    const key = `${rec}#${opened}`;
    if (!ready || !rec || peaksFor === key) return;
    peaksFor = key;
    peaks = null;
    fetch(`${base}/${encodeURIComponent(rec)}/peaks?open=${opened}`)
      .then((r) => (r.ok ? r.arrayBuffer() : null))
      .then((buf) => {
        if (buf && peaksFor === key) peaks = new Uint8Array(buf);
      })
      .catch(() => {});
  });

  // A new edit while paused: a playhead left in a cut moves on to where the edit resumes.
  $effect(() => {
    const k = playKeep;
    untrack(() => {
      if (!video || !video.paused || ended || !k.length || trim) return;
      const to = nextPlay(k, video.currentTime);
      if (typeof to === "number") put(to);
    });
  });

  // Done stops the preview: once the edit can't be changed (saving, the final video being
  // made, the question of the original, a save the space turned down), what was playing
  // pauses. Play pressed after that is the person's own.
  let wasEditable = false;
  $effect(() => {
    const editable = !!b?.editable;
    untrack(() => {
      if (wasEditable && !editable) video?.pause();
      wasEditable = editable;
    });
  });

  // ── playing ──

  /** The window is put away: the preview stops and lets go of its file. Its pause event
   * lands after the video is gone, so what the button shows is set here. */
  function unload() {
    cancelAnimationFrame(raf);
    raf = 0;
    playing = false;
    ended = false;
    if (!video) return;
    video.pause();
    video.removeAttribute("src");
    video.load();
  }

  function put(t: number) {
    if (video) video.currentTime = t;
    pos = t;
  }

  /** The skip: in a cut or before the trim, on to the next kept moment; past the edit's
   * end, pause there. Each frame while playing, and on every time update. */
  function guard() {
    const k = playKeep;
    if (!video || video.paused || !k.length) return;
    const to = nextPlay(k, video.currentTime);
    if (to === null) {
      level(gain(k, video.currentTime));
      return;
    }
    if (to === "end") {
      level(1);
      finish(k);
      return;
    }
    level(0);
    video.currentTime = to;
  }

  /** The preview's level, eased around each skip as the share page does (in 5% steps, so a
   * frame doesn't write it for nothing). */
  function level(g: number) {
    const v = Math.round(g * 20) / 20;
    if (video && video.volume !== v) video.volume = v;
  }

  function finish(k: Keep) {
    if (!video) return;
    video.pause();
    const [s, e] = k[k.length - 1];
    put(Math.max(s, e - END_SLACK));
    ended = true;
  }

  let raf = 0;
  function frame() {
    raf = 0;
    if (!video || video.paused) return;
    guard();
    pos = video.currentTime;
    raf = requestAnimationFrame(frame);
  }

  function play() {
    const k = playKeep;
    if (!video || !ready || !k.length) return;
    const at = nextPlay(k, video.currentTime);
    if (ended || at === "end") put(k[0][0]);
    else if (at !== null) put(at);
    ended = false;
    video.play().catch(() => {});
  }

  // By what the video is doing, not by what the button last showed.
  const toggle = () => (video && !video.paused ? video.pause() : play());

  function seek(t: number) {
    const k = playKeep;
    if (!ready) return;
    const to = k.length ? snap(k, t) : t;
    ended = !!k.length && nextPlay(k, to) === "end";
    put(ended ? Math.max(k[k.length - 1][0], to - END_SLACK) : to);
  }

  /** An end being dragged shows its frame (while paused): where the edit will start, or end. */
  function scrub(t: number) {
    if (!video || !video.paused || !ready) return;
    ended = false;
    put(trim && t >= trim.out ? Math.max(trim.in, t - END_SLACK) : t);
  }

  function trimmed(t: Trim) {
    trimSent = { list: JSON.stringify(b?.list ?? null), at: performance.now() };
    step({ op: "trim", in: t.in, out: t.out });
    // Nothing answered (a core that refused and said nothing): the ends go back.
    setTimeout(() => {
      if (trimSent && performance.now() - trimSent.at >= 1000) {
        trimSent = null;
        trim = null;
      }
    }, 1050);
  }

  function cut() {
    if (selection?.kind !== "range") return;
    step({ op: "cut", start: selection.start, end: selection.end });
    selection = null;
  }

  onMount(() => {
    // Its own window's slice: one window per video, each labelled by its number (N73).
    sceneGet<EditorWindow>("editor").then(take);
    const un = onScene<EditorWindow>(take, ownLabel());
    const onKey = (e: KeyboardEvent) => {
      if (e.isComposing || !w) return;
      const body = w.body;
      if (e.key === "Enter") {
        // Return is the Scene's (Done, or Close when the video couldn't come), whatever
        // holds the keyboard.
        e.preventDefault();
        if (w.ret) key("editor", "return");
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        // While the window asks, Esc is the sheet's (Keep editing, N71; Keep for 2 days,
        // B197) whatever is selected behind it.
        if (selection && !ask) selection = null;
        else if (w.esc) key("editor", "esc");
        return;
      }
      if (e.key === " ") {
        e.preventDefault();
        // The edit is held while the window asks: the preview stays paused (N71).
        if (!ask) toggle();
        return;
      }
      if (e.key === "Backspace" || e.key === "Delete") {
        // Delete never means "go back" in a window.
        e.preventDefault();
        if (body.editable) cut();
        return;
      }
      if (e.metaKey && !e.altKey && !e.ctrlKey && e.key.toLowerCase() === "z") {
        e.preventDefault();
        if (!body.editable) return;
        if (e.shiftKey && body.can_redo) step({ op: "redo" });
        else if (!e.shiftKey && body.can_undo) step({ op: "undo" });
      }
    };
    // Space is play and pause wherever the keyboard is: never a click on a focused button.
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key === " ") e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      un.then((f) => f());
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onKeyUp);
      cancelAnimationFrame(raf);
    };
  });
</script>

{#if w && b}
  <main class="ed" data-state={w.id} data-media={media} inert={!!ask}>
    <div class="stage">
      <div class="screen">
        {#if src}
          <!-- A recording has no captions to offer (v1): the person is looking at their own. -->
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            bind:this={video}
            {src}
            playsinline
            preload="auto"
            disablepictureinpicture
            onloadedmetadata={() => {
              // The edit starts where it starts: the trim's in, past a cut at the very top.
              const k = playKeep;
              if (video && k.length && nextPlay(k, video.currentTime) !== null && !ended) put(k[0][0]);
            }}
            onplay={() => {
              playing = true;
              if (!raf) raf = requestAnimationFrame(frame);
            }}
            onpause={() => (playing = false)}
            onemptied={() => {
              // A new file in its place (every open loads it afresh) stops it without a
              // pause event.
              playing = false;
            }}
            onended={() => {
              playing = false;
              ended = true;
            }}
            ontimeupdate={() => {
              guard();
              if (video) pos = video.currentTime;
            }}
            onseeked={() => {
              if (video && video.paused) pos = video.currentTime;
            }}
          ></video>
        {/if}
      </div>
    </div>

    <div class="bar" class:gone={media === "failed"}>
      <button type="button" class="play" aria-label={playing ? b.labels.pause : b.labels.play} title={playing ? b.labels.pause : b.labels.play} disabled={!ready} onclick={toggle}>
        {#if playing}
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 3.5h2.4v9H4.5zM9.1 3.5h2.4v9H9.1z" /></svg>
        {:else}
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5v9l7.5-4.5z" /></svg>
        {/if}
      </button>
      <span class="time" data-measured>{time}</span>
      {#if was}<span class="was" data-measured>{was}</span>{/if}
      <span class="grow"></span>
      {#if (media === "ready" || media === "opening") && !b.after}
        <span class="hint">{b.hint}</span>
        <button type="button" class="icon" aria-label={b.labels.undo} title={b.labels.undo} disabled={!(b.editable && b.can_undo)} onclick={() => step({ op: "undo" })}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5.5 4L2.5 7l3 3" /><path d="M2.5 7h7a4 4 0 0 1 0 8H7" /></svg>
        </button>
        <button type="button" class="icon" aria-label={b.labels.redo} title={b.labels.redo} disabled={!(b.editable && b.can_redo)} onclick={() => step({ op: "redo" })}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10.5 4l3 3-3 3" /><path d="M13.5 7h-7a4 4 0 0 0 0 8H9" /></svg>
        </button>
      {/if}
    </div>

    <div class="timeline">
      {#if b.after?.state === "making"}
        <!-- Saved: the final video being made, where the track was (B197). -->
        <div class="ruler-room"></div>
        <div class="note making" role="status">
          <span data-measured>{b.after.line}</span>
          <div class="fill" role="progressbar" aria-valuenow={b.after.pct} aria-valuemin={0} aria-valuemax={100}>
            <span style="width: {Math.max(0, Math.min(100, b.after.pct))}%"></span>
          </div>
        </div>
      {:else if b.media.state === "getting" || b.media.state === "failed"}
        <!-- Where the track goes: the line, and the bar that fills or Try again. -->
        <div class="ruler-room"></div>
        <div class="note" class:row={b.media.state === "failed"} role="status">
          <span>{b.media.line}</span>
          {#if b.media.state === "getting"}
            <div class="fill" role="progressbar" aria-valuenow={b.media.pct} aria-valuemin={0} aria-valuemax={100}>
              <span style="width: {Math.max(0, Math.min(100, b.media.pct))}%"></span>
            </div>
          {:else}
            <Btn b={b.media.retry} {send} inlineNote={false} />
          {/if}
        </div>
      {:else}
        <Timeline
          {duration}
          list={ready ? b.list : null}
          markers={ready ? b.markers : []}
          {peaks}
          playhead={ready && b.list ? pos : null}
          editable={b.editable && ready}
          seekable={ready}
          labels={b.labels}
          bind:selection
          bind:trim
          onseek={seek}
          onscrub={scrub}
          ontrim={trimmed}
          oncut={(start, end) => step({ op: "cut", start, end })}
          onputback={(index) => step({ op: "put-back", index })}
          onapply={(index) => step({ op: "apply", index })}
          onresize={(index, start, end) => step({ op: "resize", index, start, end })}
        />
      {/if}
    </div>

    <div class="grow"></div>

    <!-- Behind a question too, held: its words and buttons as they were. -->
    <footer class="foot">
      <span class="line" class:amber={b.footer_tone === "amber"} role="status">{b.footer}</span>
      <!-- After a save the edit is the video's own: nothing is reverted from here — nor
           once the space turned the save down (no `revert`, B199). -->
      {#if media !== "failed" && !b.after && b.revert}
        {#if b.original_kept}<span class="kept">{b.original_kept}</span>{/if}
        <Btn b={b.revert} {send} inlineNote={false} />
      {/if}
      <Btn b={b.done} {send} inlineNote={false} />
    </footer>
  </main>
  <!-- Its question, a sheet as the Library's are: Keep for 2 days, then Yes, delete, filled
       and a click only (Return does nothing here, law 9); or Discard, a click only, Keep
       editing (Esc) and Save (Return). The keys are the Scene's, from the window's own. -->
  {#if ask}<Sheet sheet={ask} {send} />{/if}
{/if}

<style>
  :global(html),
  :global(body),
  :global(#app) {
    height: 100%;
    margin: 0;
  }
  /* The window's type, the sheet over it included (it sits outside `.ed`). */
  :global(body) {
    font: 13px/1.45 var(--font);
    -webkit-font-smoothing: antialiased;
    color: var(--fg);
  }
  .ed {
    box-sizing: border-box;
    height: 100%;
    padding: 8px 24px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    font: 13px/1.45 var(--font);
    -webkit-font-smoothing: antialiased;
    color: var(--fg);
    background: var(--bg);
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
  }
  /* The video, 640×360 at the window's size, smaller in a shorter window; letterboxed by
     its own shape. */
  .stage {
    flex: 0 1 360px;
    min-height: 150px;
    display: flex;
    justify-content: center;
  }
  .screen {
    height: 100%;
    aspect-ratio: 16 / 9;
    max-width: 100%;
    border-radius: 8px;
    background: var(--seg);
    overflow: hidden;
  }
  .screen video {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }
  .bar {
    flex: none;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .bar.gone {
    visibility: hidden;
  }
  .play {
    width: 34px;
    height: 34px;
    border-radius: 17px;
    border: 0;
    padding: 0;
    background: var(--card);
    box-shadow: 0 0 0 1px var(--card-edge);
    color: var(--fg);
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    cursor: default;
  }
  .play svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
    display: block;
  }
  .play:disabled {
    opacity: 0.45;
  }
  .time {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .was {
    color: var(--dim);
    white-space: nowrap;
  }
  .grow {
    flex: 1 1 0;
  }
  .hint {
    color: var(--muted);
    text-align: right;
  }
  .icon {
    width: 30px;
    height: 30px;
    padding: 0;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--fg);
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    cursor: default;
  }
  .icon svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    display: block;
  }
  .icon:disabled {
    opacity: 0.5;
    color: var(--dim);
  }
  .play:focus-visible,
  .icon:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  /* Room above the track for the cuts' scissors and the markers' flags, and under it for
     the chip. */
  .timeline {
    flex: none;
    padding: 26px 0 48px;
  }
  .ruler-room {
    height: 28px;
  }
  .note {
    height: 66px;
    border-radius: 6px;
    background: var(--field);
    box-shadow: 0 0 0 1px var(--card-edge);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
  }
  .note.row {
    flex-direction: row;
    gap: 12px;
  }
  .fill {
    width: 240px;
    height: 4px;
    border-radius: 2px;
    background: var(--line);
    overflow: hidden;
  }
  .fill span {
    display: block;
    height: 4px;
    background: var(--accent);
  }
  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .foot .line {
    flex: 1 1 0;
    color: var(--muted);
  }
  .foot .line.amber {
    color: var(--amber);
  }
  /* The time the original has left, by Revert: small and dim (B197 (e)). */
  .foot .kept {
    color: var(--dim);
    font-size: 12px;
    white-space: nowrap;
  }
  .note.making span {
    color: var(--fg);
  }
  .foot :global(.b),
  .note :global(.b) {
    min-height: 30px;
    border-radius: 7px;
  }
  .foot :global(.b.filled) {
    padding: 0 16px;
  }
  .foot :global(.b.plain) {
    color: var(--fg-3);
  }
</style>

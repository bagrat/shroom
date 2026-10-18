<script module lang="ts">
  /** What the person has picked on the timeline: a part dragged across the sound, or a cut
   * clicked (by its place in the list, and its times, to find it again in the next slice). */
  export type Selection = { kind: "range"; start: number; end: number } | { kind: "cut"; index: number; start: number; end: number };
  export type Trim = { in: number; out: number };
</script>

<script lang="ts">
  // The editor's timeline (board “Editor — a cut and undo”): the ruler, then the track —
  // the whole recording's sound, the outside of the trim dimmed, the yellow ends, every
  // applied cut red and hatched with its scissors above, a suggestion dashed amber, each
  // marker a flag, the playhead, a part being selected and the chip under it. Times are
  // seconds on the recording's own timeline. It measures and draws; every change is a step
  // the window sends to the core (Editor.svelte), and the core's next slice is the truth.
  import { onMount } from "svelte";
  import { clock, fill, seconds } from "./clock";
  import { MIN_KEEP } from "./keep";
  import { draw } from "./waveform";
  import type { EditorBody } from "./types";

  let {
    duration,
    list,
    markers,
    peaks,
    playhead,
    editable,
    seekable,
    labels,
    selection = $bindable(null),
    trim = $bindable(null),
    onseek,
    onscrub,
    ontrim,
    oncut,
    onputback,
    onapply,
  }: {
    /** The recording's length: the timeline's scale. */
    duration: number;
    list: EditorBody["list"];
    markers: number[];
    peaks: Uint8Array | null;
    /** The playhead's source time; null hides it. */
    playhead: number | null;
    /** The ends drag and the sound selects (the core said so). */
    editable: boolean;
    /** A click moves the playhead (the video is here). */
    seekable: boolean;
    labels: { cut: string; put_back: string; cut_it_too?: string };
    selection?: Selection | null;
    /** The ends while one is dragged (and until the core answers): what plays meanwhile. */
    trim?: Trim | null;
    onseek: (t: number) => void;
    onscrub: (t: number) => void;
    ontrim: (t: Trim) => void;
    oncut: (start: number, end: number) => void;
    onputback: (index: number) => void;
    onapply: (index: number) => void;
  } = $props();

  /** A press that moves less than this is a click (px). */
  const CLICK = 4;

  let width = $state(0);
  let track: HTMLDivElement | undefined = $state();
  let canvas: HTMLCanvasElement | undefined = $state();
  let chipWidth = $state(0);

  const x = (t: number) => (duration > 0 ? (t / duration) * width : 0);
  const timeAt = (clientX: number) => {
    const r = track!.getBoundingClientRect();
    return r.width > 0 ? Math.min(1, Math.max(0, (clientX - r.left) / r.width)) * duration : 0;
  };

  let ends = $derived(trim ?? (list ? { in: list.in, out: list.out } : null));

  // The ruler: every 10 s up to two minutes, every 30 s past them, and wider still when the
  // labels would crowd (a long recording).
  let ticks = $derived.by(() => {
    if (!(duration > 0) || !width) return [];
    const steps = [duration > 120 ? 30 : 10, 60, 120, 300, 600, 900, 1800, 3600].filter((s) => s >= (duration > 120 ? 30 : 10));
    const step = steps.find((s) => (s / duration) * width >= 56) ?? steps[steps.length - 1];
    const out: number[] = [];
    for (let t = 0; t <= duration && x(t) <= width - 12; t += step) out.push(t);
    return out;
  });

  // The chip under what's selected: Cut <length> under a part, Put back <length> under an
  // applied cut, Cut it too under a suggestion (applying it).
  let chip = $derived.by(() => {
    if (!selection || !editable || !list) return null;
    if (selection.kind === "range") return { kind: "cut" as const, at: (selection.start + selection.end) / 2, label: fill(labels.cut, seconds(selection.end - selection.start)) };
    const c = list.cuts[selection.index];
    if (!c) return null;
    if (c.applied) return { kind: "put-back" as const, at: (c.start + c.end) / 2, label: fill(labels.put_back, seconds(c.end - c.start)) };
    return { kind: "apply" as const, at: (c.start + c.end) / 2, label: labels.cut_it_too ?? fill(labels.cut, seconds(c.end - c.start)) };
  });
  let chipLeft = $derived(chip ? Math.min(Math.max(x(chip.at) - chipWidth / 2, 0), Math.max(0, width - chipWidth)) : 0);

  function chipClick() {
    if (!chip || !selection) return;
    if (selection.kind === "range") oncut(selection.start, selection.end);
    else if (chip.kind === "put-back") onputback(selection.index);
    else onapply(selection.index);
    selection = null;
  }

  // ── the press on the track: a click moves the playhead or picks a cut; a drag selects ──

  let press: { id: number; x0: number; t0: number; moved: boolean } | null = null;

  /** The cut under source time `t`, the shortest when cuts overlap. */
  function cutAt(t: number) {
    if (!list) return -1;
    let best = -1;
    list.cuts.forEach((c, i) => {
      if (t >= c.start && t < c.end && (best < 0 || c.end - c.start < list.cuts[best].end - list.cuts[best].start)) best = i;
    });
    return best;
  }

  function pickCut(i: number) {
    const c = list?.cuts[i];
    if (c && editable) selection = { kind: "cut", index: i, start: c.start, end: c.end };
  }

  function down(e: PointerEvent) {
    if (e.button !== 0 || !seekable || !track) return;
    e.preventDefault();
    track.setPointerCapture(e.pointerId);
    press = { id: e.pointerId, x0: e.clientX, t0: timeAt(e.clientX), moved: false };
  }

  function move(e: PointerEvent) {
    if (!press || e.pointerId !== press.id) return;
    if (!press.moved && Math.abs(e.clientX - press.x0) < CLICK) return;
    if (!editable || !ends) return;
    press.moved = true;
    const t = timeAt(e.clientX);
    const start = Math.max(ends.in, Math.min(press.t0, t));
    const end = Math.min(ends.out, Math.max(press.t0, t));
    selection = end > start ? { kind: "range", start, end } : null;
  }

  function up(e: PointerEvent) {
    if (!press || e.pointerId !== press.id) return;
    const p = press;
    press = null;
    if (p.moved) return;
    const t = timeAt(e.clientX);
    const i = editable ? cutAt(t) : -1;
    if (i >= 0) {
      pickCut(i);
      return;
    }
    selection = null;
    onseek(t);
  }

  // ── a yellow end, dragged ──

  /** `by`: how far from its end the pointer took hold (s), so the end doesn't jump to it. */
  let grip = $state.raw<{ id: number; side: "in" | "out"; from: Trim; by: number } | null>(null);

  function gripDown(e: PointerEvent, side: "in" | "out") {
    if (e.button !== 0 || !editable || !ends) return;
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    grip = { id: e.pointerId, side, from: { ...ends }, by: timeAt(e.clientX) - ends[side] };
    selection = null;
  }

  function gripMove(e: PointerEvent) {
    if (!grip || e.pointerId !== grip.id) return;
    const t = timeAt(e.clientX) - grip.by;
    const room = Math.min(MIN_KEEP, duration);
    const now = trim ?? grip.from;
    trim = grip.side === "in" ? { in: Math.min(Math.max(0, t), now.out - room), out: now.out } : { in: now.in, out: Math.max(Math.min(duration, t), now.in + room) };
    onscrub(grip.side === "in" ? trim.in : trim.out);
  }

  function gripUp(e: PointerEvent) {
    if (!grip || e.pointerId !== grip.id) return;
    const g = grip;
    grip = null;
    if (trim && (trim.in !== g.from.in || trim.out !== g.from.out)) ontrim(trim);
    else trim = null;
  }

  // ── the waveform: drawn once the peaks are here, again when the box or the theme changes ──

  let tint = $state(0);
  $effect(() => {
    void width;
    void tint;
    if (canvas) draw(canvas, peaks, duration, getComputedStyle(canvas).color);
  });
  onMount(() => {
    const dark = window.matchMedia("(prefers-color-scheme: dark)");
    const retint = () => tint++;
    dark.addEventListener("change", retint);
    // A move to a display of another density redraws at its pixels.
    let dpr = window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
    const moved = () => {
      dpr.removeEventListener("change", moved);
      dpr = window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
      dpr.addEventListener("change", moved);
      retint();
    };
    dpr.addEventListener("change", moved);
    return () => {
      dark.removeEventListener("change", retint);
      dpr.removeEventListener("change", moved);
    };
  });
</script>

<div class="tl" class:editable bind:clientWidth={width}>
  <div class="ruler" data-measured>
    {#each ticks as t}<span class="tick" style="left: {x(t)}px">{clock(t)}</span>{/each}
  </div>
  <div class="track" role="presentation" bind:this={track} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={() => (press = null)}>
    <div class="clip">
      <canvas class="wave" bind:this={canvas} aria-hidden="true"></canvas>
      {#if ends && width}
        <div class="dim" style="left: 0; width: {x(ends.in)}px"></div>
        <div class="dim" style="left: {x(ends.out)}px; right: 0"></div>
      {/if}
    </div>
    {#if ends && width}
      <div class="band" style="left: {x(ends.in)}px; width: {x(ends.out) - x(ends.in)}px"></div>
      {#each ["in", "out"] as const as side}
        <div
          class="end {side}"
          class:held={grip?.side === side}
          style="left: {side === 'in' ? x(ends.in) - 12 : x(ends.out)}px"
          role="presentation"
          onpointerdown={(e) => gripDown(e, side)}
          onpointermove={gripMove}
          onpointerup={gripUp}
          onpointercancel={gripUp}
        >
          <span></span>
        </div>
      {/each}
    {/if}
    {#if list && width}
      {#each list.cuts as c, i}
        {#if c.applied}
          <div class="cut" class:picked={selection?.kind === "cut" && selection.index === i} style="left: {x(c.start)}px; width: {Math.max(4, x(c.end) - x(c.start))}px"></div>
          <span class="badge" role="presentation" style="left: {(x(c.start) + x(c.end)) / 2 - 10}px" onpointerdown={(e) => (e.stopPropagation(), pickCut(i))}
            ><svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="4.5" cy="4.5" r="2" /><circle cx="4.5" cy="11.5" r="2" /><path d="M6.2 5.6L13 11.8M6.2 10.4L13 4.2" /></svg></span
          >
        {:else}
          <div class="offer" class:picked={selection?.kind === "cut" && selection.index === i} style="left: {x(c.start)}px; width: {Math.max(4, x(c.end) - x(c.start))}px"></div>
        {/if}
      {/each}
      {#if selection?.kind === "range"}
        <div class="sel" style="left: {x(selection.start)}px; width: {x(selection.end) - x(selection.start)}px"></div>
      {/if}
    {/if}
    {#if width}
      {#each markers as m}
        <div class="marker" style="left: {x(m) - 1}px">
          <svg viewBox="0 0 10 12" aria-hidden="true"><path d="M1 0v12M1 1h8l-2 3 2 3H1z" /></svg>
        </div>
      {/each}
      {#if playhead !== null && duration > 0}
        <div class="playhead" style="left: {x(playhead)}px"><span></span></div>
      {/if}
    {/if}
    {#if chip}
      <div class="chip" style="left: {chipLeft}px" bind:clientWidth={chipWidth}>
        <button type="button" class={chip.kind === "put-back" ? "back" : "cutb"} data-measured onpointerdown={(e) => e.stopPropagation()} onclick={chipClick}>
          {#if chip.kind === "put-back"}
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5.5 4L2.5 7l3 3" /><path d="M2.5 7h7a4 4 0 0 1 0 8H7" /></svg>
          {:else}
            <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="4.5" cy="4.5" r="2" /><circle cx="4.5" cy="11.5" r="2" /><path d="M6.2 5.6L13 11.8M6.2 10.4L13 4.2" /></svg>
          {/if}
          {chip.label}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .tl {
    display: flex;
    flex-direction: column;
    gap: 10px;
    user-select: none;
    -webkit-user-select: none;
  }
  .ruler {
    position: relative;
    height: 18px;
  }
  .tick {
    position: absolute;
    top: 0;
    height: 18px;
    border-left: 1px solid var(--line);
    padding-left: 4px;
    color: var(--dim);
    font-size: 11px;
    line-height: 18px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .track {
    position: relative;
    height: 66px;
    border-radius: 6px;
    background: var(--field);
    box-shadow: 0 0 0 1px var(--card-edge);
    touch-action: none;
  }
  .clip {
    position: absolute;
    inset: 0;
    border-radius: 6px;
    overflow: hidden;
  }
  .wave {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    color: var(--muted);
  }
  /* The outside of the trim: the sound faded into the window. */
  .dim {
    position: absolute;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--bg) 72%, transparent);
  }
  .band {
    position: absolute;
    top: -2px;
    bottom: -2px;
    border-top: 2px solid var(--amber);
    border-bottom: 2px solid var(--amber);
    box-sizing: border-box;
    pointer-events: none;
  }
  .end {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 12px;
    background: var(--amber);
    display: flex;
    align-items: center;
    justify-content: center;
    touch-action: none;
  }
  .end.in {
    border-radius: 6px 0 0 6px;
  }
  .end.out {
    border-radius: 0 6px 6px 0;
  }
  .editable .end {
    cursor: ew-resize;
  }
  .end span {
    width: 2px;
    height: 22px;
    border-radius: 1px;
    background: var(--bg);
    display: block;
  }
  .cut {
    position: absolute;
    top: 0;
    bottom: 0;
    background: repeating-linear-gradient(
      135deg,
      color-mix(in srgb, var(--red) 55%, transparent) 0 4px,
      color-mix(in srgb, var(--red) 22%, transparent) 4px 8px
    );
    border-left: 2px solid var(--red);
    border-right: 2px solid var(--red);
    box-sizing: border-box;
  }
  .cut.picked {
    box-shadow: 0 0 0 1px var(--red);
  }
  .badge {
    position: absolute;
    top: -26px;
    width: 20px;
    height: 20px;
    border-radius: 10px;
    background: var(--red);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .badge svg,
  .chip svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: var(--bg);
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
    display: block;
    flex: none;
  }
  .offer {
    position: absolute;
    top: 0;
    bottom: 0;
    border: 2px dashed var(--amber);
    border-radius: 3px;
    box-sizing: border-box;
    background: color-mix(in srgb, var(--amber) 10%, transparent);
  }
  .offer.picked {
    background: color-mix(in srgb, var(--amber) 22%, transparent);
  }
  .sel {
    position: absolute;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--accent) 28%, transparent);
    border-left: 2px solid var(--accent);
    border-right: 2px solid var(--accent);
    box-sizing: border-box;
    pointer-events: none;
  }
  .marker {
    position: absolute;
    top: -16px;
    bottom: 0;
    border-left: 1px dashed var(--accent);
    pointer-events: none;
  }
  .marker svg {
    position: absolute;
    left: 0;
    top: 0;
    width: 10px;
    height: 12px;
    display: block;
    fill: var(--accent);
    stroke: var(--accent);
    stroke-width: 1.2;
  }
  .playhead {
    position: absolute;
    top: -8px;
    bottom: -8px;
    width: 2px;
    margin-left: -1px;
    background: var(--fg);
    pointer-events: none;
  }
  .playhead span {
    position: absolute;
    left: -5px;
    top: -4px;
    width: 12px;
    height: 12px;
    border-radius: 6px;
    background: var(--fg);
    display: block;
  }
  .chip {
    position: absolute;
    top: 76px;
    display: flex;
    padding: 4px;
    border-radius: 8px;
    background: var(--menu);
    border: 1px solid var(--pop-edge);
    box-shadow: var(--shadow);
    white-space: nowrap;
  }
  .chip button {
    font: inherit;
    height: 26px;
    padding: 0 10px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: default;
  }
  .chip .cutb {
    border: 0;
    background: var(--red);
    color: var(--bg);
    font-weight: 600;
  }
  .chip .back {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--fg);
  }
  .chip .back svg {
    stroke: var(--fg);
    stroke-width: 1.5;
  }
  .chip button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>

<script lang="ts">
  // The plan panel (N13): the header, the rows — each with its mark (the canvas's tick,
  // arrow, dot, or the spinner while shroom works), its title, aside and fact (row.ts) —
  // and the nest under the current row, or below the rows once every row is done. The
  // onboarding window (where the rows are the canvas's card) and Settings · Space (N36,
  // N42 — "the same machine, no second repair UI") both render it.
  import type { Snippet } from "svelte";
  import type { Action } from "../lib/ipc";
  import Nest from "./Nest.svelte";
  import { rowParts } from "./row";
  import Runs from "./Runs.svelte";
  import Spinner from "./Spinner.svelte";
  import type { PlanView } from "./types";

  // `afterRow`: what a surface hangs under a row (Settings' quiet microphone door, under
  // the last row — "Letting macOS allow recording" — before any nest).
  let {
    plan,
    send,
    activates,
    afterRow,
  }: { plan: PlanView; send: (a: Action) => void; activates: boolean; afterRow?: Snippet<[number]> } = $props();

  /** The row the nest hangs under: the Scene's `current`, a row's place among the rows
   * shown (1–6 for the owner's plan, 1–2 for a member's, N30). Out of range: below. */
  let under = $derived.by(() => {
    const c = plan.current;
    return c !== null && c >= 1 && c <= plan.rows.length ? c - 1 : null;
  });

  /** While the nest is shroom at work, its row's arrow is the spinner (the canvas's
   * “Wiring it up”) — turning, or at rest while it waits on the network. */
  let mark = $derived.by(() => {
    const n = plan.nest;
    if (under === null) return null;
    if (n.kind === "checking" || n.kind === "check" || n.kind === "calm" || n.kind === "priming") return "turning";
    // Once the card watch is a part of its own, the row keeps its arrow (N74).
    if (n.kind === "gate") return n.watch ? null : "turning";
    if (n.kind === "token" && n.checking) return "turning";
    if (n.kind === "offline" || n.kind === "unexpected") return "resting";
    return null;
  });

  /** A nest that draws nothing under its row — a step whose spinner is the row's own mark,
   * or macOS's own prompt up — gets no row at all. Decided here, never by a CSS
   * `:has(> .nest:empty)`: macOS's WebKit didn't look again when the same nest filled in
   * place (the card step's spinner, then the card gate 26 ms later), and kept the gate's
   * line and Add credit card hidden for minutes (CHECKPOINT §10, 3.2). */
  let silent = $derived(
    (plan.nest.kind === "checking" && !!mark) || plan.nest.kind === "perm-mic" || plan.nest.kind === "perm-camera" || plan.nest.kind === "relaunching",
  );

  /** A row's line starts with its glyph; the glyph is drawn, and kept as its text. */
  const glyph = (line: string) => Array.from(line)[0] ?? "";
</script>

<section class="plan">
  <h1>{plan.header}</h1>
  <ol class="rows">
    {#each plan.rows as r, i}
      {@const p = rowParts(r)}
      <li class="row {r.glyph}" class:two={p.aside.length > 0}>
        <span class="glyph">
          {#if i === under && mark}<Spinner rest={mark === "resting"} />
          {:else if r.glyph === "done"}<svg class="icon16" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5l3.2 3.2L13 4.8" /></svg>
          {:else if r.glyph === "current"}<svg class="icon16" viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 8h10M8.5 4l4 4-4 4" /></svg>
          {:else if r.glyph === "working"}<span class="small"><Spinner /></span>
          {:else}<span class="dot" aria-hidden="true"></span>{/if}<span class="ch">{glyph(r.line)}</span>
        </span>
        <span class="text">
          <span class="title"><Runs runs={p.title} /></span>
          {#if p.aside.length}<span class="aside"><span class="sep">{p.open}</span><Runs runs={p.aside} /><span class="sep">{p.close}</span>{#if r.clock}<span class="clock">{r.clock}</span>{/if}</span>{/if}
        </span>
        {#if p.fact.length}<span class="fact"><span class="sep">{p.dash}</span><Runs runs={p.fact} /></span>{/if}
      </li>
      {#if under === i && !silent}
        <li class="under"><Nest nest={plan.nest} {send} {activates} spin={!mark} /></li>
      {/if}
      {#if afterRow}{@render afterRow(i)}{/if}
    {/each}
  </ol>
  {#if under === null}
    <div class="below"><Nest nest={plan.nest} {send} {activates} /></div>
  {/if}
</section>

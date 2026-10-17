<script lang="ts">
  // The popover renders the slice it's handed — final copy included. It composes no
  // sentences and decides nothing (ARCHITECTURE §3.5). Its look is the canvas's
  // (DEVIATIONS #45): the mushroom header with Send feedback · Library · Settings · Quit, one card of
  // picker rows, one filled action.
  import { onMount, tick } from "svelte";
  import { act, fit, key, keyed, onScene, sceneGet, type Action } from "./lib/ipc";
  import Mushroom from "./lib/Mushroom.svelte";
  import Picker from "./popover/Picker.svelte";

  type Button = { action: Action; label: string; style: string; enabled: boolean; note?: string | null };
  let p = $state<any>(null);
  let root: HTMLElement | undefined = $state();
  // Which picker's menu is open — one at a time, and only while config shows.
  let menu = $state<string | null>(null);

  const send = (a: Action) => act("popover", a);

  async function refit() {
    await tick();
    if (!root) return;
    // An open menu may hang below the card: the window grows to hold it.
    const top = root.getBoundingClientRect().top;
    const open = root.querySelector(".menu")?.getBoundingClientRect();
    const h = Math.max(root.getBoundingClientRect().height, open ? open.bottom - top + 8 : 0);
    fit(Math.ceil(h) + 2);
  }

  function onopen(which: string, on: boolean) {
    menu = on ? which : menu === which ? null : menu;
    refit();
  }

  onMount(() => {
    sceneGet<any>("popover").then((s) => { if (s) { p = s; refit(); } });
    // Its own slice only: the onboarding window's and the countdown's ride the same event.
    const un = onScene<any>((s) => {
      p = s;
      if (s?.body?.kind !== "config") menu = null;
      refit();
    }, "popover");
    const onKey = (e: KeyboardEvent) => {
      // The first key into a popover that opened by itself: it's the person's now.
      if (p && !p.activates) keyed();
      // An open menu has its own keys (the picker stops them); a stray one closes it.
      if (menu) return;
      const field = (e.target as HTMLElement)?.closest("input, select, textarea");
      if (e.key === "Enter" && !field) { e.preventDefault(); key("popover", "return"); }
      if (e.key === "Escape") { e.preventDefault(); key("popover", "esc"); }
    };
    // A click outside an open menu closes it, and nothing else.
    const onDown = (e: MouseEvent) => {
      if (menu && !(e.target as HTMLElement)?.closest(".picker.open")) { menu = null; refit(); }
    };
    // Losing the keyboard closes nothing: the popover follows the person across apps and
    // Spaces. What closes it from outside — a click somewhere else — is the runner's to see.
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown, true);
    return () => {
      un.then((f) => f());
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown, true);
    };
  });
</script>

{#snippet btn(b: Button, extra = "")}
  <button class="b {b.style} {extra}" disabled={!b.enabled} onclick={() => send(b.action)}>{b.label}</button>
  {#if b.note}<p class="note">{b.note}</p>{/if}
{/snippet}

{#snippet glyph(name: string)}
  <svg class="icon16" viewBox="0 0 16 16" aria-hidden="true">
    {#if name === "open-feedback"}<path d="M5 9V4.5a1 1 0 0 1 2 0V8M7 7.5V3.5a1 1 0 0 1 2 0v4M9 7.5V4.5a1 1 0 0 1 2 0V9.5c0 2.5-1.8 4.5-4.2 4.5-1.6 0-2.6-.8-3.4-2L2.6 9.8a1 1 0 0 1 1.6-1.2L5 9.5" /><path d="M12.8 2.3a3 3 0 0 1 1.3 2" />
    {:else if name === "open-library"}<path d="M4.5 2.5h7" /><rect x="2" y="5" width="12" height="9" rx="1.8" /><path d="M7 7.8v3.4l2.8-1.7z" />
    {:else if name === "open-settings"}<circle cx="8" cy="8" r="2" /><path d="M12.54 6.16L14.46 6.63L14.46 9.37L12.54 9.84L11.86 11.02L12.42 12.9L10.04 14.28L8.68 12.85L7.32 12.85L5.96 14.28L3.58 12.9L4.14 11.02L3.46 9.84L1.54 9.37L1.54 6.63L3.46 6.16L4.14 4.98L3.58 3.1L5.96 1.72L7.32 3.15L8.68 3.15L10.04 1.72L12.42 3.1L11.86 4.98Z" />
    {:else if name === "quit"}<path d="M8 2v6M4.5 4.5a5 5 0 1 0 7 0" />
    {:else if name === "stop"}<rect x="3.5" y="3.5" width="9" height="9" rx="2" />
    {:else if name === "restart"}<path d="M3 8a5 5 0 1 0 1.6-3.7M3 2.5v2.5h2.5" />
    {:else if name === "discard"}<path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" />
    {:else if name === "quality"}<path d="M2.5 6V3.5H5M11 3.5h2.5V6M13.5 10v2.5H11M5 12.5H2.5V10" />{/if}
  </svg>
{/snippet}

{#snippet rowbtn(b: Button)}
  <button class="rowbtn {b.style}" disabled={!b.enabled} onclick={() => send(b.action)}>
    {@render glyph(b.action.do)}
    <span class="what"><span>{b.label}</span></span>
  </button>
{/snippet}

{#if p}
  <main bind:this={root} class="pop" class:config={p.body.kind === "config"}>
    {#if p.body.kind === "config"}
      <header class="head">
        <span class="wordmark"><Mushroom size={17} /><span class="name">shroom</span></span>
        <span class="tools">
          {#each p.body.footer as f}
            <button class="icon" title={f.label} aria-label={f.label} onclick={() => send(f.action)}>{@render glyph(f.action.do)}</button>
          {/each}
        </span>
      </header>
    {/if}

    {#if p.banner}
      <section class="banner {p.banner.tone}" class:inline={p.banner.tone === "frost"}>
        <div class="bline"><span class="dot" aria-hidden="true"></span><p>{p.banner.line}</p></div>
        {#if p.banner.buttons.length}
          <div class="row">{#each p.banner.buttons as b}{@render btn(b)}{/each}</div>
        {/if}
      </section>
    {/if}

    {#if p.body.kind === "config"}
      {@const c = p.body}
      <div class="card rows">
        <Picker label={c.record_label} choices={c.sources} open={menu === "source"} onopen={(on) => onopen("source", on)} onpick={send} />
        {#snippet micSub()}
          {#if c.mic_line}{c.mic_line}{/if}
          {#if c.mic_hint}{@render btn(c.mic_hint)}{/if}
        {/snippet}
        <Picker label={c.mic_label} choices={c.mics} open={menu === "mic"} onopen={(on) => onopen("mic", on)} onpick={send} sub={c.mic_line || c.mic_hint ? micSub : undefined} />
        {#snippet cameraSub()}
          {c.camera_line}{#if c.camera_settings}{" "}{@render btn(c.camera_settings)}{/if}
        {/snippet}
        <Picker label={c.camera_label} choices={c.cameras} open={menu === "camera"} onopen={(on) => onopen("camera", on)} onpick={send} sub={c.camera_line ? cameraSub : undefined} />
        <div class="qrow">
          {@render glyph("quality")}
          <span class="qname">{c.quality_label}</span>
          <div class="seg" role="group" aria-label={c.quality_label}>
            {#each c.qualities as q}<button class:on={q.selected} aria-pressed={q.selected} onclick={() => send(q.action)}>{q.label}</button>{/each}
          </div>
        </div>
      </div>
      <div class="start">
        {@render btn(c.start, "big")}
        <p class="hint">{c.hint}</p>
        {#if c.room_line}<p class="room">{c.room_line}</p>{/if}
      </div>
    {:else if p.body.kind === "paused"}
      {@const b = p.body}
      <div class="title">
        <h1>{b.title}</h1>
        {#if b.context}<p class="context">{b.context}</p>{/if}
        {#if b.sub}<p class="quiet">{b.sub}</p>{/if}
      </div>
      {@render btn(b.resume, "big")}
      <div class="card">
        {@render rowbtn(b.stop)}
        {@render rowbtn(b.restart)}
        {@render rowbtn(b.discard)}
      </div>
    {:else if p.body.kind === "question"}
      {@const q = p.body}
      <div class="title">
        <h1>{q.title}</h1>
        <p class="context">{q.line}</p>
      </div>
      <div class="answers">
        {@render btn(q.buttons[0])}
        <span class="right">{#each q.buttons.slice(1) as b}{@render btn(b)}{/each}</span>
      </div>
    {:else if p.body.kind === "stopped"}
      {@const s = p.body}
      {#if s.title}
        <div class="title">
          <h1>{s.title}</h1>
          <p class="context">{s.line}</p>
        </div>
      {:else}
        <div class="bline told"><span class="dot amber" aria-hidden="true"></span><p class="line">{s.line}</p></div>
      {/if}
      <!-- The stopped face's link text — never a link-styled button (“Leave this space…”). -->
      {#if s.link}<p class="link">{s.link}</p>{/if}
      <div class="stack">
        {#each s.buttons as b}{@render btn(b, b.style === "red-text" ? "centered" : "big")}{/each}
      </div>
    {/if}

    {#if p.status_line}<p class="status"><span class="dot {p.status_tone ?? 'busy'}" aria-hidden="true"></span>{p.status_line}</p>{/if}
  </main>
{/if}

<style>
  :global(html), :global(body) { margin: 0; background: transparent; }
  :global(body) { font: 13px/1.45 var(--font); -webkit-user-select: none; user-select: none; }
  .pop {
    background: var(--bg); color: var(--fg); border-radius: 12px; padding: 14px; display: flex; flex-direction: column; gap: 12px;
    /* No box-shadow: the panel is cut to the card, so macOS casts it (overlay.rs). */
    border: 1px solid var(--pop-edge); box-sizing: border-box;
  }
  .pop.config { padding-top: 10px; }
  h1 { font-size: 15px; margin: 0; font-weight: 600; line-height: 1.35; }
  p { margin: 0; }
  .quiet { color: var(--muted); font-size: 12px; }
  .note { color: var(--muted); font-size: 11px; text-align: center; }

  /* the header */
  .head { display: flex; align-items: center; justify-content: space-between; min-height: 28px; }
  .wordmark .name { font-size: 14px; }
  .tools { display: flex; gap: 2px; }
  .icon { width: 28px; height: 28px; border: 0; background: transparent; color: var(--muted); padding: 0; border-radius: 6px; display: flex; align-items: center; justify-content: center; cursor: default; }
  .icon:hover { background: var(--hover); color: var(--fg); }
  .icon:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }

  /* the banners */
  .banner { border-radius: 9px; padding: 10px; display: flex; flex-direction: column; gap: 10px; }
  .banner.amber { background: var(--amber-bg); }
  .banner.frost { background: var(--frost-bg); padding: 8px 10px; flex-direction: row; flex-wrap: wrap; align-items: center; gap: 8px; }
  .banner.frost .bline { flex: 1 1 140px; align-items: center; }
  .bline { display: flex; gap: 8px; align-items: flex-start; }
  .bline .dot { margin-top: 5px; }
  .banner.frost .bline .dot { margin-top: 0; }
  .dot { width: 8px; height: 8px; border-radius: 4px; flex: none; display: block; }
  .amber .dot, .dot.amber { background: var(--amber); }
  .frost .dot { background: var(--accent); }
  .banner .row { display: flex; gap: 8px; align-items: center; justify-content: space-between; flex-wrap: wrap; }
  .banner.frost .row { justify-content: flex-end; flex-direction: row-reverse; gap: 10px; }
  .banner .b { min-height: 30px; }
  .banner.amber .b.filled { background: var(--amber-fill); border-color: var(--amber-fill); color: var(--on-amber); }
  .banner.amber .b.red-text { color: var(--red-on-amber); }
  .banner.frost .b.filled { min-height: 28px; padding: 0 12px; }
  .banner .b.link { color: var(--muted); text-decoration: none; }
  .banner .b.plain { border-color: var(--border); }

  /* config's card */
  .rows { position: relative; }
  .qrow { min-height: 38px; padding: 0 6px 0 10px; display: flex; align-items: center; gap: 10px; color: var(--fg); }
  .qrow > :global(svg) { color: var(--muted); }
  .qname { flex: 1 1 0; }
  .start { display: flex; flex-direction: column; gap: 6px; }
  .b.big { min-height: 36px; width: 100%; }
  .hint { color: var(--muted); font-size: 12px; text-align: center; }
  .room { color: var(--amber); font-size: 12px; text-align: center; }
  .rows :global(.b.link) { font-size: 12px; }

  /* the paused popover */
  .title { display: flex; flex-direction: column; gap: 4px; }
  .context { color: var(--fg-2); }
  .rowbtn { font: inherit; min-height: 36px; padding: 6px 10px; border: 0; background: transparent; color: var(--fg); display: flex; align-items: center; gap: 10px; text-align: left; cursor: default; width: 100%; }
  .rowbtn > :global(svg) { color: var(--muted); }
  .rowbtn.red-text, .rowbtn.red-text > :global(svg) { color: var(--red); }
  .rowbtn:hover:not(:disabled) { background: var(--hover); }
  .card > .rowbtn:first-child { border-radius: 9px 9px 0 0; }
  .card > .rowbtn:last-child { border-radius: 0 0 9px 9px; }
  .rowbtn:disabled { opacity: 0.45; }
  .rowbtn:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .what { display: flex; flex-direction: column; }

  /* the questions */
  .answers { display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap; }
  .answers .right { display: flex; gap: 8px; margin-left: auto; }

  /* the stops */
  .told { gap: 8px; }
  .stack { display: flex; flex-direction: column; gap: 10px; align-items: stretch; }
  .stack .b.centered { align-self: center; }
  .stack .b.plain { min-height: 36px; }
  /* The stopped face's link text — never a link-styled button (“Leave this space…”). */
  p.link { background: var(--card); box-shadow: 0 0 0 1px var(--card-edge); border-radius: 9px; padding: 10px; font-family: var(--mono); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; -webkit-user-select: text; user-select: text; }

  /* the status line */
  .status { display: flex; align-items: center; gap: 7px; color: var(--muted); font-size: 12px; }
  .status .dot { width: 6px; height: 6px; border-radius: 3px; }
  .dot.busy { background: var(--accent); }
  .dot.waiting { background: var(--dim); }
  .dot.done { background: var(--green); }
</style>

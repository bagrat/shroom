<script lang="ts">
  // One of config's picker rows (the canvas's): an icon, the pick's name, a chevron — and
  // its menu of the Scene's choices, the “none” choice last under a hairline (Camera only,
  // No mic, Camera off). System default comes first, the device it means in grey beneath and
  // a hairline after it; the row names that device (N44). It renders the choices it's handed
  // and sends the one clicked.
  import type { Action } from "../lib/ipc";
  import type { Snippet } from "svelte";

  type Choice = { action: Action; label: string; selected: boolean; enabled: boolean; detail?: string };
  let {
    label,
    choices,
    open = false,
    onopen,
    onpick,
    sub,
  }: { label: string; choices: Choice[]; open?: boolean; onopen: (on: boolean) => void; onpick: (a: Action) => void; sub?: Snippet } = $props();

  let rowEl: HTMLButtonElement | undefined = $state();
  let menuEl: HTMLDivElement | undefined = $state();

  // What each choice is: a display, a microphone, a camera — or none of them, crossed out
  // (Camera only is “no display”: the camera is the camera picker's).
  type Glyph = "display" | "display-off" | "camera" | "camera-off" | "mic" | "mic-off";
  const glyph = (a: Action): Glyph => {
    const x = a as { do: string; arg?: any };
    if (x.do === "set-source") return x.arg?.kind === "camera" ? "display-off" : "display";
    if (x.do === "set-mic-default") return "mic";
    if (x.do === "set-camera-default") return "camera";
    if (x.do === "set-mic") return x.arg == null ? "mic-off" : "mic";
    return x.arg == null ? "camera-off" : "camera";
  };
  const isDefault = (a: Action) => (a as { do: string }).do.endsWith("-default");
  const none = (a: Action) => {
    const x = a as { do: string; arg?: any };
    if (isDefault(a)) return false;
    return x.do === "set-source" ? x.arg?.kind === "camera" : x.arg == null;
  };

  const picked = $derived(choices.find((c) => c.selected) ?? null);
  const live = $derived(choices.filter((c) => c.enabled));
  // Nothing real to pick — nothing live but No mic or Camera off: the row shows it, greyed.
  // One device is still a pick (Camera only on a Mac with one camera): the row stays live.
  // Camera only is a source like a display, never "nothing".
  const off = (a: Action) => (a as { do: string }).do !== "set-source" && !isDefault(a) && (a as { arg?: unknown }).arg == null;
  const greyed = $derived(!live.some((c) => !off(c.action)));
  const shown = $derived(picked ?? choices.find((c) => none(c.action)) ?? choices[0]);

  function toggle() {
    if (!greyed) onopen(!open);
  }
  function pick(c: Choice) {
    onopen(false);
    if (c.enabled && !c.selected) onpick(c.action);
    rowEl?.focus();
  }
  function items(): HTMLButtonElement[] {
    return [...(menuEl?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
  }
  $effect(() => {
    if (open && menuEl) (items().find((b) => b.getAttribute("aria-checked") === "true") ?? items()[0])?.focus();
  });
  function onRowKey(e: KeyboardEvent) {
    if (["ArrowDown", "ArrowUp", " ", "Enter"].includes(e.key) && !greyed) {
      e.preventDefault();
      e.stopPropagation();
      onopen(true);
    }
  }
  function onMenuKey(e: KeyboardEvent) {
    const list = items();
    const at = list.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      list[(at + (e.key === "ArrowDown" ? 1 : list.length - 1)) % list.length]?.focus();
    } else if (e.key === "Escape" || e.key === "Tab") {
      e.preventDefault();
      onopen(false);
      rowEl?.focus();
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      (document.activeElement as HTMLButtonElement)?.click();
    } else return;
    // The menu's keys are the menu's: the popover's Return and Esc never see them.
    e.stopPropagation();
  }
</script>

{#snippet icon(g: Glyph)}
  <svg class="icon16" viewBox="0 0 16 16" aria-hidden="true">
    {#if g === "display" || g === "display-off"}<rect x="1.5" y="2.5" width="13" height="9" rx="1.5" /><path d="M6 14h4M8 11.5V14" />{#if g === "display-off"}<path d="M1.5 1.5l13 13" />{/if}
    {:else if g === "mic" || g === "mic-off"}<rect x="6" y="1.5" width="4" height="8" rx="2" /><path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.5" />{#if g === "mic-off"}<path d="M2 2l12 12" />{/if}
    {:else}<rect x="1.5" y="4" width="9" height="8" rx="1.8" /><path d="M10.5 7l4-2v6l-4-2" />{#if g === "camera-off"}<path d="M1.5 2.5l13 11" />{/if}{/if}
  </svg>
{/snippet}

<div class="picker" class:open>
  <button
    bind:this={rowEl}
    class="prow"
    class:greyed
    class:none={shown && none(shown.action)}
    aria-label="{label}: {shown?.detail ?? shown?.label ?? ''}"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-disabled={greyed}
    onclick={toggle}
    onkeydown={onRowKey}
  >
    {#if shown}{@render icon(glyph(shown.action))}{/if}
    <span class="name">{shown?.detail ?? shown?.label ?? ""}</span>
    {#if !greyed}<svg class="chev" viewBox="0 0 12 12" aria-hidden="true"><path d={open ? "M3 7.5l3-3 3 3" : "M3 4.5l3 3 3-3"} /></svg>{/if}
  </button>
  <!-- Its quiet line sits under the name, outside the row's button (a link inside a button
       would be two controls in one). -->
  {#if sub}<p class="sub">{@render sub()}</p>{/if}
  {#if open}
    <div class="menu" role="menu" aria-label={label} bind:this={menuEl} tabindex="-1" onkeydown={onMenuKey}>
      {#each choices as c, i}
        {#if (none(c.action) && i > 0) || (i > 0 && isDefault(choices[i - 1].action))}<div class="sep" role="separator"></div>{/if}
        <button role="menuitemradio" aria-checked={c.selected} disabled={!c.enabled} class:on={c.selected} class:two={!!c.detail} onclick={() => pick(c)}>
          {@render icon(glyph(c.action))}
          {#if c.detail}<span class="name"><span>{c.label}</span><span class="detail">{c.detail}</span></span>{:else}<span class="name">{c.label}</span>{/if}
          {#if c.selected}<svg class="check" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5l3.2 3.2L13 4.8" /></svg>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .picker { position: relative; }
  .prow { font: inherit; width: 100%; min-height: 38px; padding: 0 10px; border: 0; background: transparent; color: var(--fg); display: flex; align-items: center; gap: 10px; text-align: left; cursor: default; border-radius: 0; }
  .prow > :global(svg.icon16) { color: var(--muted); }
  .prow.greyed { color: var(--dim); }
  .prow.greyed > :global(svg.icon16) { color: var(--dim); }
  .prow.none:not(.greyed) .name { color: var(--fg-2); }
  .open .prow { background: var(--hover); }
  .prow:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; border-radius: 6px; }
  .prow .name { flex: 1 1 0; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { margin: -8px 0 0; padding: 0 10px 8px 36px; font-size: 12px; color: var(--muted); }
  .chev { width: 10px; height: 10px; fill: none; stroke: var(--muted); stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; flex: none; }
  .open .chev { stroke: var(--fg); }
  .menu { position: absolute; top: calc(100% + 4px); left: 0; right: 0; z-index: 10; background: var(--menu); border: 1px solid var(--pop-edge); border-radius: 9px; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35); padding: 4px; display: flex; flex-direction: column; outline: none; }
  .menu button { font: inherit; height: 32px; padding: 0 8px; border: 0; border-radius: 6px; background: transparent; color: var(--fg); display: flex; align-items: center; gap: 10px; text-align: left; cursor: default; }
  .menu button :global(svg.icon16) { color: var(--fg-2); }
  .menu button.on, .menu button:hover:not(:disabled), .menu button:focus-visible { background: var(--menu-on); outline: none; }
  .menu button:disabled { color: var(--dim); }
  .menu .name { flex: 1 1 0; min-width: 0; }
  .menu button.two { height: 38px; }
  .menu button.two .name { display: flex; flex-direction: column; line-height: 1.2; }
  .menu .detail { color: var(--muted); font-size: 11px; }
  .check { width: 14px; height: 14px; fill: none; stroke: var(--accent); stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; flex: none; }
  .sep { height: 1px; background: var(--line); margin: 4px 8px; }
</style>

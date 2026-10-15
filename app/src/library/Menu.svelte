<script lang="ts">
  // A row's right-click menu: the Scene's `menu` buttons, in its order, enabled as it says.
  // A pick sends its action and closes the menu; a click away, Esc or a scroll closes it.
  import { onMount } from "svelte";
  import type { Action } from "../lib/ipc";
  import type { Button } from "../onboarding/types";

  let { items, x, y, send, close }: { items: Button[]; x: number; y: number; send: (a: Action) => void; close: () => void } =
    $props();
  let el: HTMLElement | undefined = $state();
  let at = $state({ left: 0, top: 0 });

  onMount(() => {
    // Keep the menu inside the window, as a native menu would.
    const r = el!.getBoundingClientRect();
    at = {
      left: Math.max(4, Math.min(x, window.innerWidth - r.width - 4)),
      top: Math.max(4, Math.min(y, window.innerHeight - r.height - 4)),
    };
    const away = (e: Event) => {
      if (!el?.contains(e.target as Node)) close();
    };
    window.addEventListener("mousedown", away, true);
    window.addEventListener("blur", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("mousedown", away, true);
      window.removeEventListener("blur", close);
      window.removeEventListener("scroll", close, true);
    };
  });
</script>

<div class="menu" role="menu" bind:this={el} style="left: {at.left}px; top: {at.top}px">
  {#each items as b}
    <button
      type="button"
      role="menuitem"
      class="mi {b.style}"
      disabled={!b.enabled}
      onclick={() => {
        close();
        send(b.action);
      }}>{b.label}</button
    >
  {/each}
</div>

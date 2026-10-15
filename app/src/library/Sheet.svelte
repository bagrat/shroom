<script lang="ts">
  // A sheet over its window (law 9 for the destructive ones): the window dimmed, one card
  // over it — the title, its lines, the one line on why the answer is greyed when it is,
  // and the buttons right-aligned in the Scene's order — Cancel first, the answer
  // rightmost, red-outlined. There is no Return default: no
  // button takes the keyboard when the sheet opens, and the window's Return goes to the
  // Scene (whose `ret` is empty under a destructive sheet). Esc ≡ the Scene's esc (back).
  import { onMount } from "svelte";
  import type { Action } from "../lib/ipc";
  import Btn from "../onboarding/Btn.svelte";
  import type { Sheet } from "./types";

  let { sheet, send }: { sheet: Sheet; send: (a: Action) => void } = $props();
  let el: HTMLElement | undefined = $state();
  let notes = $derived([...new Set(sheet.buttons.map((b) => b.note).filter((n): n is string => !!n && n !== sheet.blocked))]);

  onMount(() => {
    // The sheet itself holds the keyboard — never a button in it.
    (document.activeElement as HTMLElement | null)?.blur?.();
    el?.focus({ preventScroll: true });
  });
</script>

<div class="scrim">
  <div class="sheet" role="alertdialog" aria-modal="true" aria-labelledby="sheet-title" tabindex="-1" bind:this={el} data-sheet={sheet.id}>
    <h2 id="sheet-title">{sheet.title}</h2>
    {#each sheet.lines as l}<p>{l}</p>{/each}
    {#if sheet.blocked}<p class="blocked">{sheet.blocked}</p>{/if}
    <!-- A greyed answer's note sits with the lines, never inside the button row. -->
    {#each notes as n}<p class="note">{n}</p>{/each}
    <div class="sheet-acts">{#each sheet.buttons as b}<Btn {b} {send} inlineNote={false} />{/each}</div>
  </div>
</div>

<script lang="ts">
  // One Scene button: its label, its look (Style), enabled as the Scene says, and the grey
  // note under a greyed control when it has one. In a row of buttons (a Library row, a
  // sheet's answers) the note isn't drawn inline (`inlineNote={false}`): the surface shows
  // it once, apart from the row, and the button carries it as its tooltip.
  import type { Action } from "../lib/ipc";
  import type { Button } from "./types";

  let { b, send, inlineNote = true }: { b: Button; send: (a: Action) => void; inlineNote?: boolean } = $props();
</script>

<button type="button" class="b {b.style}" disabled={!b.enabled} title={inlineNote ? undefined : (b.note ?? undefined)} onclick={() => send(b.action)}>{b.label}</button>
{#if b.note && inlineNote}<p class="note">{b.note}</p>{/if}

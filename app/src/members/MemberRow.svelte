<script lang="ts">
  // One Members row: its line, its fact (“keeps your Cloudflare token”), its buttons at the
  // row's end — [ New link ] on an expired invite, Remove… as red text — and the link just
  // born under it.
  import type { Action } from "../lib/ipc";
  import Btn from "../onboarding/Btn.svelte";
  import type { MemberRow } from "../settings/types";
  import Pill from "./Pill.svelte";

  let { row, send, sub = false }: { row: MemberRow; send: (a: Action) => void; sub?: boolean } = $props();
</script>

<li class="mrow" class:sub data-key={row.key}>
  <div class="mmain">
    <span class="mline">{row.line}</span>{#if row.fact}<span class="fact">{row.fact}</span>{/if}
  </div>
  {#if row.buttons.length}<div class="mbtns">{#each row.buttons as b}<Btn {b} {send} />{/each}</div>{/if}
</li>
{#if row.born}<li class="mnest"><Pill pill={row.born} {send} /></li>{/if}

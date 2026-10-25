<script lang="ts">
  // A wait's parts (N74, N78): ✓ done, a spinner underway with its clock, • still to come,
  // ✕ where a run stopped. Drawn by the setup, Settings · Space and the popover's banner.
  import Spinner from "./Spinner.svelte";
  import type { Part } from "./types";

  let { parts }: { parts: Part[] } = $props();
</script>

<ol class="parts">
  {#each parts as p}
    <li class="part {p.glyph}">
      <span class="mark">
        {#if p.glyph === "done"}<svg class="icon13" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5l3.2 3.2L13 4.8" /></svg>
        {:else if p.glyph === "working"}<Spinner />
        {:else if p.glyph === "stopped"}<svg class="icon13 cross" viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
        {:else}<span class="dot" aria-hidden="true"></span>{/if}
      </span>
      <span class="line">{p.line}</span>
      {#if p.clock}<span class="clock">{p.clock}</span>{/if}
    </li>
  {/each}
</ol>

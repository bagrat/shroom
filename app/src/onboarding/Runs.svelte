<script lang="ts">
  // A line's runs, each in the emphasis the Scene gives it (and none it doesn't).
  import type { Run } from "./types";
  let { runs }: { runs: Run[] } = $props();
</script>

{#each runs as r}{#if r.em === "strong"}<strong>{r.text}</strong>{:else if r.em === "em"}<em>{r.text}</em>{:else}{r.text}{/if}{/each}

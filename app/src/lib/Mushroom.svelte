<script lang="ts">
  // The mushroom mark, as the canvas draws it beside “shroom” (DEVIATIONS #45). Decor: the
  // wordmark beside it says the name.
  let { size = 17 }: { size?: number } = $props();
</script>

<svg class="mushroom" viewBox="18 20 84 90" aria-hidden="true" width={size} height={Math.round((size * 90) / 84)}>
  <path class="stem" d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" />
  <path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0" />
  <circle cx="50" cy="44" r="6" fill="#ECEFF4" /><circle cx="72" cy="48" r="3.6" fill="#ECEFF4" /><circle cx="33" cy="58" r="3.4" fill="#ECEFF4" />
  <circle cx="64" cy="58" r="2.6" fill="#ECEFF4" /><circle cx="88" cy="58" r="2.2" fill="#ECEFF4" />
</svg>

<style>
  .mushroom { display: block; flex: none; }
  /* The stem is pale on both themes; on light it takes a thin edge to stand off the page. */
  .stem { fill: #e5e9f0; stroke: #4c566a; stroke-width: 2; }
  @media (prefers-color-scheme: dark) { .stem { stroke: none; } }
</style>

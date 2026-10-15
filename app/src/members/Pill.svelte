<script lang="ts">
  // A born link (N28, N33): the pill — the address up to `/join/`, the code drawn as a
  // mask, since the code never leaves the runner's stash (law 13) — beside its one action,
  // [ Copy link ] (filled; the Scene reads “Copied” for a moment), and its line beneath.
  import type { Action } from "../lib/ipc";
  import Btn from "../onboarding/Btn.svelte";
  import type { LinkPill } from "../settings/types";

  let { pill, send }: { pill: LinkPill; send: (a: Action) => void } = $props();
</script>

<div class="born" data-link={pill.link}>
  <div class="pill-row">
    <span class="pill"><span class="pre">{pill.prefix}</span><span class="code" aria-hidden="true"></span></span>
    <Btn b={pill.copy} {send} />
  </div>
  <p class="quiet">{pill.line}</p>
</div>

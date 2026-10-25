<script lang="ts">
  // Settings · Space (N36, N42): the plan in this Mac's mood — inspection (every row ticked,
  // its fact on it), repair (the first broken row current, the wizard's own nest beneath
  // it), an owner-class Mac without the token, a member's Mac — then a denied microphone's
  // quiet door, and Space used (owner-class, N78) as a row of the plan's card right under
  // “Cloudflare billing set up”; beneath the rows, in one card: the “made with shroom”
  // switch (owner-class) and, on the Mac that holds the setup key, naming's (N66); the
  // exits at its foot. No invite door shows (B159): a link pasted anywhere in the tab opens its
  // invite (Settings.svelte).
  import type { Toggle } from "../onboarding/types";
  import type { Action } from "../lib/ipc";
  import Btn from "../onboarding/Btn.svelte";
  import Plan from "../onboarding/Plan.svelte";
  import type { SpacePane } from "./types";

  let { sp, send, activates }: { sp: SpacePane; send: (a: Action) => void; activates: boolean } = $props();
</script>

<!-- A denied microphone's quiet door hangs under its row — the last one, "Letting macOS
     allow recording" (N36) — before the plan's closing line. -->
{#snippet micDoor(i: number)}
  {#if sp.mic && i === sp.plan.rows.length - 1}<li class="mic"><Btn b={sp.mic} {send} /></li>{/if}
{/snippet}

<!-- Space used (N78; Bagrat, 2026-10-25): its own row under the billing row (the third),
     no glyph — a fact about the space, not a step — its value at the row's end. -->
{#snippet afterRow(i: number)}
  {#if sp.used && i === 2}<li class="used-row"><span></span><span class="used-label">{sp.used_label}</span><span class="used">{sp.used}</span></li>{/if}
  {@render micDoor(i)}
{/snippet}

<!-- A switch's row: its label is its own words; the line under them only explains. -->
{#snippet switchRow(t: Toggle, line: string | null, id: string)}
  <div class="mark" class:off={!t.enabled}>
    <div class="mark-text">
      <label for={id}>{t.label}</label>
      {#if line}<p class="quiet">{line}</p>{/if}
    </div>
    <input
      {id}
      type="checkbox"
      role="switch"
      class="switch"
      checked={t.on}
      aria-checked={t.on}
      disabled={!t.enabled}
      onclick={(e) => {
        e.preventDefault();
        send(t.action);
      }}
    />
  </div>
{/snippet}

<section class="space">
  <Plan plan={sp.plan} {send} {activates} {afterRow} />
  {#if sp.mark || sp.ai}
    <!-- One card beneath the plan: the switches' rows; the exits close it. -->
    <div class="beneath card">
      {#if sp.mark}{@render switchRow(sp.mark, sp.mark_line, "mark-switch")}{/if}
      {#if sp.ai}{@render switchRow(sp.ai, sp.ai_line, "ai-switch")}{/if}
    </div>
  {/if}
  {#if sp.exits.length}
    <div class="exits card">{#each sp.exits as b}<Btn {b} {send} />{/each}</div>
  {/if}
</section>

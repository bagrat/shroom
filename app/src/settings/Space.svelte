<script lang="ts">
  // Settings · Space (N36, N42): the plan in this Mac's mood — inspection (every row ticked,
  // its fact on it), repair (the first broken row current, the wizard's own nest beneath
  // it), an owner-class Mac without the token, a member's Mac — then a denied microphone's
  // quiet door, and beneath the rows, in one card: the “made with shroom” switch
  // (owner-class), and the exits at its foot. No invite door shows (B159): a link pasted
  // anywhere in the tab opens its invite (Settings.svelte).
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

<section class="space">
  <Plan plan={sp.plan} {send} {activates} afterRow={micDoor} />
  {#if sp.mark}
    <!-- One card beneath the plan: the switch's row; the exits close it. -->
    <div class="beneath card">
      {#if sp.mark}
        {@const t = sp.mark}
        <!-- The switch's label is its own words; the line under them only explains. -->
        <div class="mark" class:off={!t.enabled}>
          <div class="mark-text">
            <label for="mark-switch">{t.label}</label>
            {#if sp.mark_line}<p class="quiet">{sp.mark_line}</p>{/if}
          </div>
          <input
            id="mark-switch"
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
      {/if}
    </div>
  {/if}
  {#if sp.exits.length}
    <div class="exits card">{#each sp.exits as b}<Btn {b} {send} />{/each}</div>
  {/if}
</section>

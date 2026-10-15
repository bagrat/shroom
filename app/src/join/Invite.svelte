<script lang="ts">
  // The invite window's faces that aren't a plan (Region 6, N29–N31): member-welcome and
  // its own-Mac variant, join-switch-confirm (its Join greyed while uploads drain, the
  // reason one of its lines), the dead invite with its paste door, the invite already
  // here, an arrival waiting on its space (the spinner at rest). Every word is the Scene's.
  // A face with a title is drawn as the welcome's twin (its lines at the top, the buttons
  // at the window's foot); one without is its line in the middle of the window, as the
  // canvas draws a dead invite.
  import type { Action } from "../lib/ipc";
  import Btn from "../onboarding/Btn.svelte";
  import LoginBox from "../onboarding/LoginBox.svelte";
  import Spinner from "../onboarding/Spinner.svelte";
  import type { InviteBody } from "../onboarding/types";
  import Door from "./Door.svelte";

  let { b, send, activates }: { b: InviteBody; send: (a: Action) => void; activates: boolean } = $props();
</script>

<section class="invite" class:hero={!!b.title}>
  {#if b.title}<h1>{b.title}</h1>{/if}
  <div class="say">
    <div class="lines" class:wait={b.spinner}>
      {#if b.spinner}<Spinner rest />{/if}
      <div class="said">{#each b.lines as l}<p>{l}</p>{/each}</div>
    </div>
    {#if b.door}<Door line={b.door} focus={activates} />{/if}
  </div>
  {#if b.login}<LoginBox t={b.login} {send} />{/if}
  {#if b.buttons.length}
    <!-- A row of buttons keeps its order on one line: a greyed button's note (law 4's
         mid-take line) is drawn once, under the row, never between the buttons. -->
    <div class="acts">{#each b.buttons as btn}<Btn b={btn} {send} inlineNote={false} />{/each}</div>
    {#each b.buttons.filter((x) => x.note) as btn}<p class="note">{btn.note}</p>{/each}
  {/if}
</section>

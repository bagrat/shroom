<script lang="ts">
  // Settings · Members (N28, N33): the head and its one line of N5 truth; the list — You
  // (no fact) with the quiet *Add another Mac* beside it and its link's nest under it, the
  // owner's other Macs as sub-rows, then members and pending invites; then the composer,
  // the face's ask — “Who's this for?” + [ Invite ], Return in the field ≡ Invite — and its
  // line. Every word is the Scene's.
  import { onMount } from "svelte";
  import { type Action } from "../lib/ipc";
  import { typed } from "../lib/typed";
  import Btn from "../onboarding/Btn.svelte";
  import Runs from "../onboarding/Runs.svelte";
  import type { MembersPane } from "../settings/types";
  import MemberRow from "./MemberRow.svelte";
  import Pill from "./Pill.svelte";

  let { m, send, activates }: { m: MembersPane; send: (a: Action) => void; activates: boolean } = $props();

  // The name field is a typed field the Scene owns (lib/typed.ts): a refused edit is taken
  // back out, and the composer empties when the Scene says the invite was made.
  let nameEl: HTMLInputElement | undefined = $state();

  onMount(() => {
    // The composer is the ask: the field arrives focused, in a window that may take the keyboard.
    if (activates && m.name.enabled) nameEl?.focus();
  });

  // The name invites carry, being changed (B158): the field arrives focused, its Return is
  // Save (law 8); Esc is the window's — the Scene puts the name back.
  function focusOnArrival(node: HTMLInputElement) {
    if (activates) node.focus();
  }

  function fromKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.isComposing && m.from.kind === "changing") {
      e.preventDefault();
      e.stopPropagation();
      if (m.from.save.enabled) send(m.from.save.action);
    }
  }

  function nameKey(e: KeyboardEvent) {
    // Return in the field is Invite — the field owns it (law 8).
    if (e.key === "Enter" && !e.isComposing) {
      e.preventDefault();
      e.stopPropagation();
      if (m.invite.enabled) send(m.invite.action);
    }
  }
</script>

<section class="members">
  <h1>{m.title}</h1>
  <p class="truth">{m.line}</p>
  <ul class="mrows card">
    <li class="mrow you">
      <div class="mmain"><span class="mline">{m.you}</span></div>
      <div class="mbtns"><Btn b={m.add_mac} {send} /></div>
    </li>
    {#if m.mac_link}<li class="mnest"><Pill pill={m.mac_link} {send} /></li>{/if}
    {#each m.macs as row (row.key)}<MemberRow {row} {send} sub />{/each}
    {#each m.rows as row (row.key)}<MemberRow {row} {send} />{/each}
  </ul>
  <div class="composer">
    <input
      bind:this={nameEl}
      class="who"
      type="text"
      data-field="invite-name"
      placeholder={m.name.placeholder}
      aria-label={m.name.placeholder}
      disabled={!m.name.enabled}
      autocomplete="off"
      spellcheck="false"
      use:typed={{ field: "invite-name", scene: m.name }}
      onkeydown={nameKey}
    />
    <Btn b={m.invite} {send} />
  </div>
  <p class="quiet invite-line">{m.invite_line}</p>
  {#if m.from.kind === "shown"}
    <p class="quiet from"><span><Runs runs={m.from.line.runs} /></span> <Btn b={m.from.change} {send} /></p>
  {:else}
    <div class="quiet from changing">
      <label for="from-name">{m.from.label}</label>
      <input
        id="from-name"
        class="from-name"
        type="text"
        data-field="from-name"
        maxlength="40"
        autocomplete="off"
        spellcheck="false"
        use:typed={{ field: "from-name", scene: m.from.field }}
        use:focusOnArrival
        onkeydown={fromKey}
      />
      <Btn b={m.from.save} {send} />
    </div>
  {/if}
</section>

<script lang="ts">
  // Settings · General (N36): one card — the login box (reads the system's truth, as the
  // Scene says), the update row (its line, its button, What's new), the help row (Copy
  // diagnostics, Report a problem…, B180) — then the line under it, whose doors are runs of
  // the line itself; your coding agent (N62); Uninstall shroom… at the foot. Your videos is
  // a tab of its own (Videos.svelte, N79).
  import type { Action } from "../lib/ipc";
  import Btn from "../onboarding/Btn.svelte";
  import type { Button } from "../onboarding/types";
  import type { GeneralPane } from "./types";

  let { g, send }: { g: GeneralPane; send: (a: Action) => void } = $props();
  let agentRow: HTMLLabelElement | undefined = $state();
  // The command's settings link (spec N62, DECISIONS B216): while the Scene lights the
  // switch's row, the row is brought into view; the light fades as the Scene puts it out.
  $effect(() => {
    if (g.agent.lit) agentRow?.scrollIntoView({ block: "nearest" });
  });

  /** The help line cut at each door's label, in order; a door the line doesn't carry
   * follows it. */
  function cut(line: string, links: Button[]): (string | Button)[] {
    const out: (string | Button)[] = [];
    let at = 0;
    const rest: Button[] = [];
    for (const b of links) {
      const i = line.indexOf(b.label, at);
      if (i < 0) {
        rest.push(b);
        continue;
      }
      if (i > at) out.push(line.slice(at, i));
      out.push(b);
      at = i + b.label.length;
    }
    if (at < line.length) out.push(line.slice(at));
    return [...out, ...rest];
  }
  let help = $derived(cut(g.help, g.help_links));
</script>

<section class="general">
  <div class="card">
    <!-- The login box, drawn as a switch: its label at the row's start, the switch at its end. -->
    <label class="srow login" class:off={!g.login.enabled}>
      <span class="grow">{g.login.label}</span>
      <input
        type="checkbox"
        class="switch"
        checked={g.login.on}
        disabled={!g.login.enabled}
        onclick={(e) => {
          e.preventDefault();
          send(g.login.action);
        }}
      />
    </label>

    <div class="srow updates">
      <span class="grow lbl">{g.updates_label}</span>
      <div class="update">
        <p class="update-line">{g.update_line}</p>
        {#if g.update || g.whats_new}
          <div class="acts">
            {#if g.update}<Btn b={g.update} {send} />{/if}
            {#if g.whats_new}<Btn b={g.whats_new} {send} />{/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="srow help-row">
      <span class="grow">{g.help_label}</span>
      <div class="acts">
        <Btn b={g.copy_diagnostics} {send} />
        <Btn b={g.report} {send} />
      </div>
    </div>
  </div>
  {#if g.sent}<p class="sent" role="status">{g.sent}</p>{/if}
  <p class="help">
    {#each help as part}{#if typeof part === "string"}{part}{:else}<button
          type="button"
          class="b {part.style} inline-link"
          disabled={!part.enabled}
          onclick={() => send(part.action)}>{part.label}</button
        >{/if}{/each}
  </p>
  <!-- Your coding agent (spec N62, DECISIONS B219): the prompt to paste and its Copy, what
       the agent reads (Read it first), and whether it may start and stop recordings. -->
  <div class="card agent">
    <div class="agent-top">
      <div class="agent-head">
        <span class="grow agent-label">{g.agent.label}</span>
        <Btn b={g.agent.read_first} {send} />
      </div>
      <div class="prompt">
        <code>{g.agent.prompt}</code>
        <Btn b={g.agent.copy} {send} />
      </div>
      <p class="agent-line">{g.agent.line}</p>
    </div>
    <label class="srow agent-switch" class:off={!g.agent.recording.enabled} class:lit={g.agent.lit} bind:this={agentRow}>
      <span class="grow">{g.agent.recording.label}</span>
      <input
        type="checkbox"
        class="switch"
        checked={g.agent.recording.on}
        disabled={!g.agent.recording.enabled}
        onclick={(e) => {
          e.preventDefault();
          send(g.agent.recording.action);
        }}
      />
    </label>
  </div>
  <!-- Uninstall shroom…, quiet text at the tab's foot (B135); greyed mid-take with its line. -->
  <div class="uninstall"><Btn b={g.uninstall} {send} /></div>
</section>

<script lang="ts">
  // The plan's nest: what sits under the current row (or below the plan once every row is
  // done). Every kind renders only what the Scene says; action buttons sit flush-left,
  // wizard-wide (N15). `spin`: whether the nest draws its own spinner — under its row the
  // row's mark is the spinner instead (Plan), so the nest draws none.
  import { fade } from "svelte/transition";
  import { type Action } from "../lib/ipc";
  import { typed } from "../lib/typed";
  import Door from "../join/Door.svelte";
  import Btn from "./Btn.svelte";
  import PasteField from "./PasteField.svelte";
  import Spinner from "./Spinner.svelte";
  import type { Nest } from "./types";

  let {
    nest,
    send,
    activates,
    spin = true,
  }: { nest: Nest; send: (a: Action) => void; activates: boolean; spin?: boolean } = $props();

  // The link name: a typed field the Scene owns (lib/typed.ts). Every keystroke goes to the
  // core, which filters it (lowercase, charset); a character it refused is taken back out.
  let nameEl: HTMLInputElement | undefined = $state();
  let nameShown = $derived(nest.kind === "name");
  $effect(() => {
    // The row is the ask: the field takes the keyboard when it appears, in a window that may.
    if (nameShown && activates && nameEl) nameEl.focus();
  });

  function nameKey(e: KeyboardEvent) {
    if (nest.kind !== "name") return;
    // Return in the field is Claim — the field owns it (law 8), never the window default.
    if (e.key === "Enter" && !e.isComposing) {
      e.preventDefault();
      e.stopPropagation();
      if (nest.claim.enabled) send(nest.claim.action);
    }
  }
</script>

<div class="nest {nest.kind}" data-nest={nest.kind}>
  {#if nest.kind === "account"}
    <p>{nest.question}</p>
    <div class="acts">{#each nest.buttons as b}<Btn {b} {send} />{/each}</div>
  {:else if nest.kind === "signing-up"}
    <p>{nest.line}</p>
    <div class="acts">{#each nest.buttons as b}<Btn {b} {send} />{/each}</div>
  {:else if nest.kind === "token"}
    <p>{nest.line}</p>
    <div class="acts"><Btn b={nest.create} {send} /></div>
    <div class="pasterow">
      <!-- Still while shroom checks the paste: no caret, nothing to type (N45). -->
      <PasteField field="token" placeholder={nest.placeholder} held={nest.held} disabled={nest.checking} />
      {#if nest.checking && spin}<Spinner />{/if}
    </div>
    {#if nest.error}<p class="err">{nest.error}</p>{/if}
    {#if nest.quiet}<p class="quiet">{nest.quiet}</p>{/if}
  {:else if nest.kind === "pick-account"}
    <p>{nest.line}</p>
    <div class="choices" role="radiogroup">
      {#each nest.accounts as c}
        <label class="radio" class:off={!c.enabled}>
          <input
            type="radio"
            name="account"
            checked={c.selected}
            disabled={!c.enabled}
            onclick={(e) => { e.preventDefault(); send(c.action); }}
          />
          <span>{c.label}</span>
        </label>
      {/each}
    </div>
    <div class="acts"><Btn b={nest.choose} {send} /></div>
  {:else if nest.kind === "checking"}
    {#if spin}<div class="wait"><Spinner /></div>{/if}
  {:else if nest.kind === "gate"}
    <p>{nest.line}</p>
    {#if nest.button || spin}
      <div class="acts">
        {#if nest.button}<Btn b={nest.button} {send} />{/if}
        {#if spin}<Spinner />{/if}
      </div>
    {/if}
    {#if nest.quiet}<p class="quiet">{nest.quiet}</p>{/if}
  {:else if nest.kind === "name"}
    <div class="url">
      <span class="pre">{nest.prefix}</span><input
        bind:this={nameEl}
        class="name field-input"
        class:taken={!!nest.taken}
        type="text"
        style="width: calc({Math.max(nest.value.length + 1, 16)}ch + 22px)"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        aria-invalid={!!nest.taken}
        use:typed={{ field: "name", scene: nest }}
        onkeydown={nameKey}
      /><span class="suf">{nest.suffix}</span>
    </div>
    {#if nest.taken}<p class="err">{nest.taken}</p>{/if}
    {#if nest.unexpected}<p class="amber">{nest.unexpected}</p>{/if}
    <p>{nest.line}</p>
    <div class="acts">
      <Btn b={nest.claim} {send} />
      {#if nest.hint}<p class="quiet hint" out:fade={{ duration: 600 }}>{nest.hint}</p>{/if}
    </div>
  {:else if nest.kind === "calm"}
    <div class="wait">{#if spin}<Spinner />{/if}<p>{nest.line}</p></div>
  {:else if nest.kind === "found"}
    <p>{nest.line}</p>
  {:else if nest.kind === "offline"}
    <div class="wait">{#if spin}<Spinner rest />{/if}<p class="amber">{nest.line}</p></div>
  {:else if nest.kind === "unexpected"}
    <div class="wait">{#if spin}<Spinner rest />{/if}<p class="amber">{nest.line}</p></div>
    <div class="acts"><Btn b={nest.button} {send} /></div>
  {:else if nest.kind === "done"}
    {#each nest.lines as l}<p>{l}</p>{/each}
    <div class="acts"><Btn b={nest.allow} {send} /></div>
  {:else if nest.kind === "perm-mic"}
    <!-- macOS's own microphone prompt is up: nothing of ours. -->
  {:else if nest.kind === "perm-camera"}
    <!-- macOS's own camera prompt is up: nothing of ours. -->
  {:else if nest.kind === "perm-screen"}
    <p>{nest.line}</p>
    <div class="quiet inline">{nest.quiet} <Btn b={nest.reopen} {send} /></div>
    {#if nest.settings}<div class="quiet inline">{nest.settings.quiet} <Btn b={nest.settings.button} {send} /></div>{/if}
  {:else if nest.kind === "relaunching"}
    <!-- The app is quitting to reopen: nothing to render. -->
  {:else if nest.kind === "priming"}
    <div class="wait">{#if spin}<Spinner />{/if}<p>{nest.line}</p></div>
  {:else if nest.kind === "all-set"}
    <div class="said">{#each nest.lines as l}<p>{l}</p>{/each}</div>
    <div class="acts">{#each nest.buttons as b}<Btn {b} {send} />{/each}</div>
  {:else if nest.kind === "ready"}
    <p>{nest.line}</p>
  {:else if nest.kind === "name-changed"}
    <p>{nest.line}</p>
  {:else if nest.kind === "storage-gone"}
    <p>{nest.line}</p>
    <div class="acts">{#each nest.buttons as b}<Btn {b} {send} />{/each}</div>
  {:else if nest.kind === "no-token"}
    <p>{nest.line}</p>
    {#if nest.button}<div class="acts"><Btn b={nest.button} {send} /></div>{/if}
  {:else if nest.kind === "screen-off"}
    <div class="acts"><Btn b={nest.button} {send} /></div>
  {:else if nest.kind === "join-refused"}
    <p>{nest.line}</p>
    {#if nest.door}<Door line={nest.door} focus={activates} />{/if}
  {/if}
</div>

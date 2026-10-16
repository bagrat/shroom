<script lang="ts">
  // Report a problem's sheet over Settings · General (B180): the title and its line, the
  // description and the reply-to (typed fields the Scene owns, lib/typed.ts), “See exactly
  // what's sent” opening the whole report as it goes, the line a send that didn't go
  // through leaves with its mail link, then Cancel and Send. Return types a new line in
  // the description; ⌘Return asks for Send, which the core takes only when it offers it.
  import { onMount } from "svelte";
  import type { Action } from "../lib/ipc";
  import { typed } from "../lib/typed";
  import Btn from "../onboarding/Btn.svelte";
  import type { Sheet } from "../library/types";
  import type { ReportForm } from "./types";

  let { sheet, form, send }: { sheet: Sheet; form: ReportForm; send: (a: Action) => void } = $props();
  let words: HTMLTextAreaElement | undefined = $state();
  let answer = $derived(sheet.buttons[sheet.buttons.length - 1]);

  onMount(() => words?.focus({ preventScroll: true }));

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter" && e.metaKey && answer?.enabled) {
      e.preventDefault();
      send(answer.action);
    }
  }
</script>

<div class="scrim">
  <div class="sheet wide" role="dialog" aria-modal="true" aria-labelledby="sheet-title" tabindex="-1" data-sheet={sheet.id}>
    <h2 id="sheet-title">{sheet.title}</h2>
    {#each sheet.lines as l}<p>{l}</p>{/each}
    <textarea
      class="words"
      aria-label={form.words.placeholder}
      placeholder={form.words.placeholder}
      disabled={!form.words.enabled}
      bind:this={words}
      onkeydown={onKey}
      use:typed={{ field: "report-words", scene: form.words }}
    ></textarea>
    <label class="reply">
      <span>{form.reply_label}</span>
      <input type="email" autocomplete="email" disabled={!form.reply_to.enabled} onkeydown={onKey} use:typed={{ field: "report-reply-to", scene: form.reply_to }} />
    </label>
    <details class="what">
      <summary>{form.disclosure}</summary>
      {#if form.preview}<pre>{form.preview}</pre>{/if}
    </details>
    {#if form.preparing}<p class="preparing">{form.preparing}</p>{/if}
    {#if form.failed}
      <p class="failed" role="status">
        {form.failed.slice(0, form.mail ? Math.max(0, form.failed.indexOf(form.mail.label)) : form.failed.length)}{#if form.mail}<button
            type="button"
            class="b link inline"
            onclick={() => form.mail && send(form.mail.action)}>{form.mail.label}</button
          >{form.failed.slice(form.failed.indexOf(form.mail.label) + form.mail.label.length)}{/if}
      </p>
    {/if}
    <div class="sheet-acts">{#each sheet.buttons as b}<Btn {b} {send} inlineNote={false} />{/each}</div>
  </div>
</div>

<style>
  .sheet.wide {
    width: min(520px, 100%);
    max-height: calc(100vh - 72px);
    overflow-y: auto;
  }
  .words,
  .reply input {
    font: inherit;
    font-size: 13px;
    color: var(--fg);
    background: var(--well, var(--bg));
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    box-sizing: border-box;
    width: 100%;
  }
  .words {
    height: 76px;
    resize: none;
    margin-top: 4px;
  }
  .reply {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: var(--fg-2);
  }
  .reply input {
    height: 30px;
  }
  .what summary {
    cursor: default;
    font-size: 13px;
    color: var(--muted);
  }
  .what pre {
    margin: 8px 0 0;
    max-height: 140px;
    overflow: auto;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font: 11px/1.5 ui-monospace, Menlo, monospace;
    color: var(--fg-2);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .failed {
    color: var(--fg);
  }
  .preparing {
    font-size: 12px;
    color: var(--muted);
  }
  .b.link.inline {
    font: inherit;
    color: var(--frost);
    padding: 0;
    border: 0;
    background: transparent;
    text-decoration: underline;
  }
</style>

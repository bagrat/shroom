<script lang="ts">
  // The Send feedback window (N60) renders the slice it's handed — final copy included. It
  // composes no sentences and decides nothing (ARCHITECTURE §3.5). The form: its line, the
  // words and the reply-to (typed fields the Scene owns, lib/typed.ts), the log box and its
  // line, “See exactly what's sent”, the line a send that didn't go through leaves with its
  // mail link, then Cancel and Send. Once sent, the form gives way to the sent face and Done.
  // Return types a new line in the words; ⌘Return asks for Send, which the core takes only
  // when it offers it.
  import { onMount } from "svelte";
  import { act, key, onScene, sceneGet, type Action } from "./lib/ipc";
  import { typed } from "./lib/typed";
  import Btn from "./onboarding/Btn.svelte";
  import { isFeedback, type FeedbackWindow } from "./feedback/types";

  let w = $state.raw<FeedbackWindow | null>(null);
  let words: HTMLTextAreaElement | undefined = $state();
  const send = (a: Action) => act("feedback", a);
  const take = (s: unknown) => {
    if (isFeedback(s)) w = s;
  };
  let answer = $derived(w?.body.buttons[w.body.buttons.length - 1]);

  // A text field owns its Return (law 8); the checkbox doesn't.
  const typing = (t: EventTarget | null) => t instanceof HTMLElement && !!t.closest('input:not([type="checkbox"]), textarea');

  $effect(() => {
    if (w?.activates && words && !w.body.sent) words.focus({ preventScroll: true });
  });

  onMount(() => {
    sceneGet<FeedbackWindow>("feedback").then(take);
    const un = onScene<FeedbackWindow>(take, "feedback");
    const onKey = (e: KeyboardEvent) => {
      if (e.isComposing || !w) return;
      if (e.key === "Enter" && e.metaKey) {
        e.preventDefault();
        if (!w.body.sent && answer?.enabled) send(answer.action);
        return;
      }
      if (e.key === "Enter" && !typing(e.target)) {
        e.preventDefault();
        if (w.ret) key("feedback", "return");
      }
      if (e.key === "Escape") {
        e.preventDefault();
        if (w.esc) key("feedback", "esc");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      un.then((f) => f());
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

{#if w}
  {@const b = w.body}
  <main class="fb" data-state={w.id}>
    {#if b.sent}
      <div class="sent" role="status">
        <span class="tick" aria-hidden="true"
          ><svg viewBox="0 0 16 16" class="icon16"><path d="M3.5 8.4l3 2.9 6-6.6" /></svg></span
        >
        <h1>{b.sent.title}</h1>
        <p>{b.sent.line}</p>
      </div>
      <div class="acts"><Btn b={b.sent.done} {send} /></div>
    {:else}
      <p class="line">{b.line}</p>
      <textarea
        class="words"
        aria-label={b.title}
        placeholder={b.words.placeholder}
        disabled={!b.words.enabled}
        bind:this={words}
        use:typed={{ field: "feedback-words", scene: b.words }}
      ></textarea>
      <label class="reply">
        <span>{b.reply_label}</span>
        <input class="field-input" type="email" autocomplete="email" disabled={!b.reply_to.enabled} use:typed={{ field: "feedback-reply-to", scene: b.reply_to }} />
      </label>
      <label class="log" class:off={!b.log.enabled}>
        <input
          type="checkbox"
          checked={b.log.on}
          disabled={!b.log.enabled}
          onclick={(e) => {
            e.preventDefault();
            send(b.log.action);
          }}
        />
        <span>{b.log.label}</span>
      </label>
      <p class="note">{b.log_note}</p>
      <details class="what">
        <summary>{b.disclosure}</summary>
        {#if b.preview}<pre>{b.preview}</pre>{/if}
      </details>
      {#if b.preparing}<p class="preparing">{b.preparing}</p>{/if}
      {#if b.failed}
        <p class="failed" role="status">
          {b.failed.slice(0, b.mail ? Math.max(0, b.failed.indexOf(b.mail.label)) : b.failed.length)}{#if b.mail}<button
              type="button"
              class="b link inline"
              onclick={() => b.mail && send(b.mail.action)}>{b.mail.label}</button
            >{b.failed.slice(b.failed.indexOf(b.mail.label) + b.mail.label.length)}{/if}
        </p>
      {/if}
      <div class="grow"></div>
      <div class="acts">{#each b.buttons as x}<Btn b={x} {send} inlineNote={false} />{/each}</div>
    {/if}
  </main>
{/if}

<style>
  :global(html),
  :global(body),
  :global(#app) {
    height: 100%;
    margin: 0;
  }
  .fb {
    box-sizing: border-box;
    height: 100%;
    padding: 8px 24px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    font: 13px/1.5 var(--font);
    -webkit-font-smoothing: antialiased;
    color: var(--fg);
    background: var(--bg);
  }
  p {
    margin: 0;
  }
  .line {
    color: var(--fg-2);
  }
  .words,
  .reply input {
    font: inherit;
    font-size: 13px;
    color: var(--fg);
    background: var(--field);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    box-sizing: border-box;
    width: 100%;
  }
  .words {
    height: 84px;
    resize: none;
  }
  .reply {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--fg-2);
  }
  .reply input {
    height: 30px;
    padding: 0 10px;
  }
  .log {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .log input {
    margin: 0;
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }
  .log.off {
    opacity: 0.45;
  }
  .note {
    margin-top: -7px;
    padding-left: 24px;
    font-size: 12px;
    color: var(--muted);
  }
  .what summary {
    cursor: default;
    color: var(--muted);
  }
  .what pre {
    margin: 8px 0 0;
    max-height: 110px;
    overflow: auto;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font: 11px/1.5 ui-monospace, Menlo, monospace;
    color: var(--fg-2);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
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
  .grow {
    flex: 1 1 0;
  }
  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .sent {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    text-align: center;
  }
  .sent h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .sent p {
    color: var(--fg-2);
  }
  .tick {
    width: 44px;
    height: 44px;
    border-radius: 22px;
    background: var(--frost-bg);
    color: var(--frost);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .tick .icon16 {
    width: 22px;
    height: 22px;
  }
</style>

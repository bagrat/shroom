<script lang="ts">
  // A paste-only field (the token, the invite link). Validates the moment the paste lands —
  // no Continue. The pasted text never lands in the field or in any state here: the handler
  // hands it to the runner, which keeps it (law 13), and the field shows only what the Scene
  // says — empty, or, while the Scene says a paste is held, a row of dots. The dots are a
  // drawing of “taken”: one fixed row whatever was pasted, never its text and never its
  // length (DECISIONS B104). Typing does nothing; there is nothing to type.
  import { pasteSecret } from "../lib/ipc";

  let {
    field,
    placeholder,
    focus = false,
    held = false,
    disabled = false,
    label,
  }: { field: "token" | "invite"; placeholder: string; focus?: boolean; held?: boolean; disabled?: boolean; label?: string } = $props();
  let el: HTMLInputElement | undefined = $state();

  /** A held paste, drawn: the same row for every paste. */
  const DOTS = "•".repeat(16);
  const shown = $derived(held ? DOTS : "");

  function onpaste(e: ClipboardEvent) {
    e.preventDefault();
    // Back to what the Scene says, synchronously: nothing pasted ever shows.
    (e.currentTarget as HTMLInputElement).value = shown;
    const text = e.clipboardData?.getData("text/plain") ?? "";
    if (text.trim()) pasteSecret(field, text);
  }

  $effect(() => {
    if (el) el.value = shown;
  });
  $effect(() => {
    if (focus && el) el.focus();
  });
</script>

<input
  bind:this={el}
  class="paste field-input"
  class:held
  data-field={field}
  type="text"
  {placeholder}
  {disabled}
  aria-label={label ?? placeholder}
  autocomplete="off"
  autocapitalize="off"
  spellcheck="false"
  {onpaste}
  onbeforeinput={(e) => e.preventDefault()}
  ondrop={(e) => e.preventDefault()}
/>

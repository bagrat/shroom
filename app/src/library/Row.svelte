<script lang="ts">
  // One Library row (N35): the poster, then title, then date · length, then a state word
  // only when the Scene gives one (silence means live); the thin bar under an uploading row;
  // the amber dot on a recovered take pinned on top. Its buttons: the ones that show on
  // hover or selection, then the always-visible ones at the row's end. Gestures map to the Scene's
  // actions: a click selects, ⌘-click toggles, ⇧-click extends, a click on the title
  // renames (after the double-click interval, so a double-click is Open page instead),
  // right-click opens the Scene's menu.
  import { type Action } from "../lib/ipc";
  import { typed } from "../lib/typed";
  import Btn from "../onboarding/Btn.svelte";
  import type { Row } from "./types";

  let {
    row,
    send,
    activates,
    menu,
    away,
  }: {
    row: Row;
    send: (a: Action) => void;
    activates: boolean;
    menu: (row: Row, x: number, y: number) => void;
    /** The title field lost the keyboard: a click away saves (the Scene's ret). */
    away: () => void;
  } = $props();

  // macOS's double-click interval, near enough: a title click waits this long to rename.
  const DOUBLE = 350;
  let pending: ReturnType<typeof setTimeout> | null = null;

  const inControl = (t: EventTarget | null) => t instanceof HTMLElement && !!t.closest("button, input");

  function onclick(e: MouseEvent) {
    if (inControl(e.target)) return;
    const g = row.gestures;
    if (e.detail >= 2) {
      if (pending) clearTimeout(pending);
      pending = null;
      if (e.detail === 2 && g.double_click && !e.metaKey && !e.shiftKey) send(g.double_click);
      return;
    }
    if (e.metaKey) {
      if (g.cmd_click) send(g.cmd_click);
      return;
    }
    if (e.shiftKey) {
      if (g.shift_click) send(g.shift_click);
      return;
    }
    if (g.click) send(g.click);
    const onTitle = e.target instanceof HTMLElement && !!e.target.closest(".title");
    if (onTitle && g.title_click) {
      const a = g.title_click;
      pending = setTimeout(() => {
        pending = null;
        send(a);
      }, DOUBLE);
    }
  }

  function oncontextmenu(e: MouseEvent) {
    if (!row.menu.length) return;
    e.preventDefault();
    menu(row, e.clientX, e.clientY);
  }

  // The title being renamed: a typed field the Scene owns (lib/typed.ts) — every keystroke
  // goes to the core, and the Scene's value comes back in once it has counted them. Return
  // and Esc go to the Scene (its ret saves, its esc cancels); a click away saves too.
  //
  // The field takes the keyboard with the old title selected, ready to retype — once for
  // the field, when the rename opens (or the moment its window may first take the
  // keyboard), and never again while it's being typed in: a Scene arrives with every key,
  // and selecting again on each would have the next key replace everything typed before
  // it. It remembers that it has, so nothing about when it's called can make it twice.
  // (On the input it comes after `use:typed`: the title has to be in the field first.)
  function retype(node: HTMLInputElement, may: boolean) {
    let done = false;
    const go = (may: boolean) => {
      if (!may || done) return;
      done = true;
      node.focus();
      node.select();
    };
    go(may);
    return { update: go };
  }

  // The poster (N35), from shroom's own scheme; a neutral placeholder until it's there,
  // or when there isn't one. Decorative: the title says what the row is.
  let posterFailed = $state(false);
  let posterSrc = $derived(row.poster ? `shroom-poster://localhost/${row.poster.split("/").map(encodeURIComponent).join("/")}` : null);
  $effect(() => {
    void posterSrc;
    posterFailed = false;
  });
</script>

<!-- The keys (⌘C, Delete, Return, Esc) are the window's, sent to the Scene from Library.svelte. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<li
  class="lrow"
  class:selected={row.selected}
  class:pinned={row.pinned}
  class:tappable={!!row.gestures.click}
  data-key={row.key}
  aria-selected={row.selected}
  role="option"
  {onclick}
  {oncontextmenu}
>
  <span class="lead" aria-hidden="true">{#if row.pinned}<span class="dot {row.tone ?? 'amber'}"></span>{/if}</span>
  <span class="thumb" aria-hidden="true"
    >{#if posterSrc && !posterFailed}<img src={posterSrc} alt="" draggable="false" onerror={() => (posterFailed = true)} />{:else}<svg viewBox="0 0 16 16"
        ><path d="M5 3.5v9l7.5-4.5z" /></svg
      >{/if}</span
  >
  <div class="main">
    <div class="line1">
      {#if row.rename}
        <input
          class="title-edit"
          type="text"
          placeholder={row.rename.placeholder}
          aria-label={row.rename.placeholder}
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          data-field="title"
          use:typed={{ field: "title", scene: row.rename }}
          use:retype={activates}
          onblur={away}
        />
      {:else if row.title !== null}
        {#if row.kept}
          <!-- N74: pinned to keep — the small pin says so, to VoiceOver and on hover. -->
          <svg class="kept" viewBox="0 0 16 16" role="img" aria-label={row.kept}><title>{row.kept}</title><path
              d="M9.8 1.5l4.7 4.7-1.2.5-2.5 2.5.2 3.1-1.3 1.3-2.7-2.7-3.9 3.9H2.5v-.6l3.9-3.9-2.7-2.7 1.3-1.3 3.1.2 2.5-2.5z"
            /></svg
          >
        {/if}
        <span class="title">{row.title}</span>
      {:else if row.state}
        <span class="title state {row.tone ?? ''}">{row.state}</span>
      {/if}
    </div>
    <div class="line2">
      <span class="meta">{row.meta}</span>{#if row.state && row.title !== null}<span class="state {row.tone ?? ''}">{row.state}</span>{/if}{#if row.going}<span class="state amber">{row.going}</span>{/if}{#if row.member}<span class="member">{row.member}</span>{/if}
    </div>
    {#if row.progress !== null}
      <div class="bar" role="progressbar" aria-valuenow={row.progress} aria-valuemin={0} aria-valuemax={100}>
        <span style="width: {Math.max(0, Math.min(100, row.progress))}%"></span>
      </div>
    {/if}
  </div>
  <div class="tail">
    <!-- The hover pair sits left of the always-visible buttons, so the row's end keeps one
         column down the list whether or not a row shows its hover pair. -->
    <!-- A greyed button's note shows once, in the window (Library.svelte), never in the row. -->
    {#if row.hover.length}<div class="hover">{#each row.hover as b}<Btn {b} {send} inlineNote={false} />{/each}</div>{/if}
    {#if row.buttons.length}<div class="always">{#each row.buttons as b}<Btn {b} {send} inlineNote={false} />{/each}</div>{/if}
  </div>
</li>

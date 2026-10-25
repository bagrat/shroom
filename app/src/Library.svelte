<script lang="ts">
  // The Library window (N35) renders the slice it's handed — final copy included. It
  // composes no sentences and decides nothing (ARCHITECTURE §3.5): every gesture, key and
  // field goes back to the core as the Scene's action, `key()` or `field()`.
  import { onMount } from "svelte";
  import { act, key, onScene, sceneGet, type Action } from "./lib/ipc";
  import { typed } from "./lib/typed";
  import Btn from "./onboarding/Btn.svelte";
  import Cleanup from "./library/Cleanup.svelte";
  import Menu from "./library/Menu.svelte";
  import Row from "./library/Row.svelte";
  import Sheet from "./library/Sheet.svelte";
  import { isLibrary, type LibraryWindow, type Row as RowT } from "./library/types";
  import type { Button } from "./onboarding/types";

  let w = $state.raw<LibraryWindow | null>(null);
  // Whether this window may take the keyboard, as a value of its own: read straight off
  // `w`, it would change with every Scene, and every field that takes the keyboard when it
  // appears would take it again each time.
  let activates = $derived(!!w?.activates);
  // A greyed row button's note (law 4's line mid-take, the updating line), once for the
  // window — never repeated in every row.
  let notes = $derived(
    w ? [...new Set(w.body.rows.flatMap((r) => [...r.buttons, ...r.hover]).map((b) => b.note).filter((n): n is string => !!n))] : [],
  );
  let menu = $state.raw<{ items: Button[]; x: number; y: number } | null>(null);

  const send = (a: Action) => act("library", a);
  const take = (s: unknown) => {
    if (!isLibrary(s)) return;
    w = s;
    if (s?.body.sheet) menu = null;
  };

  $effect(() => {
    if (w) document.title = w.body.title;
  });

  const isText = (t: EventTarget | null): t is HTMLInputElement =>
    t instanceof HTMLInputElement && !["checkbox", "radio", "button"].includes(t.type);

  function away() {
    // A click away from the title field saves it — when saving is what the Scene offers.
    if (w?.ret && !w.body.sheet) key("library", "return");
  }

  onMount(() => {
    sceneGet<LibraryWindow>("library").then(take);
    const un = onScene<LibraryWindow>(take, "library");
    const onKey = (e: KeyboardEvent) => {
      if (e.isComposing || !w) return;
      const t = e.target;
      const inSearch = t instanceof HTMLElement && t.dataset.field === "library-search";
      if (e.key === "Escape") {
        e.preventDefault();
        if (menu) {
          menu = null;
          return;
        }
        // The search field's own Esc clears it, as a macOS search field does — there and
        // then, as an edit like any typed one (the field sends and counts it, lib/typed.ts).
        if (inSearch && (t as HTMLInputElement).value) {
          (t as HTMLInputElement).value = "";
          t.dispatchEvent(new Event("input", { bubbles: true }));
          return;
        }
        if (w.esc) key("library", "esc");
        return;
      }
      if (e.key === "Enter") {
        // No Return default but the Scene's: under a destructive sheet there is none (law 9),
        // and no button in the window answers Return by itself.
        e.preventDefault();
        if (menu || inSearch) return;
        if (w.ret) key("library", "return");
        return;
      }
      if (isText(t)) return;
      const del = e.key === "Backspace" || e.key === "Delete";
      // Delete never means "go back" in a window.
      if (del) e.preventDefault();
      if (w.body.sheet || menu) return;
      const sel = w.body.rows.filter((r) => r.selected);
      if (del && sel.length) {
        send({ do: "delete-selected" });
        return;
      }
      if (e.key.toLowerCase() === "c" && e.metaKey && !e.shiftKey && !e.altKey) {
        const copy = sel.find((r) => r.gestures.copy)?.gestures.copy;
        if (copy) {
          e.preventDefault();
          send(copy);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      un.then((f) => f());
      window.removeEventListener("keydown", onKey);
    };
  });

  function openMenu(row: RowT, x: number, y: number) {
    menu = { items: row.menu, x, y };
  }
</script>

{#if w}
  {@const b = w.body}
  <div class="lib" data-state={w.id} data-faces={w.faces.join("|")}>
    <div class="behind" inert={!!b.sheet}>
      {#if b.cleanup}
        <Cleanup c={b.cleanup} {send} />
      {:else}
      <header class="toolbar">
        <h1>{b.title}</h1>
        {#if b.scope.length}
          <div class="seg scope" role="radiogroup" aria-label={b.title}>
            {#each b.scope as c}
              <button
                type="button"
                role="radio"
                class:on={c.selected}
                aria-checked={c.selected}
                disabled={!c.enabled}
                onclick={() => {
                  if (!c.selected) send(c.action);
                }}>{c.label}</button
              >
            {/each}
          </div>
        {/if}
        <label class="search" class:off={!b.search.enabled}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.2" /><path d="M10.2 10.2l3.3 3.3" /></svg>
          <!-- A typed field the Scene owns (lib/typed.ts): a refused edit is taken back out. -->
          <input
            type="text"
            role="searchbox"
            data-field="library-search"
            placeholder={b.search.placeholder}
            aria-label={b.search.placeholder}
            disabled={!b.search.enabled}
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            use:typed={{ field: "library-search", scene: b.search }}
          />
        </label>
      </header>
      {#if b.notice}<p class="notice">{b.notice}</p>{/if}
      {#if b.going_line}
        <!-- N74: auto-cleanup's warning, three days ahead — amber, Review… only, no Not now. -->
        <div class="cline going" data-line="going">
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" /></svg>
          <span class="text">{b.going_line.line}</span>
          {#each b.going_line.buttons as nb}<Btn b={nb} {send} />{/each}
        </div>
      {/if}
      {#if b.cleaned_line}
        <!-- N74: once a round of auto-cleanup went, what went and what it freed, with OK. -->
        <div class="cline" data-line="cleaned">
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" /></svg>
          <span class="text">{b.cleaned_line.line}</span>
          {#each b.cleaned_line.buttons as nb}<Btn b={nb} {send} />{/each}
        </div>
      {/if}
      {#if b.cleanup_line}
        <!-- N68: the videos worth deleting, in one quiet line; Review… opens them, Not now puts it away. -->
        <div class="cline" data-line="cleanup">
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" /></svg>
          <span class="text">{b.cleanup_line.line}</span>
          <Btn b={b.cleanup_line.review} {send} />
          <Btn b={b.cleanup_line.dismiss} {send} />
        </div>
      {/if}
      {#each notes as n}<p class="notes">{n}</p>{/each}
      <div class="list">
        {#if b.empty}
          <!-- The empty face: the mushroom, dimmed, over the Scene's line. -->
          <div class="empty">
            <svg class="mush" viewBox="18 20 84 90" aria-hidden="true">
              <path class="stem" d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" />
              <path class="cap" d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" />
              <circle cx="50" cy="44" r="6" /><circle cx="72" cy="48" r="3.6" /><circle cx="33" cy="58" r="3.4" /><circle cx="64" cy="58" r="2.6" /><circle cx="88" cy="58" r="2.2" />
            </svg>
            <p>{b.empty}</p>
          </div>
        {:else}
          <!-- One grouped card of rows; a recovered take pinned on top gives every row its dot's room. -->
          <ul class="rows card" class:pins={b.rows.some((r) => r.pinned)} role="listbox" aria-multiselectable="true" aria-label={b.title}>
            {#each b.rows as row (row.key)}
              <Row {row} {send} {activates} menu={openMenu} {away} />
            {/each}
          </ul>
        {/if}
      </div>
      {#if b.footer}<footer class="foot">{b.footer}</footer>{/if}
      {/if}
    </div>
    {#if b.sheet}<Sheet sheet={b.sheet} {send} />{/if}
    {#if menu}<Menu items={menu.items} x={menu.x} y={menu.y} {send} close={() => (menu = null)} />{/if}
  </div>
{/if}

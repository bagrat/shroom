<script lang="ts">
  // The Settings window (N36) renders the slice it's handed — final copy included. It
  // composes no sentences and decides nothing (ARCHITECTURE §3.5). The band at its top is
  // the Scene's tabs; the pane is General, Space or Members, each in cards of rows; a sheet
  // (Disconnect, Leave, member-remove-confirm, Report a problem) sits over them.
  import { onMount } from "svelte";
  import { act, key, onScene, pasteSecret, sceneGet, type Action } from "./lib/ipc";
  import Sheet from "./library/Sheet.svelte";
  import General from "./settings/General.svelte";
  import Space from "./settings/Space.svelte";
  import ReportSheet from "./settings/ReportSheet.svelte";
  import Members from "./members/Members.svelte";
  import { isSettings, type SettingsWindow } from "./settings/types";

  let w = $state.raw<SettingsWindow | null>(null);
  // Whether this window may take the keyboard, as a value of its own: read straight off
  // `w`, it would change with every Scene, and every field that takes the keyboard when it
  // appears would take it again each time.
  let activates = $derived(!!w?.activates);

  const send = (a: Action) => act("settings", a);
  const take = (s: unknown) => {
    if (isSettings(s)) w = s;
  };

  // A text field owns its Return (law 8); checkboxes and radios don't.
  const typing = (t: EventTarget | null) =>
    t instanceof HTMLElement && !!t.closest('input:not([type="checkbox"]):not([type="radio"]), textarea, select');

  /** The paste field a paste anywhere in the window lands in: the token field when it shows
   * (the repair mood, Add a token here), else the invite (any Mac, N29) — no door shows for
   * it (B159). */
  function pasteTarget(): "token" | "invite" | null {
    if (!w || w.body.sheet || w.body.pane.kind !== "space") return null;
    if (w.body.pane.plan.nest.kind === "token") return "token";
    return "invite";
  }

  onMount(() => {
    sceneGet<SettingsWindow>("settings").then(take);
    const un = onScene<SettingsWindow>(take, "settings");
    const onKey = (e: KeyboardEvent) => {
      if (e.isComposing || !w) return;
      if (e.key === "Enter" && !typing(e.target)) {
        // No button answers Return by itself; the Scene's ret does (none under a sheet — law 9).
        e.preventDefault();
        if (w.ret) key("settings", "return");
      }
      if (e.key === "Escape") {
        e.preventDefault();
        if (w.esc) key("settings", "esc");
      }
    };
    const onPaste = (e: ClipboardEvent) => {
      if (typing(e.target)) return;
      const f = pasteTarget();
      if (!f) return;
      e.preventDefault();
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (text.trim()) pasteSecret(f, text);
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("paste", onPaste);
    return () => {
      un.then((f) => f());
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("paste", onPaste);
    };
  });
</script>

{#if w}
  {@const b = w.body}
  <div class="set" data-state={w.id} data-faces={w.faces.join("|")}>
    <div class="behind" inert={!!b.sheet}>
      <!-- The tabs: one segmented control, centred in the band at the window's top. -->
      <div class="tabbar">
        <div class="seg tabs" role="tablist">
          {#each b.tabs as t}
            <button
              type="button"
              role="tab"
              class="tab"
              class:on={t.selected}
              aria-selected={t.selected}
              disabled={!t.enabled}
              onclick={() => {
                if (!t.selected) send(t.action);
              }}>{t.label}</button
            >
          {/each}
        </div>
      </div>
      <main class="pane {b.pane.kind}">
        {#if b.pane.kind === "general"}
          <General g={b.pane} {send} />
        {:else if b.pane.kind === "space"}
          <Space sp={b.pane} {send} {activates} />
        {:else}
          <Members m={b.pane} {send} activates={activates && !b.sheet} />
        {/if}
      </main>
    </div>
    {#if b.sheet && b.report}<ReportSheet sheet={b.sheet} form={b.report} {send} />{:else if b.sheet}<Sheet sheet={b.sheet} {send} />{/if}
  </div>
{/if}

<script lang="ts">
  // The onboarding window renders the slice it's handed — the welcome, then the plan; or,
  // for an invite, its own faces (Region 6) — final copy included. It composes no sentences and decides nothing (ARCHITECTURE §3.5).
  // Under the brand mark (the one thing here that isn't the Scene's), drawn as the canvas
  // draws it (DEVIATIONS #45).
  import { onMount } from "svelte";
  import { act, key, onScene, pasteSecret, sceneGet, type Action } from "./lib/ipc";
  import Btn from "./onboarding/Btn.svelte";
  import LoginBox from "./onboarding/LoginBox.svelte";
  import Plan from "./onboarding/Plan.svelte";
  import Runs from "./onboarding/Runs.svelte";
  import Wordmark from "./onboarding/Wordmark.svelte";
  import Invite from "./join/Invite.svelte";
  import Sheet from "./library/Sheet.svelte";
  import { isOnboarding, type OnboardingWindow } from "./onboarding/types";

  let w = $state.raw<OnboardingWindow | null>(null);
  // Whether this window may take the keyboard, as a value of its own: read straight off
  // `w`, it would change with every Scene, and every field that takes the keyboard when it
  // appears would take it again each time.
  let activates = $derived(!!w?.activates);

  const send = (a: Action) => act("onboarding", a);
  const take = (s: unknown) => {
    if (isOnboarding(s)) w = s;
  };

  /** The paste field showing now, if any — a paste anywhere in the window lands there. The
   * welcome shows none and takes a pasted link all the same (B159). */
  function pasteTarget(): "token" | "invite" | null {
    if (!w) return null;
    if (w.body.kind === "welcome") return w.body.sheet ? null : "invite";
    if (w.body.kind === "invite") return w.body.door ? "invite" : null;
    if (w.body.nest.kind === "join-refused") return w.body.nest.door ? "invite" : null;
    return w.body.nest.kind === "token" ? "token" : null;
  }

  // A text field owns its Return (law 8); checkboxes and radios don't.
  const typing = (t: EventTarget | null) =>
    t instanceof HTMLElement && !!t.closest('input:not([type="checkbox"]):not([type="radio"]), textarea, select');

  onMount(() => {
    sceneGet<OnboardingWindow>("onboarding").then(take);
    const un = onScene<OnboardingWindow>(take, "onboarding");
    const onKey = (e: KeyboardEvent) => {
      if (e.isComposing) return;
      if (e.key === "Enter" && !typing(e.target)) {
        e.preventDefault();
        key("onboarding", "return");
      }
      if (e.key === "Escape") {
        e.preventDefault();
        key("onboarding", "esc");
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
  <main class="ob ob-{w.body.kind}" data-state={w.id}>
    <Wordmark />
    {#if w.body.kind === "welcome"}
      {@const b = w.body}
      <section class="welcome">
        <div class="said">{#each b.paragraphs as para}<p><Runs runs={para.runs} /></p>{/each}</div>
        <LoginBox t={b.login} {send} />
        <!-- The foot: Uninstall shroom… quiet at the left (B160), the buttons at the right,
             in the Scene's order (law 8: the welcome's filled action leads). -->
        <div class="foot" inert={!!b.sheet}>
          <Btn b={b.uninstall} {send} />
          <div class="acts">{#each b.buttons as btn}<Btn b={btn} {send} />{/each}</div>
        </div>
      </section>
      {#if b.sheet}<Sheet sheet={b.sheet} {send} />{/if}
    {:else if w.body.kind === "invite"}
      <Invite b={w.body} {send} {activates} />
    {:else}
      <Plan plan={w.body} {send} {activates} />
    {/if}
  </main>
{/if}

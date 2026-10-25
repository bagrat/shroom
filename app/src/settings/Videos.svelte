<script lang="ts">
  // Settings · Videos (N78; General's group Your videos until then, N68): one card — the
  // space's play-count switch and this Mac's suggestions switch, each with its line under
  // the label, then auto-cleanup's pop-up (N75), live on the Mac that set up the space.
  import type { Action } from "../lib/ipc";
  import type { VideosPane } from "./types";

  let { v, send }: { v: VideosPane; send: (a: Action) => void } = $props();
</script>

<section class="videos">
  <div class="card">
    {#each [{ t: v.count, hint: v.count_hint, note: v.count_note }, { t: v.suggest, hint: v.suggest_hint, note: null }] as s}
      <label class="srow tall" class:off={!s.t.enabled}>
        <span class="grow"
          ><span class="lbl2">{s.t.label}</span><span class="hint">{s.hint}</span>{#if s.note}<span class="hint note" role="status">{s.note}</span>{/if}</span
        >
        <input
          type="checkbox"
          class="switch"
          checked={s.t.on}
          disabled={!s.t.enabled}
          onclick={(e) => {
            e.preventDefault();
            send(s.t.action);
          }}
        />
      </label>
    {/each}
    {#if v.autoclean}
      {@const a = v.autoclean}
      {@const live = a.choices.some((c) => c.enabled)}
      <!-- N75: auto-cleanup — a pop-up on the Mac that set up the space, read-only elsewhere. -->
      <label class="srow tall" class:off={!live}>
        <span class="grow"
          ><span class="lbl2">{a.label}</span><span class="hint">{a.hint}</span>{#if a.note}<span class="hint note" role="status">{a.note}</span>{/if}</span
        >
        <select
          class="pop"
          disabled={!live}
          onchange={(e) => {
            const el = e.currentTarget as HTMLSelectElement;
            const c = a.choices[el.selectedIndex];
            // The pop-up shows what the space has until the core says otherwise (a question
            // may come first, and Cancel leaves it as it was).
            el.selectedIndex = a.choices.findIndex((x) => x.selected);
            if (c && !c.selected) send(c.action);
          }}
        >
          {#each a.choices as c}<option selected={c.selected}>{c.label}</option>{/each}
        </select>
      </label>
    {/if}
  </div>
</section>

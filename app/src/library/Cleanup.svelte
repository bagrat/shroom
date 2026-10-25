<script lang="ts">
  // Suggested to clean up (N68), in place of the Library's list: ‹ Library, the title and
  // the line on what deleting them all frees and saves, then one card of suggestions — the
  // poster, the title, when it was made · its size · why — each with Open page, Keep and
  // Delete (red text, law 9). Under them, the foot and Delete all…; a one-tap delete's
  // Undo line floats at the window's foot until the core sends it. Every word is the
  // Scene's; nothing here decides.
  import type { Action } from "../lib/ipc";
  import Btn from "../onboarding/Btn.svelte";
  import type { CleanupBody } from "./types";

  let { c, send }: { c: CleanupBody; send: (a: Action) => void } = $props();

  const poster = (p: string | null) => (p ? `shroom-poster://localhost/${p.split("/").map(encodeURIComponent).join("/")}` : null);
  let failed = $state<Record<string, boolean>>({});
</script>

<div class="cview" data-view="cleanup">
  <header class="chead">
    <Btn b={c.back} {send} />
    <h1>{c.title}</h1>
    <!-- With videos going, the line on the suggestions sits under their own heading (N74). -->
    {#if c.intro && !c.going.length}<p class="intro">{c.intro}</p>{/if}
  </header>
  <div class="list">
    {#if c.empty}
      <div class="empty"><p>{c.empty}</p></div>
    {:else}
      <!-- N74: what auto-cleanup will delete leads, a heading per date, each with Open page and Pin. -->
      {#each c.going as g (g.heading)}
        <div class="gohead">
          <h2>{g.heading}</h2>
          {#if g.line}<p>{g.line}</p>{/if}
        </div>
        <ul class="rows card" aria-label={g.heading}>
          {#each g.rows as row (row.key)}
            {@const src = poster(row.poster)}
            <li class="lrow" data-key={row.key}>
              <span class="lead" aria-hidden="true"></span>
              <span class="thumb" aria-hidden="true"
                >{#if src && !failed[row.key]}<img {src} alt="" draggable="false" onerror={() => (failed = { ...failed, [row.key]: true })} />{:else}<svg viewBox="0 0 16 16"
                    ><path d="M5 3.5v9l7.5-4.5z" /></svg
                  >{/if}</span
              >
              <div class="main">
                <div class="line1"><span class="title">{row.title ?? ""}</span></div>
                <div class="line2"><span class="meta">{row.meta}</span>{#if row.going}<span class="state amber">{row.going}</span>{/if}</div>
              </div>
              <div class="tail"><div class="always">{#each row.buttons as b}<Btn {b} {send} inlineNote={false} />{/each}</div></div>
            </li>
          {/each}
        </ul>
      {/each}
      {#if c.suggest_heading}<div class="gohead"><h2>{c.suggest_heading}</h2>{#if c.intro}<p>{c.intro}</p>{/if}</div>{/if}
      {#if c.rows.length}
      <ul class="rows card" aria-label={c.title}>
        {#each c.rows as row (row.key)}
          {@const src = poster(row.poster)}
          <li class="lrow" data-key={row.key}>
            <span class="lead" aria-hidden="true"></span>
            <span class="thumb" aria-hidden="true"
              >{#if src && !failed[row.key]}<img {src} alt="" draggable="false" onerror={() => (failed = { ...failed, [row.key]: true })} />{:else}<svg viewBox="0 0 16 16"
                  ><path d="M5 3.5v9l7.5-4.5z" /></svg
                >{/if}</span
            >
            <div class="main">
              <div class="line1"><span class="title">{row.title ?? ""}</span></div>
              <div class="line2"><span class="meta">{row.meta}</span></div>
            </div>
            <div class="tail"><div class="always">{#each row.buttons as b}<Btn {b} {send} inlineNote={false} />{/each}</div></div>
          </li>
        {/each}
      </ul>
      {/if}
    {/if}
  </div>
  {#if c.foot || c.delete_all}
    <footer class="cfoot">
      {#if c.foot}<p>{c.foot}</p>{/if}
      {#if c.delete_all}<div class="tail"><Btn b={c.delete_all} {send} inlineNote={false} /></div>{/if}
    </footer>
  {/if}
  {#if c.undo}
    <div class="undo" role="status">
      <span>{c.undo.line}</span>
      <Btn b={c.undo.undo} {send} />
    </div>
  {/if}
</div>

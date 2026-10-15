<script lang="ts">
  // “Start shroom when you log in” (U4): the welcome's box, and member-welcome's — drawn as
  // the canvas draws it, a switch row in a card. A click sends its action and the switch
  // keeps the Scene's state until the Scene changes it.
  import type { Action } from "../lib/ipc";
  import type { Toggle } from "./types";

  let { t, send }: { t: Toggle; send: (a: Action) => void } = $props();
</script>

<div class="card login">
  <label class="check" class:off={!t.enabled}>
    <span>{t.label}</span>
    <input
      class="switch"
      type="checkbox"
      checked={t.on}
      disabled={!t.enabled}
      onclick={(e) => {
        e.preventDefault();
        send(t.action);
      }}
    />
  </label>
</div>

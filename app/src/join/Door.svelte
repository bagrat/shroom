<script lang="ts">
  // The invite paste door (N29): its line, then a paste-only field beside it. The line is
  // the field's label — “Have a new link? Paste it here.” points at it. A paste goes to the
  // runner and nowhere else (law 13); the Scene answers. Used by the invite window's
  // dead-invite faces (Settings · Space shows no door since B159). While a pasted link is
  // held, the Scene says so (`held`) and the field shows its dots.
  import PasteField from "../onboarding/PasteField.svelte";

  let { line, focus = false, held = false }: { line: string; focus?: boolean; held?: boolean } = $props();
</script>

<div class="paste-door" data-door="invite">
  <p class="quiet door-line">{line}</p>
  <PasteField field="invite" placeholder="" label={line} {focus} {held} />
</div>

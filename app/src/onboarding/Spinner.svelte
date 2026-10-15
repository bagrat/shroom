<script lang="ts">
  // The quiet spinner: the wizard's only progress surface — the canvas's ring with its
  // Frost arc. `rest` stops it (wizard-offline: the spinner rests).
  let { rest = false }: { rest?: boolean } = $props();
</script>

<svg class="spin" class:rest viewBox="0 0 16 16" aria-hidden="true"><circle class="track" cx="8" cy="8" r="6" /><path class="arc" d="M8 2a6 6 0 0 1 6 6" /></svg>

<script lang="ts">
  import type { PanelLook } from "./panel";

  const { dialog, children }: PanelLook = $props();
</script>

<dialog
  {...dialog}
  class="settings-panel fixed inset-y-0 left-0 m-0 h-dvh max-h-none w-full max-w-[calc(var(--silver-side)*(1+var(--silver))-var(--spacing-gutter)/2)] bg-page narrow:max-w-none p-0 text-body text-text shadow-sheet backdrop:bg-transparent backdrop:backdrop-brightness-50"
>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render children()}
</dialog>

<style>
  .settings-panel {
    transition: translate var(--transition-duration-panel) var(--ease-arrive);
  }
  @starting-style {
    .settings-panel[open] {
      translate: -100% 0;
    }
  }
</style>

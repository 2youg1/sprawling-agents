<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The settings panel, the shipped look: the `<dialog>` frame and its
  // arrival. Every attribute and handler the element needs arrives in
  // `dialog` from the seat (`panel.svelte`).
  //
  // One width: from the window's left edge to the shell's right silver
  // line (client D24), so its tree stands where the sessions pane stands
  // and its group where the conversation stands; the whole window on one
  // column.

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
  /* The panel arrives from the left edge it belongs to, as an arrival
   * rather than a cut (client/Spec.lean §7L), so it only ever
   * decelerates. It never animates out: the shell takes it away when the
   * address bar stops naming it, and a panel that lingered would answer
   * clicks for a page already gone. Motion off zeroes the duration token
   * (`theme/motion-state.css`), so the panel then simply appears. */
  .settings-panel {
    transition: translate var(--transition-duration-panel) var(--ease-arrive);
  }
  @starting-style {
    .settings-panel[open] {
      translate: -100% 0;
    }
  }
</style>

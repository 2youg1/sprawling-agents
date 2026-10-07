<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the segmented control is drawn, and nothing else (client D95):
  // a track per band, a slider under the chosen cell, a rule between
  // bands. Every word arrives translated and every role, key handler,
  // tab stop and `aria-*` value arrives inside a wire bag, which is
  // spread unchanged on the element it is for, so another look - one
  // built from a component library - draws the same control by taking
  // the same `SegmentedLook`.
  //
  // The slider travels on `transform` over cells of one width, so the
  // move is a compositor job and no script measures anything. Its
  // duration is a theme token, which `data-motion="off"` and the
  // machine's reduced motion both set to zero, so it stands still
  // without a rule of its own here.
  //
  // The track never grows past the box it stands in: where the cells'
  // words do not fit on one line at one width each, they break inside
  // their cells instead of running past the box's edge, so a narrow
  // column gets a taller track rather than a clipped choice.
  import Tip from "./tip.svelte";

  import type { CellLook, SegmentedLook, Tone } from "./segmented";

  const look: SegmentedLook = $props();

  // The fill each tone paints the chosen cell with, spelled for
  // Tailwind to read out of this file as text.
  const FILL: Record<Tone, string> = {
    plain: "bg-raised-hover",
    alert: "bg-alert",
    accent: "bg-accent",
  };

  // The ink a cell takes. The chosen cell on an `alert` or `accent`
  // fill is read against a coloured solid and takes the page's own
  // rung; on the plain fill it is read against a surface one step up,
  // and takes the full text token the rest of the page is set in.
  const ink = (cell: CellLook, tone: Tone): string => {
    switch (cell.state) {
      case "refused":
        return "text-text-disabled";
      case "free":
        return "text-text-quiet hover:text-text";
      case "held":
        switch (tone) {
          case "alert":
          case "accent":
            return "text-on-accent";
          case "plain":
            return "text-text";
        }
    }
  };
</script>

<div class="inline-flex max-w-full items-end gap-snug" {...look.group}>
  {#each look.bands as band, at (band.key)}
    {#if at > 0}
      <span aria-hidden="true" class="w-hair self-stretch bg-edge-panel"></span>
    {/if}
    <div class="flex min-w-0 flex-col gap-tight">
      {#if band.heading !== undefined}
        <span class="px-base text-note text-text-quiet">{band.heading}</span>
      {/if}
      <!-- The track states how many cells it holds and which one is
          chosen; the slider inherits both, so its width is one cell
          and its travel is that width times the chosen position, and a
          change of choice moves one value. -->
      <div
        class="relative grid auto-cols-fr grid-flow-col rounded-pill bg-track"
        style:--cells={String(band.cells.length)}
        style:--held={String(Math.max(band.held, 0))}
      >
        {#if band.held >= 0}
          <span aria-hidden="true" class={["slider pointer-events-none absolute inset-y-0 left-0 rounded-pill", FILL[band.tone]]}
          ></span>
        {/if}
        {#each band.cells as cell (cell.key)}
          {#snippet drawn()}
            <button
              type="button"
              class={["cell relative h-full w-full rounded-pill px-base py-tight text-label text-balance", ink(cell, band.tone)]}
              {...cell.wire}
            >
              {cell.label}
            </button>
          {/snippet}
          <!-- One definition of the cell, drawn bare or inside its reason;
              the reason's id is already in the cell's wire bag. -->
          {#if cell.why !== undefined}
            <Tip id={cell.why.id} text={cell.why.text}>
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
              {@render drawn()}
            </Tip>
          {:else}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
            {@render drawn()}
          {/if}
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  /* The slider only ever arrives: whichever way it travels, it comes to
   * rest under the cell just chosen, so it decelerates both ways. */
  .slider {
    width: calc(100% / var(--cells));
    transform: translateX(calc(var(--held) * 100%));
    transition: transform var(--transition-duration-short) var(--ease-arrive);
  }

  /* A free cell's ink brightens under the pointer and fades back when
   * it leaves: the resting state leaves, the hovered state arrives
   * (docs/frontend-method.md §4-43). */
  .cell {
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .cell:hover {
    transition-timing-function: var(--ease-arrive);
  }

  /* A forced-colour mode replaces every fill, so the slider would
   * vanish and every cell, each edged in the system ink like any pill,
   * would look chosen alike. The slider keeps the system's own
   * selection colour and the chosen cell its ink, which the mode
   * guarantees stand apart from everything else it paints. */
  @media (forced-colors: active) {
    .slider {
      forced-color-adjust: none;
      background-color: Highlight;
    }
    .cell[aria-checked="true"] {
      forced-color-adjust: none;
      color: HighlightText;
    }
  }
</style>

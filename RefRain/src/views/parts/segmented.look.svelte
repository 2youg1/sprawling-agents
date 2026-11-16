<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Tip from "./tip.svelte";

  import type { CellLook, SegmentedLook, Tone } from "./segmented";

  const look: SegmentedLook = $props();

  const FILL: Record<Tone, string> = {
    plain: "bg-raised-hover",
    alert: "bg-alert",
    accent: "bg-accent",
  };

  const ink = (cell: CellLook, tone: Tone): string => {
    switch (cell.state) {
      case "refused":
        return "aria-disabled:text-text-disabled";
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
      <div
        class="relative grid auto-cols-fr grid-flow-col rounded-pill bg-track"
        style:--cells={String(band.cells.length)}
        style:--held={String(Math.max(band.held, 0))}
      >
        {#if band.held >= 0}
          <span
            aria-hidden="true"
            class={[
              "slider pointer-events-none absolute inset-y-0 left-0 rounded-pill",
              FILL[band.tone],
            ]}
          ></span>
        {/if}
        {#each band.cells as cell (cell.key)}
          {#snippet drawn()}
            <button
              type="button"
              class={[
                "cell relative h-full w-full rounded-pill px-base py-tight text-label text-balance",
                ink(cell, band.tone),
              ]}
              {...cell.wire}
            >
              {cell.label}
            </button>
          {/snippet}

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
  .slider {
    width: calc(100% / var(--cells));
    transform: translateX(calc(var(--held) * 100%));
    transition: transform var(--transition-duration-short) var(--ease-arrive);
  }

  .cell {
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .cell:hover {
    transition-timing-function: var(--ease-arrive);
  }

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

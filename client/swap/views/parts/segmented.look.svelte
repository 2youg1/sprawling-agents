<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A second look for the segmented control, written the way a
  // component library writes one: no utility classes, every style in its
  // own block and read from the theme's tokens, and a different drawing
  // - a vertical list of options, a check before the chosen one, an
  // accent edge around it, and a fade where the shipped look slides. It
  // takes the same `SegmentedLook` as the shipped look and nothing else,
  // and spreads every wire bag where the shipped look does, so drawing
  // the control with it proves the key table, the tab stop and the
  // `aria-*` values live in `segmented.ts` and the seat, not in the
  // shipped markup (client D95).
  import Glyph from "../../../src/views/parts/glyph.svelte";
  import Tip from "../../../src/views/parts/tip.svelte";
  import type { CellLook, SegmentedLook } from "../../../src/views/parts/segmented";

  const look: SegmentedLook = $props();
</script>

{#snippet option(cell: CellLook)}
  <button type="button" class="option" {...cell.wire}>
    <span class="mark"><Glyph name="check" size="sm" /></span>
    <span class="label">{cell.label}</span>
  </button>
{/snippet}

<div class="choices" {...look.group}>
  {#each look.bands as band (band.key)}
    <div class="band">
      {#if band.heading !== undefined}
        <span class="heading">{band.heading}</span>
      {/if}
      {#each band.cells as cell (cell.key)}
        {#if cell.why !== undefined}
          <Tip id={cell.why.id} text={cell.why.text}>
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
            {@render option(cell)}
          </Tip>
        {:else}
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
          {@render option(cell)}
        {/if}
      {/each}
    </div>
  {/each}
</div>

<style>
  .choices {
    display: inline-flex;
    flex-direction: column;
    gap: var(--spacing-snug);
    max-width: 100%;
  }
  .band {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-tight);
    min-width: 0;
  }
  .heading {
    color: var(--color-text-faint);
    font-size: var(--text-note);
  }
  .option {
    display: flex;
    align-items: center;
    gap: var(--spacing-snug);
    width: 100%;
    min-height: var(--spacing-control-sm);
    padding: 0 var(--spacing-snug);
    border: 1px solid var(--color-edge-panel);
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--color-text-quiet);
    font-size: var(--text-label);
    text-align: start;
    transition: border-color var(--transition-duration-panel) var(--ease-leave);
  }
  .option[aria-checked="true"] {
    border-color: var(--color-accent);
    color: var(--color-text);
    transition-timing-function: var(--ease-arrive);
  }
  .option[aria-disabled="true"] {
    color: var(--color-text-disabled);
  }
  .mark {
    display: inline-flex;
    opacity: 0;
    transition: opacity var(--transition-duration-panel) var(--ease-leave);
  }
  .option[aria-checked="true"] .mark {
    opacity: 1;
    transition-timing-function: var(--ease-arrive);
  }
  .label {
    min-width: 0;
  }
</style>

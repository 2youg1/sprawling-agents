<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { PHASE_FILL } from "../runs/phase";
  import type { WearLook } from "./wear";

  const { hint, track, read, thumb, marks }: WearLook = $props();

  const pct = (share: number): string => `${String(share * 100)}%`;
  const TICK = 0.012;
</script>

<div
  class="track relative h-full cursor-pointer touch-none overflow-hidden"
  title={hint}
  {...track}
>
  <div class="line bg-edge"></div>
  {#each read as [top, height], at (at)}
    <div class="worn bg-edge-input" style:top={pct(top)} style:height={pct(height)}></div>
  {/each}
  {#each marks as mark, at (at)}
    <div
      class={["tick", PHASE_FILL[mark.phase]]}
      style:top={pct(Math.max(0, mark.top + mark.height / 2 - TICK / 2))}
      style:height={pct(TICK)}
    ></div>
  {/each}
  <div class="thumb bg-text-faint" style:top={pct(thumb[0])} style:height={pct(thumb[1])}></div>
</div>

<style>
  .track {
    opacity: 0.6;
    transition:
      opacity var(--transition-duration-short) var(--ease-leave),
      padding-inline var(--transition-duration-short) var(--ease-leave);
  }
  .track:hover {
    opacity: 0.85;
    padding-inline: 1px;
    transition-timing-function: var(--ease-arrive);
  }
  .line,
  .worn {
    position: absolute;
    right: var(--spacing-tight);
    width: 1px;
  }
  .track:hover .line,
  .track:hover .worn {
    right: calc(var(--spacing-tight) - 1px);
    width: var(--spacing-wear);
  }
  .line {
    inset-block: 0;
  }
  .tick {
    position: absolute;
    right: calc(var(--spacing-tight) - 1px);
    width: var(--spacing-wear);
    min-height: var(--spacing-wear);
    border-radius: var(--radius-pill);
    z-index: 1;
  }
  .track:hover .tick {
    right: calc(var(--spacing-tight) - var(--spacing-hair));
    width: calc(var(--spacing-wear) + var(--spacing-hair));
  }
  .thumb {
    position: absolute;
    right: calc(var(--spacing-tight) - 1px);
    width: var(--spacing-wear);
    min-height: var(--spacing-wear);
    border-radius: var(--radius-pill);
    z-index: 2;
    transition:
      background-color var(--transition-duration-short) var(--ease-leave),
      width var(--transition-duration-short) var(--ease-leave),
      right var(--transition-duration-short) var(--ease-leave);
  }
  .track:hover .thumb {
    right: calc(var(--spacing-tight) - var(--spacing-hair));
    width: calc(var(--spacing-wear) + var(--spacing-hair));
    background-color: var(--color-text-quiet);
    transition-timing-function: var(--ease-arrive);
  }
</style>

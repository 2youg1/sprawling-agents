<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the read-wear bar is drawn, and nothing else (refrain S7.3): a
  // hairline track, the stretches already read worn a shade darker over
  // it, each round's tick as a short centred hairline in its phase's
  // colour from `runs/phase.ts`, and the thumb where the view is — drawn
  // last, so it stays on top and in a colour of its own. Every layer
  // arrives in shares of the whole content (`WearLook`, `./wear`), so
  // the bar needs no height of its own.
  //
  // The whole bar sits at 60 % so it stays a map rather than a chrome
  // strip; under the pointer it widens by one pixel on each side and the
  // thumb darkens, which is what says the track can be pressed.
  import { PHASE_FILL } from "../runs/phase";
  import type { WearLook } from "./wear";

  const { hint, track, read, thumb, marks }: WearLook = $props();

  const pct = (share: number): string => `${String(share * 100)}%`;
  // A round's tick is a fixed short hairline at its midpoint, so a short
  // round and a long one read as the same kind of mark rather than a
  // scatter of dots of random size.
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
    /* One pixel wider on each side under the pointer. */
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
    /* Above every tick: the view's place must stay readable. */
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

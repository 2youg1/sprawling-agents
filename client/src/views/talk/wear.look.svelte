<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the read-wear bar is drawn, and nothing else (refrain S7.3): a
  // hairline track, the stretches already read worn a shade darker over
  // it, the thumb where the view is, and each round's tick in its
  // phase's colour from `runs/phase.ts`. Every layer arrives in shares
  // of the whole content (`WearLook`, `./wear`), so the bar needs no
  // height of its own.
  //
  // The thumb and the ticks are one width, centred on the track's line,
  // and a tick has the same floor, so a round at the very foot stays a
  // dot rather than vanishing; the track clips what it draws, so that
  // dot never reaches past the column. The thumb darkens while a pointer
  // rests on the track, which is what says the track can be pressed.
  import { PHASE_FILL } from "../runs/phase";
  import type { WearLook } from "./wear";

  const { hint, track, read, thumb, marks }: WearLook = $props();

  const pct = (share: number): string => `${String(share * 100)}%`;
</script>

<div class="track relative h-full cursor-pointer touch-none overflow-hidden" title={hint} {...track}>
  <div class="line bg-edge"></div>
  {#each read as [top, height], at (at)}
    <div class="worn bg-edge-input" style:top={pct(top)} style:height={pct(height)}></div>
  {/each}
  <div class="tick thumb bg-raised-hover" style:top={pct(thumb[0])} style:height={pct(thumb[1])}></div>
  {#each marks as mark, at (at)}
    <div class={["tick", PHASE_FILL[mark.phase]]} style:top={pct(mark.top)} style:height={pct(Math.min(mark.height, 0.01))}></div>
  {/each}
</div>

<style>
  .line,
  .worn {
    position: absolute;
    right: var(--spacing-tight);
    width: 1px;
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
  }
  .thumb {
    transition: background-color var(--transition-duration-short) var(--ease-leave);
  }
  .track:hover .thumb {
    background-color: var(--color-text-faint);
    transition-timing-function: var(--ease-arrive);
  }
</style>

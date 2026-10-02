<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The rhythm a reply arrived in, 40 px wide, at the end of its head
(refrain §3-3). One bar per slice of `rhythm.ts`, as tall as the share of
the busiest slice it carried, so a steady stream is a flat row and a
stall is a gap. Drawn for the eye only: it is an impression of a pace,
and a screen reader reading twenty heights would be reading noise. -->
<script lang="ts">
  import { SLICES } from "./rhythm";

  interface Props {
    readonly rhythm: readonly number[];
  }

  const { rhythm }: Props = $props();

  // Two pixels a slice, one of them a bar; the box is a line of note text
  // tall, and a slice that carried anything is at least one pixel.
  const HEIGHT = 12;
</script>

<svg
  class="inline-block h-[12px] w-[40px] shrink-0 self-center text-text-faint"
  viewBox="0 0 {SLICES * 2} {HEIGHT}"
  aria-hidden="true"
>
  {#each rhythm as share, slice (slice)}
    {#if share > 0}
      {@const tall = Math.max(1, Math.round(share * HEIGHT))}
      <rect x={slice * 2} y={HEIGHT - tall} width="1" height={tall} fill="currentColor" />
    {/if}
  {/each}
</svg>

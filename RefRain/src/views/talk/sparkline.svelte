<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { SLICES } from "./rhythm";
  interface Props {
    readonly rhythm: readonly number[];
  }

  const { rhythm }: Props = $props();

  const HEIGHT = 12;
</script>

<svg
  class="inline-block h-sparkline-tall w-sparkline shrink-0 self-center text-text-faint"
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

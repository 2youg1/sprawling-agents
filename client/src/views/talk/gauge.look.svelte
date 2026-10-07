<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the context ring is drawn round the coin key, and nothing else
  // (docs/frontend-method.md §7J): a 1 px ring in the key's square, the
  // used part eaten away clockwise from twelve o'clock, and the two
  // checkpoints on it. Every word and every attribute arrives in
  // `RingLook` (`./gauge`), so another look draws the same ring by
  // spreading the same meter wire onto its own focusable element.
  //
  // Only the remaining arc is stroked: an arc of the page colour laid
  // over a whole ring leaves a fringe where the two meet. The arc moves
  // rather than jumps, over the page duration, because it crosses the
  // ring the way the eye crosses a page.
  import type { Snippet } from "svelte";

  import Tip from "../parts/tip.svelte";
  import Checkpoint from "./checkpoint.look.svelte";
  import type { RingLook } from "./gauge";

  interface Props {
    // The ring, or null where the session's model stated no window and
    // the key stands alone.
    readonly ring: RingLook | null;
    readonly children: Snippet;
  }

  const { ring, children: key }: Props = $props();
</script>

{#if ring === null}
  <span class="grid size-key shrink-0 place-items-center">{@render key()}</span>
{:else}
  <Tip text={ring.reading}>
    <span class="relative grid size-key shrink-0 place-items-center">
      <span class="absolute inset-0 rounded-pill" {...ring.meter}></span>
      <svg class="pointer-events-none absolute inset-0 -rotate-90" viewBox="0 0 40 40" aria-hidden="true">
        <!-- r is 18 of the view box's 40 units, the share of the key's
        square `checkpoint.look.svelte` turns its dots out to. -->
        <circle
          class="left fill-none stroke-text"
          cx="20"
          cy="20"
          r="18"
          stroke-width="1"
          pathLength="100"
          stroke-dasharray={ring.arc.dasharray}
          stroke-dashoffset={ring.arc.dashoffset}
        />
      </svg>
      {#each ring.checkpoints as checkpoint (checkpoint.reminder)}
        <Checkpoint {...checkpoint} along="ring" />
      {/each}
      {@render key()}
    </span>
  </Tip>
{/if}

<style>
  .left {
    transition:
      stroke-dasharray var(--transition-duration-page) var(--ease-arrive),
      stroke-dashoffset var(--transition-duration-page) var(--ease-arrive);
  }
</style>

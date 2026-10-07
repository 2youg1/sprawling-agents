<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How one recent session is drawn: one control-tall row whose link
  // holds the room, how the session began and how long ago it was last
  // written, in three columns that line up from row to row; then the
  // fork key, always drawn rather than on hover, and the digit that
  // reaches the row. The fork key names itself in a hint, or says there
  // why it cannot fork.
  import Glyph from "../parts/glyph.svelte";
  import Tip from "../parts/tip.svelte";
  import type { RecentRowLook } from "./recent_row";

  const look: RecentRowLook = $props();
</script>

<li class="-mx-snug flex h-control items-center gap-snug rounded-card px-snug hover:wash">
  <a
    class="grid min-w-0 flex-1 grid-cols-[minmax(10ch,1fr)_minmax(0,auto)_var(--spacing-figure)] items-center gap-x-base rounded-control whitespace-nowrap focus-visible:wash"
    {...look.link}
  >
    <span class="min-w-0 truncate">{look.room}</span>
    <span class="truncate text-note text-text-faint">{look.began}</span>
    <span class="figure text-right text-note text-text-faint">{look.ago}</span>
  </a>
  <Tip text={look.fork.why ?? look.fork.name}>
    {#snippet children(hint: string)}
      <button
        class="fork grid size-control-sm shrink-0 place-items-center rounded-control text-text-quiet hover:bg-raised-hover hover:text-text aria-disabled:text-text-disabled"
        {...look.fork.wire}
        aria-describedby={hint}
      >
        <!-- The same mark the thread's branch action carries, so the
        two doors to one act look alike. -->
        <Glyph name="branch" size="sm" />
      </button>
    {/snippet}
  </Tip>
  <kbd class="entry-n" aria-hidden="true"></kbd>
</li>

<style>
  /* The key's ground comes in decelerating and goes accelerating
   * (docs/frontend-method.md §4-43). */
  .fork {
    transition:
      background-color var(--transition-duration-short) var(--ease-leave),
      color var(--transition-duration-short) var(--ease-leave);
  }
  .fork:hover {
    transition-timing-function: var(--ease-arrive);
  }
</style>

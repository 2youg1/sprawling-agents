<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How one working room is drawn: a link row with the phase's mark in
  // the tier the runs board paints it, the room, what the run is doing
  // and for how long, the digit that reaches the row, and the task on a
  // second line under the room.
  import Glyph from "../parts/glyph.svelte";
  import type { Weight } from "../parts/glyph";
  import type { WorkingRowLook } from "./working_row";

  const look: WorkingRowLook = $props();

  const INK: Readonly<Record<Weight, string>> = { quiet: "text-text-quiet", live: "text-accent", alert: "text-alert" };
</script>

<li>
  <a
    class="grid grid-cols-[var(--spacing-glyph-sm)_minmax(0,1fr)_auto_auto] items-center gap-x-snug rounded-card px-snug py-tight hover:wash focus-visible:wash"
    {...look.link}
  >
    <Glyph name={look.mark.glyph} size="sm" class={INK[look.mark.weight]} />
    <span class="min-w-0 truncate font-label">{look.room}</span>
    <span class="figure text-note text-text-faint">{look.doing}</span>
    <kbd class="entry-n" aria-hidden="true"></kbd>
    {#if look.task !== null}
      <span class="col-start-2 col-end-5 truncate text-note text-text-quiet">{look.task}</span>
    {/if}
  </a>
</li>

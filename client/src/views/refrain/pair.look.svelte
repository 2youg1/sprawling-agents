<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the two versions a comparison reads are chosen (client/Spec.lean
  // §7N): two native lists rather than rows of cells, because a version's
  // name, where it came from and when do not fit an equal cell
  // (docs/frontend-method.md). Each list's name stands in a column of its
  // own, so the two lists start on one line whatever the names' lengths.
  import type { PairLook } from "./pair";

  const look: PairLook = $props();
</script>

<div class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-base gap-y-snug border-b border-edge px-wide py-snug">
  {#each look.sides as side (side.key)}
    <span class="text-note text-text-faint">{side.label}</span>
    <select
      {...side.wire}
      value={side.held}
      class="h-control-sm w-full min-w-0 rounded-control border border-edge-input bg-raised px-snug text-note text-text transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive"
    >
      {#each side.options as option (option.value)}
        <option value={option.value} disabled={option.disabled}>{option.label}</option>
      {/each}
    </select>
  {/each}
  {#if look.more !== null}
    <p class="col-span-2 text-note text-text-faint">{look.more}</p>
  {/if}
</div>

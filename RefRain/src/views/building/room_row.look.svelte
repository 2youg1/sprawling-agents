<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- One room of the building, a row like the tree's beside it: the row's
wash under the pointer, and the live badge at its end while runs work
there. -->
<script lang="ts">
  import Badge from "../parts/badge.svelte";
  import type { RoomRowLook } from "./room_row";

  const look: RoomRowLook = $props();
</script>

<button
  {...look.wire}
  class="flex h-step w-full items-center gap-snug rounded-control px-snug text-left leading-none text-text-quiet hover:wash"
>
  <span class="min-w-0 flex-1 truncate">{look.name}</span>
  {#if look.live !== undefined}
    <Badge text={look.live} weight="live" dot />
  {/if}
</button>

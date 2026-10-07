<script lang="ts">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a progress bar is drawn, and nothing else (`./progress`,
// `ProgressLook`).
//
// While the end is unknown the empty track breathes with the theme's
// `pulse`, and nothing is filled: a part-filled bar would claim a
// fraction the bar does not have, and with motion off the track stands
// still and empty, which is what is known.

import type { ProgressLook } from "./progress";

const look: ProgressLook = $props();
</script>

<div class="flex w-full min-w-0 items-center gap-base">
  <div
    {...look.bar}
    class={["h-snug min-w-0 flex-1 overflow-hidden rounded-pill bg-raised", look.reached === undefined && "pulse"]}
  >
    {#if look.reached !== undefined}
      <div class="h-full rounded-pill bg-progress-done" style:width="{Math.round(look.reached.share * 100)}%"></div>
    {/if}
  </div>
  {#if look.reached !== undefined}
    <span class="shrink-0 font-mono text-note text-text-quiet">{look.reached.done} / {look.reached.total}</span>
  {/if}
</div>

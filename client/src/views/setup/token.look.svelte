<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import type { TokenLook } from "./token";

  const { name, well, reset }: TokenLook = $props();
</script>

<li class="flex items-center gap-tight">
  <input {...well} class="h-control w-control shrink-0 rounded-control border border-edge-input bg-raised" />
  <span class="min-w-0 flex-1 truncate font-mono text-note text-text-quiet">{name}</span>
  {#if reset !== undefined}
    <!-- A control's hover lifts it from the card it stands on, the one
    hover controls take across the page. -->
    <button
      {...reset.wire}
      class="h-control-sm shrink-0 rounded-control px-snug text-note text-text-quiet transition-colors duration-short ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive"
    >
      {reset.word}
    </button>
  {/if}
</li>

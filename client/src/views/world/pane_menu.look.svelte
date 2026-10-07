<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How a workbench pane's label is drawn: words in the blend tier, and
in the panorama a quiet button that opens a small menu under it. The
button reaches a step past the column's edge so its words stand on the
same line as the pane's text below. Every role, key and `aria-*` value
comes in the bags. -->
<script lang="ts">
  import type { PaneMenuLook } from "./pane_menu";

  const look: PaneMenuLook = $props();
</script>

{#if look.arranged === "words"}
  <h2 class="flex h-control shrink-0 items-center text-note text-text-faint">{look.label}</h2>
{:else}
  <div class="relative flex h-control shrink-0 items-center">
    <h2 class="contents">
      <button
        {...look.trigger}
        class="-mx-snug flex h-control-sm items-center rounded-control px-snug text-note text-text-faint hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text"
      >
        {look.label}
      </button>
    </h2>
    {#if look.open}
      <ul {...look.popup} {...look.list} class="absolute top-full left-0 flex min-w-[16ch] flex-col rounded-card bg-raised p-tight shadow-float">
        {#each look.items as item (item.key)}
          <li {...item.holder}>
            <button
              {...item.wire}
              class="flex h-control-sm w-full items-center rounded-control px-snug text-left text-note text-text hover:wash focus-visible:wash disabled:text-text-disabled"
            >
              {item.word}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}


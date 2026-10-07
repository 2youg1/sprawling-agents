<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the skyline's legend is drawn, and nothing else: a disclosure
  // whose summary names it, and under it each mark the drawing carries,
  // drawn small in the drawing's own inks, beside its word
  // (`./legend.ts`, `LegendLook`).
  import type { LegendLook } from "./legend";

  const look: LegendLook = $props();
</script>

<details class="text-note text-text-faint">
  <summary class="cursor-pointer text-text-quiet transition-colors ease-leave hover:text-text hover:ease-arrive">{look.summary}</summary>
  <ul class="mt-snug flex flex-wrap items-center gap-wide">
    {#each look.marks as each (each.mark)}
      <li class="flex items-center gap-tight">
        <svg viewBox="0 0 16 16" class="size-glyph shrink-0" aria-hidden="true">
          {#if each.mark === "window"}
            <rect x="5" y="3" width="6" height="8" rx="1" class="fill-accent-solid" />
          {:else if each.mark === "figure"}
            <circle cx="8" cy="5" r="2.4" class="fill-drawn-figure" />
            <path d="M4.6 13 q3.4 -6 6.8 0 z" class="fill-drawn-figure" />
          {:else if each.mark === "flag"}
            <line x1="5" y1="2" x2="5" y2="14" class="stroke-drawn-stem" stroke-width="1.2" />
            <path d="M5 3 l7 1.8 l-7 2.2 z" class="fill-accent" />
          {:else if each.mark === "lamp"}
            <line x1="8" y1="6" x2="8" y2="14" class="stroke-drawn-part" stroke-width="1.2" />
            <circle cx="8" cy="4.4" r="2.6" class="fill-alert" />
          {:else if each.mark === "plinth"}
            <rect x="1" y="6" width="14" height="4" rx="2" class="fill-drawn-line" />
            <rect x="1" y="6" width="8" height="4" rx="2" class="fill-accent" />
          {/if}
        </svg>
        <span>{each.label}</span>
      </li>
    {/each}
  </ul>
</details>

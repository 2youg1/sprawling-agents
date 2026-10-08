<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // How a popover is drawn, and nothing else: every word arrives
  // translated and every key and press arrives wired (`./popover_wiring`,
  // `PopoverLook`), and each wire bag is spread on the element it names.
  // Another look - one built from a component library - draws the same
  // list by taking the same value.
  //
  // What is applied reads as the deeper wash. The cursor is a bar at the
  // row's leading edge, on an applied row as well, and takes the rung of
  // the surface only where nothing is applied, so the two fills never
  // paint over each other; the bar keeps its shape when forced colours
  // take every fill away.
  import type { PopoverLook } from "./popover_wiring";

  const look: PopoverLook = $props();
</script>

<!-- No stacking number on the panel below: a positioned box is painted
     after every box that is not positioned, so the list already covers
     the text box and the buttons it opens over. It rises into place
     from the side it opens on through the theme's entrances, which fall
     back to a cut under reduced motion. -->
<div
  class={[
    "absolute left-0 w-max min-w-full max-w-full rounded-panel border border-edge-panel bg-raised p-snug shadow-float",
    look.side === "above" ? "bottom-full mb-snug rise" : "top-full mt-snug drop",
  ]}
  {...look.dialog}
>
  {#if look.header !== undefined}{@render look.header()}{/if}
  <div class={["gap-snug", look.layout === "equal" ? "grid auto-cols-fr grid-flow-col" : "flex"]}>
    {#each look.lists as list (list.key)}
      <div class="flex min-w-0 flex-col">
        <div class="mb-tight px-snug text-note text-text-faint">{list.label}</div>
        <ul class="max-h-palette overflow-y-auto" {...list.wire}>
          {#each list.rows as item (item.key)}
            <li
              class={[
                "relative flex cursor-pointer items-center justify-between gap-snug rounded-control px-snug py-tight text-body",
                item.chosen
                  ? "applied wash-strong text-text"
                  : item.cursor
                    ? "bg-raised-hover text-text-quiet"
                    : "text-text-quiet",
                item.cursor && "cursor",
              ]}
              {...item.wire}
            >
              {#if look.row !== undefined}
                {@render look.row(item.row)}
              {:else}
                <!-- The name never shrinks; the secondary cell does.
                     Both are capped rather than one holding its width
                     against the other, so a one-word hint beside a long
                     label survives whole. -->
                <span class="min-w-0 max-w-[24ch] truncate">{item.row.label}</span>
                {#if item.row.secondary !== undefined}
                  <span class="min-w-0 max-w-[20ch] line-clamp-2 text-note text-text-faint"
                    >{item.row.secondary}</span
                  >
                {/if}
              {/if}
            </li>
          {:else}
            <li role="presentation" class="px-snug py-tight text-note text-text-faint">{look.empty}</li>
          {/each}
        </ul>
      </div>
    {/each}
  </div>
</div>

<style>
  .cursor::before {
    content: "";
    position: absolute;
    inset-block: var(--spacing-tight);
    inset-inline-start: 0;
    border-inline-start: var(--spacing-hair) solid var(--color-accent);
  }
  /* Forced colours drop the wash and keep an outline. */
  .applied {
    outline: 1px solid transparent;
    outline-offset: -1px;
  }
</style>

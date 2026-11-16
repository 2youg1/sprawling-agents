<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import type { PopoverLook } from "./popover_wiring";

  const look: PopoverLook = $props();

  const panelCap = $derived(
    look.room === null ? undefined : `calc(${String(look.room)}px - var(--spacing-snug) * 2)`,
  );
</script>

<div
  class={[
    "absolute left-0 z-(--layer-popover) w-max min-w-full max-w-full overflow-y-auto rounded-panel border border-edge-panel bg-raised p-snug shadow-float",
    look.side === "above" ? "bottom-full mb-snug rise" : "top-full mt-snug drop",
  ]}
  style:max-height={panelCap}
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
                "relative flex cursor-pointer items-center justify-between gap-snug rounded-control px-snug py-tight text-body transition-[background-color,color,scale] ease-leave active:scale-(--scale-press) active:ease-arrive still:active:scale-100",
                item.chosen
                  ? "applied wash-strong text-text active:bg-raised-hover"
                  : item.cursor
                    ? "bg-raised-hover text-text-quiet hover:text-text hover:ease-arrive active:wash-strong"
                    : "text-text-quiet hover:bg-raised-hover hover:text-text hover:ease-arrive active:wash-strong",
                item.cursor && "cursor",
              ]}
              {...item.wire}
            >
              {#if look.row !== undefined}
                {@render look.row(item.row)}
              {:else}
                <span class="min-w-0 max-w-[28ch] wrap-anywhere text-pretty">{item.row.label}</span>
                {#if item.row.secondary !== undefined}
                  <span class="min-w-0 max-w-[20ch] line-clamp-2 text-note text-text-faint"
                    >{item.row.secondary}</span
                  >
                {/if}
              {/if}
            </li>
          {:else}
            <li role="presentation" class="px-snug py-tight text-note text-text-faint">
              {look.empty}
            </li>
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

  .applied {
    outline: 1px solid transparent;
    outline-offset: -1px;
  }
</style>

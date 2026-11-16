<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  import type { ComboboxLook } from "./combobox";

  const look: ComboboxLook = $props();
</script>

<div class="relative flex w-full min-w-0 flex-col gap-tight" {...look.root}>
  <button
    class="flex h-control w-full min-w-0 items-center justify-between gap-snug rounded-control border border-edge-input bg-raised px-base text-body text-text hover:bg-raised-hover"
    {...look.trigger}
  >
    <span class={["truncate", look.unset && "text-text-faint"]} style:font-family={look.faceStyle}>
      {look.face}
    </span>
  </button>
  {#if look.layer !== undefined}
    <div
      class={[
        "layer absolute left-0 z-(--layer-popover) w-full flex-col rounded-panel border border-edge-panel bg-raised p-tight shadow-float",
        look.side === "below" ? "top-full mt-tight" : "bottom-full mb-tight",
        look.open && "open",
      ]}
      data-side={look.side}
      {...look.layer}
    >
      <input
        class="mb-tight h-control w-full rounded-control bg-raised px-base text-body text-text placeholder:text-text-faint"
        {...look.search}
      />
      <ul class="max-h-output overflow-y-auto" {...look.list}>
        {#each look.options as option (option.key)}
          <li
            class={[
              "relative flex cursor-pointer items-baseline justify-between gap-snug rounded-control px-base py-snug text-body",
              option.chosen
                ? "applied wash-strong text-text"
                : option.cursor
                  ? "bg-raised-hover text-text"
                  : "text-text-quiet",
              option.cursor && "cursor",
            ]}
            {...option.wire}
          >
            <span class="truncate" style:font-family={option.face}>{option.label}</span>
            {#if option.note !== undefined}
              <span class="shrink-0 text-note text-text-faint">{option.note}</span>
            {/if}
          </li>
        {:else}
          <li role="presentation" class="px-base py-snug text-note text-text-faint">
            {look.empty}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .layer {
    display: none;
    opacity: 0;
    translate: 0 calc(-1 * var(--spacing-snug));
    transition:
      opacity var(--transition-duration-short) var(--ease-leave),
      translate var(--transition-duration-short) var(--ease-leave),
      display var(--transition-duration-short) allow-discrete;
  }
  .layer[data-side="above"] {
    translate: 0 var(--spacing-snug);
  }
  .layer.open {
    display: flex;
    opacity: 1;
    translate: 0 0;
    transition-duration: var(--transition-duration-panel);
    transition-timing-function: var(--ease-arrive);
  }
  @starting-style {
    .layer.open {
      opacity: 0;
      translate: 0 calc(-1 * var(--spacing-snug));
    }
    .layer.open[data-side="above"] {
      translate: 0 var(--spacing-snug);
    }
  }
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

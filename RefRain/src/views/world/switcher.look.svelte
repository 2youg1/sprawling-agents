<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The switcher's drawing only: a filter above a tree. All keyboard,
  // role and aria decisions arrive in SwitcherLook.
  import type { SwitcherLook } from "./switcher";

  const look: SwitcherLook = $props();
</script>

<div class="flex min-h-0 flex-col gap-snug">
  <input
    {...look.box}
    class="h-control rounded-control bg-page px-base text-body text-text"
    placeholder={look.box["aria-label"]}
  />
  {#if look.empty !== undefined}
    <p class="px-base py-snug text-note text-text-faint">{look.empty}</p>
  {:else}
    <div {...look.tree} class="max-h-palette overflow-y-auto">
      {#each look.rows as row (row.key)}
        <div
          {...row.wire}
          class={[
            "flex w-full cursor-pointer items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body transition-[background-color,color] ease-leave aria-disabled:text-text-disabled",
            row.wire["aria-selected"]
              ? "bg-raised-hover ease-arrive"
              : "hover:bg-raised-hover hover:ease-arrive active:wash-strong active:ease-arrive",
          ]}
          style:padding-inline-start={`calc(${String(row.depth)} * var(--spacing-base) + var(--spacing-base))`}
        >
          <span class="max-w-full shrink-0 truncate font-mono">{row.label}</span>
          <span class="min-w-0 shrink-1000 truncate text-note text-text-faint">{row.hint}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

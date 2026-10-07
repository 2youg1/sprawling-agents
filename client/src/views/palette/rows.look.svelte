<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the palette's list is drawn, and nothing else (`./rows.ts`,
  // `RowsLook`): each row is a menu entry - its label in mono, and at its
  // end the key's chord or a quieter line. An entry rests on the raised
  // sheet and lifts one step, to `raised-hover`, under the pointer and
  // under the cursor (docs/frontend-method.md §7A-5); the lift arrives
  // and leaves on the theme's two curves (§4-43). A verb this place
  // cannot run is drawn in the disabled ink and never lifts under the
  // pointer, but the cursor still marks it, because the cursor is how a
  // keyboard reaches its reason.
  import { Kbd } from "../parts/kbd.svelte";
  import type { RowsLook } from "./rows";

  const look: RowsLook = $props();
</script>

<div {...look.list} class="max-h-palette overflow-y-auto">
  {#each look.sections as section (section.key)}
    <div {...section.wire}>
      {#if section.heading !== undefined}
        <div {...section.heading.wire} class="px-base pt-snug text-label font-label text-text-faint">
          {section.heading.text}
        </div>
      {/if}
      {#each section.rows as row (row.key)}
        <div
          {...row.wire}
          class={[
            "flex w-full cursor-pointer items-center justify-between gap-snug rounded-control px-base py-snug text-left text-body",
            "transition-colors ease-leave aria-disabled:text-text-disabled",
            row.refused ? "" : "hover:bg-raised-hover hover:ease-arrive",
            row.active ? "bg-raised-hover ease-arrive" : "",
          ]}
        >
          <span class="truncate font-mono">{row.label}</span>
          <span class="shrink-0 text-note text-text-faint">
            {#if row.end.kind === "chord"}
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
              {@render Kbd({ action: row.end.action })}
            {:else}
              {row.end.text}
            {/if}
          </span>
        </div>
      {/each}
    </div>
  {/each}
</div>

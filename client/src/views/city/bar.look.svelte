<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the city's row of figures is drawn, and nothing else
  // (`./bar.ts`, `BarLook`): a description list in as many columns as
  // the page holds, each figure under its word and closed by one rule.
  // A figure with a page to act on is a link whose ink turns to the
  // accent under the pointer, arriving on the arrive curve and leaving
  // on leave (docs/frontend-method.md §4-43).
  import type { BarLook } from "./bar";

  const look: BarLook = $props();

  const INK = { alert: "text-alert", plain: "text-text" } as const;
</script>

<dl
  class="grid grid-cols-[repeat(auto-fit,minmax(16ch,1fr))] gap-x-gutter gap-y-base border-b border-edge pb-base"
  aria-label={look.label}
>
  {#each look.readings as reading (reading.key)}
    <div class="flex min-w-0 flex-col">
      <dt class="truncate text-note text-text-faint">{reading.label}</dt>
      <dd class="text-heading figure">
        {#if reading.wire !== null}
          <a {...reading.wire} class={["transition-colors ease-leave hover:text-accent hover:ease-arrive", INK[reading.tone]]}>{reading.value}</a>
        {:else}
          <span class={INK[reading.tone]}>{reading.value}</span>
        {/if}
      </dd>
    </div>
  {/each}
</dl>

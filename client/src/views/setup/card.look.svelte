<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import type { CardLook } from "./card";

  const { title, note, constraint, receipt, children }: CardLook = $props();
</script>

<!-- The title and its line sit as one heading block, as on the cards a
press saves, so the two kinds of card share one rhythm in one grid. -->
<div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
  <div class="flex flex-col gap-hair">
    <span class="text-label font-label text-text">{title}</span>
    {#if note !== undefined}
      <p class="text-note text-text-faint">{note}</p>
    {/if}
  </div>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render children()}
  {#if constraint !== undefined || receipt !== undefined}
    <div class="flex items-baseline justify-between gap-base">
      {#if constraint !== undefined}
        <span class="text-note text-text-faint">{constraint}</span>
      {/if}
      {#if receipt !== undefined}
        <span {...receipt.wire} class="ml-auto inline-flex items-center gap-tight text-note text-text-quiet">
          {#if receipt.said !== undefined}
            <span class="fade inline-flex items-center gap-tight">
              <Glyph name="check" size="sm" class="shrink-0" />
              {receipt.said}
            </span>
          {/if}
        </span>
      {/if}
    </div>
  {/if}
</div>

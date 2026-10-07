<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the page frame is drawn (`./page`, `PageLook`): the header line
  // with its rule, and the body under it.
  import type { PageLook } from "./page";

  const look: PageLook = $props();
</script>

<div class="flex w-full min-w-0 flex-1 flex-col gap-wide pb-section">
  <header class="flex min-w-0 flex-wrap items-end justify-between gap-x-wide gap-y-snug border-b border-edge pb-base">
    <div class="flex min-w-0 flex-col gap-tight">
      <svelte:element this={look.rank === "page" ? "h1" : "h2"} class="text-title font-title" {...look.heading}>
        {look.title}
      </svelte:element>
      {#if look.note !== undefined}
        <p class="text-note text-text-quiet">{look.note}</p>
      {/if}
    </div>
    {#if look.aside !== undefined}
      <div class="flex min-w-0 flex-wrap items-center gap-base">
        {@render look.aside()}
      </div>
    {/if}
  </header>
  {@render look.children()}
</div>

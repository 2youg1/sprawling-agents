<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import type { HeadLook } from "./head";

  const look: HeadLook = $props();
</script>

<div class="head">
  <div class="where">
    <p class="flex min-w-0 items-baseline truncate">
      <span class="truncate text-text-faint">{look.place.folder}</span><span class="text-text-quiet"
        >{look.place.name}</span
      >
    </p>
    {#if look.version !== null}
      <span class="shrink-0 text-text-faint">{look.version}</span>
    {/if}
    <span
      {...look.wire}
      class={["receipt", look.receipt === null && "sr-only"]}
      data-alert={look.receipt?.alert ?? false}>{look.receipt?.text ?? ""}</span
    >
  </div>
  <div class="flex shrink-0 items-center gap-snug">
    {@render look.actions()}
  </div>
</div>

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-tight) var(--spacing-base);
    min-height: var(--spacing-control);
    padding: 0 var(--spacing-wide);
    border-bottom: 1px solid var(--color-edge);
    font-size: var(--text-note);
    white-space: nowrap;
  }
  .where {
    display: flex;
    flex: 1 1 24ch;
    align-items: baseline;
    gap: var(--spacing-base);
    min-width: 0;
  }
  .receipt {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--spacing-tight);
    align-self: center;
    color: var(--color-text-quiet);
  }
  .receipt[data-alert="true"] {
    color: var(--color-alert);
  }
  .receipt[data-alert="true"]::before {
    content: "";
    width: var(--spacing-dot);
    height: var(--spacing-dot);
    border-radius: 50%;
    background: var(--color-alert);
  }
</style>

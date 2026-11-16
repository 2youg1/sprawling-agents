<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { TabsLook } from "./tabs";

  const look: TabsLook = $props();
</script>

<div class="flex flex-wrap items-center gap-tight border-b border-edge" {...look.list}>
  {#each look.tabs as tab (tab.lens.id)}
    <button
      type="button"
      class={[
        "tab flex items-center gap-tight px-base py-snug text-label",
        tab.current && "current",
      ]}
      {...tab.wire}
    >
      {tab.lens.label}
      {@render look.mark?.(tab.lens)}
    </button>
  {/each}
</div>
{#each look.panels as panel (panel.lens.id)}
  <div {...panel.wire}>
    {#if panel.shown}
      {@render look.panel(panel.lens)}
    {/if}
  </div>
{/each}

<style>
  .tab {
    position: relative;
    color: var(--color-text-quiet);
    transition: color var(--transition-duration-short) var(--ease-leave);
  }
  .tab:hover,
  .tab.current {
    color: var(--color-text);
    transition-timing-function: var(--ease-arrive);
  }

  .tab::after {
    content: "";
    position: absolute;
    inset-inline: 0;
    bottom: -1px;
    border-block-end: var(--spacing-hair) solid var(--color-accent);
    transform: scaleX(0);
    transition: transform var(--transition-duration-short) var(--ease-leave);
  }

  .tab.current::after {
    transform: scaleX(1);
    transition-timing-function: var(--ease-arrive);
  }
</style>

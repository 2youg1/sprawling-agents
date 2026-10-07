<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How a set of tabs is drawn, and nothing else: every name arrives
  // translated and every role, id, key and focus arrives decided in the
  // wire bags (`./tabs`, `TabsLook`), which this look spreads on the
  // element each one names. The bar under the current tab is the one
  // accent this part takes (docs/frontend-method.md §7B); every other
  // tab is said by its own ink. In a column too narrow for every name
  // the strip wraps rather than running past its frame, so no lens is
  // hidden behind a scroll the person cannot see.
  import type { TabsLook } from "./tabs";

  const look: TabsLook = $props();
</script>

<div class="flex flex-wrap items-center gap-tight border-b border-edge" {...look.list}>
  {#each look.tabs as tab (tab.lens.id)}
    <button
      type="button"
      class={["tab flex items-center gap-tight px-base py-snug text-label", tab.current && "current"]}
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
  /* The bar sits on the list's rule rather than above it, so the current
   * tab and the rule read as one line. It grows from the middle as its
   * tab arrives and shrinks as it leaves; leaving accelerates and
   * arriving decelerates (§4-43), so the base state names the leaving
   * curve and the current state the arriving one. The ink follows the
   * same two curves on hover. The bar is a border rather than a fill
   * because forced colours keep a border in the system's ink and
   * paint a fill over with the page. */
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

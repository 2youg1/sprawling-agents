<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import Glyph from "../parts/glyph.svelte";

  import type { StripLook } from "./strip";

  const look: StripLook = $props();
</script>

<div class="strip flex shrink-0 items-stretch border-b border-edge pr-snug text-note">
  <div {...look.list} class="flex min-w-0 flex-1 items-stretch overflow-x-auto">
    {#each look.tabs as tab (tab.key)}
      <div
        class={[
          "tab group/tab relative flex shrink-0 items-center gap-snug border-r border-edge pr-snug first:pl-wide",
          tab.front ? "front bg-page text-text" : "text-text-faint hover:text-text-quiet",
        ]}
      >
        <a
          {...tab.wire}
          class="name flex h-full items-center gap-snug pl-pane whitespace-nowrap group-first/tab:pl-0"
        >
          {#if tab.terminal}<Glyph name="terminal" size="sm" />{/if}
          <span class="truncate">{tab.label}</span>
          {#if tab.unsaved !== undefined}
            <span class="size-dot shrink-0 rounded-pill bg-alert" aria-hidden="true"></span>
            <span class="sr-only">{tab.unsaved}</span>
          {/if}
        </a>
        <span class="flex size-glyph-sm shrink-0">
          <button
            {...tab.close}
            class={[
              "mark relative size-glyph-sm items-center justify-center rounded-control text-text-faint before:absolute before:-inset-tight before:content-[''] hover:bg-raised hover:text-text",
              tab.front ? "flex" : "hidden group-focus-within/tab:flex group-hover/tab:flex",
            ]}
          >
            <Glyph name="cross" size="sm" />
          </button>
        </span>
      </div>
    {/each}
  </div>
  <button
    {...look.closeAll}
    class="key my-auto ml-snug flex size-control-sm items-center justify-center rounded-control text-text-faint hover:bg-raised hover:text-text"
  >
    <Glyph name="cross" />
  </button>
</div>

<style>
  .strip {
    height: calc(6 * var(--spacing-baseline));
  }

  .name {
    max-width: calc(24 * var(--spacing-baseline));
  }

  .front::after {
    content: "";
    position: absolute;
    inset-inline: 0;
    bottom: -1px;
    height: 1px;
    background-color: var(--color-page);
  }

  .tab,
  .mark,
  .key {
    transition:
      color var(--transition-duration-short) var(--ease-leave),
      background-color var(--transition-duration-short) var(--ease-leave);
  }
  .tab:hover,
  .tab:focus-within,
  .mark:hover,
  .key:hover {
    transition-timing-function: var(--ease-arrive);
  }

  @media (forced-colors: active) {
    .front .name {
      text-decoration: underline;
      text-underline-offset: var(--spacing-tight);
    }
  }
</style>

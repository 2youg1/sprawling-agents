<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the inspector's tab strip is drawn, and nothing else (client
  // D95): a row of tabs cut from the region below - the front one on the
  // page's own fill, its bottom edge open into the region - each with a
  // close mark, and the key that closes the whole inspector at the right
  // end. Every word arrives translated and every role, tab stop, handler
  // and `aria-*` value arrives inside a wire bag spread unchanged on the
  // element it is for.
  //
  // A tab keeps the same padding in front and behind, so bringing it
  // forward changes its fill and never moves its words. The close mark
  // of a tab behind shows while the pointer or the focus is on that tab,
  // which is when a hand can reach it.
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
        <a {...tab.wire} class="name flex h-full items-center gap-snug pl-pane whitespace-nowrap group-first/tab:pl-0">
          {#if tab.terminal}<Glyph name="terminal" size="sm" />{/if}
          <span class="truncate">{tab.label}</span>
          {#if tab.unsaved !== undefined}
            <span class="size-dot shrink-0 rounded-pill bg-alert" aria-hidden="true"></span>
            <span class="sr-only">{tab.unsaved}</span>
          {/if}
        </a>
        <button
          {...tab.close}
          class={[
            "mark relative flex size-glyph-sm items-center justify-center rounded-control text-text-faint before:absolute before:-inset-tight before:content-[''] hover:bg-raised hover:text-text",
            tab.front ? "" : "opacity-0 group-focus-within/tab:opacity-100 group-hover/tab:opacity-100",
          ]}
        >
          <Glyph name="cross" size="sm" />
        </button>
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

  /* The front tab's fill runs over the strip's bottom rule, so the tab
   * and the region below read as one sheet. */
  .front::after {
    content: "";
    position: absolute;
    inset-inline: 0;
    bottom: -1px;
    height: 1px;
    background-color: var(--color-page);
  }

  /* Ink, fill and the close mark's appearance fade back when the pointer
   * leaves and arrive when it comes: the resting state leaves, the
   * hovered state arrives (docs/frontend-method.md §4-43). */
  .tab,
  .mark,
  .key {
    transition:
      color var(--transition-duration-short) var(--ease-leave),
      background-color var(--transition-duration-short) var(--ease-leave),
      opacity var(--transition-duration-short) var(--ease-leave);
  }
  .tab:hover,
  .tab:focus-within,
  .tab:hover .mark,
  .tab:focus-within .mark,
  .key:hover {
    transition-timing-function: var(--ease-arrive);
  }

  /* A forced-colour mode paints every fill alike, so the front tab would
   * look like the others; its name keeps a rule under it instead. */
  @media (forced-colors: active) {
    .front .name {
      text-decoration: underline;
      text-underline-offset: var(--spacing-tight);
    }
  }
</style>

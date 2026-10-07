<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the code view is drawn, and nothing else (`./code`, `CodeLook`).
  //
  // The row that names the file also carries the one secondary action:
  // copy, shown when the pointer or the keyboard is inside this view and
  // nowhere else, so the code is the only thing that greets the eye. At
  // rest the key is invisible and click-through, so the corner never
  // stands between a hand and the text under it, and it arrives as
  // `fade` - the motion vocabulary's name for a state change that keeps
  // its position; a machine that asks for less motion gets the key
  // without the arrival.
  import Copy from "./copy.svelte";
  import Inked from "./inked.svelte";
  import type { CodeLook } from "./code";

  const look: CodeLook = $props();
</script>

<div class="group flex min-h-0 min-w-0 flex-col">
  <div class="flex items-center gap-tight border-b border-edge px-snug py-tight text-note">
    {#if look.crumbs.length > 0}
      <nav class="flex min-w-0 flex-wrap items-center gap-tight" {...look.trail}>
        {#each look.crumbs as crumb, at (at)}
          {#if at > 0}<span class="text-text-faint" aria-hidden="true">/</span>{/if}<span
            class={crumb.last ? "text-text-quiet" : "text-text-faint"}>{crumb.part}</span
          >
        {/each}
      </nav>
    {/if}
    <span
      class={[
        "pointer-events-none ms-auto flex shrink-0 opacity-0 transition-opacity ease-leave still:transition-none",
        "group-hover:pointer-events-auto group-hover:opacity-100 group-hover:ease-arrive",
        "group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-focus-within:ease-arrive",
      ]}
    >
      <Copy {...look.copy} />
    </span>
    {#if look.aside !== undefined}<span class="flex shrink-0 items-center">{@render look.aside()}</span>{/if}
  </div>
  <!-- The code scrolls sideways and never folds a line in half: the
  gutter stays put at the left while the text runs under it. -->
  <div class="min-h-0 flex-1 overflow-auto" {@attach look.scroller}>
    <div class="relative flex min-w-max font-mono text-note leading-relaxed">
      <pre class="sticky left-0 shrink-0 select-none bg-chrome px-snug text-right text-text-faint" aria-hidden="true">{look.gutter}</pre>
      <pre class="px-snug text-text-quiet"><Inked text={look.text} source={look.path} /></pre>
      {#if look.cited !== undefined && look.cited >= 1}
        <div
          {@attach look.mark}
          class="cited pointer-events-none absolute inset-x-0 h-lh bg-accent/12"
          style:top="calc({look.cited - 1} * 1lh)"
          aria-hidden="true"
        ></div>
      {/if}
    </div>
  </div>
</div>

<style>
  /* The cited line's accent bar, drawn inside its left edge so it takes
  no width from the code. */
  .cited {
    box-shadow: inset var(--spacing-hair) 0 0 var(--color-accent);
  }
</style>

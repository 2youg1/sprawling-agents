<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One section of the mailbox column: a heading that stays at the top
  // of the column while its entries scroll under it, how many entries
  // it holds, and the section's own controls at its right end. With
  // nothing in it, one line under the heading says what is missing, on
  // the column's own left edge: the section is one part of a screen
  // whose heading already names it, so the empty seat of a whole screen
  // (`parts/empty.svelte`, with its outline and inset) is not its shape.
  import type { Snippet } from "svelte";

  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";

  interface Props {
    readonly title: Key;
    readonly count: number;
    // What the line says when the section holds nothing.
    readonly empty: Key;
    readonly tools?: Snippet | undefined;
    readonly children: Snippet;
  }

  const { title, count, empty, tools, children }: Props = $props();

  const { lang } = ui();
  const id = $props.id();
</script>

<section class="mt-wide first:mt-snug" aria-labelledby="{id}-title">
  <h3
    class="sticky top-0 flex h-control items-center gap-snug border-b border-edge bg-raised text-note text-text-faint"
  >
    <span id="{id}-title">{say($lang, title)}</span>
    <span class="figure text-text-quiet">{count}</span>
    <span class="flex-1"></span>
    {#if tools !== undefined}{@render tools()}{/if}
  </h3>
  {#if count === 0}
    <p class="py-snug text-note text-text-quiet">{say($lang, empty)}</p>
  {/if}
  {@render children()}
</section>

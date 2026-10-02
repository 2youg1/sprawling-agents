<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The frame every page but the conversation stands in: one header line
// - the title on the left, what the page states about itself as a whole
// (a total, a scope, a count) on the right - and the page's body under
// it, as wide as the main region.
//
// **One frame, so one left edge and one header.** Each page drew its
// own heading with its own padding and its own cap, and at a desktop
// width the result was a column of 1040px pinned to the left with a
// figure floating at its far end. The body is not capped here: a table,
// a list and a grid of cards use the width they are given, and a region
// that holds prose caps itself at the measure where it is drawn.
//
// **The frame adds no inset of its own.** It stands in the shell's
// `<main>`, whose left edge is the line of column 2 (docs/frontend-method.md §4-33,
// 4-50), so the title, the rule under it and the body all start on that
// line rather than a padding's width to the right of it.

import type { Snippet } from "svelte";

export interface PageProps {
  readonly title: string;
  // Whether this frame is the page or a region inside one: a document
  // may have exactly one heading of the page's own rank.
  readonly rank?: "page" | "section" | undefined;
  // One line under the title saying what the page governs.
  readonly note?: string | undefined;
  // One line over the title saying what the page belongs to: the way
  // back up, such as the city above a building.
  readonly above?: Snippet | undefined;
  // What stands at the right end of the header line.
  readonly aside?: Snippet | undefined;
  readonly children: Snippet;
}
</script>

<script lang="ts">
  const { title, rank = "page", note, above, aside, children }: PageProps = $props();
</script>

<div class="flex w-full min-w-0 flex-1 flex-col gap-wide pb-section">
  <header class="flex min-w-0 flex-wrap items-end justify-between gap-x-wide gap-y-snug border-b border-edge pb-base">
    <div class="flex min-w-0 flex-col gap-tight">
      {#if above !== undefined}
        <div class="flex min-w-0 items-center gap-snug text-note text-text-faint">{@render above()}</div>
      {/if}
      {#if rank === "page"}
        <h1 class="text-title font-title" tabindex="-1">{title}</h1>
      {:else}
        <h2 class="text-title font-title">{title}</h2>
      {/if}
      {#if note !== undefined}
        <p class="text-note text-text-quiet">{note}</p>
      {/if}
    </div>
    {#if aside !== undefined}
      <div class="flex min-w-0 flex-wrap items-center gap-base">
        {@render aside()}
      </div>
    {/if}
  </header>
  {@render children()}
</div>

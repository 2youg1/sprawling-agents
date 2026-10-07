<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // How the runs board is drawn, and nothing else (`./board.ts`,
  // `BoardLook`): the heading and its counts; the runs waiting for the
  // person, each a key that brings its row into view; the legend; the
  // folded clock's scale; the tree, a scrolling list whose drawn rows
  // are padded above and below to the whole tree's height; and the line
  // that names the keys. Every role, `aria-*` value, key and scroll
  // handler arrives in a wire bag spread unchanged on its element.
  //
  // A row is a row of the page: the pointer lays the page's wash over
  // it, and the cursor row the stronger wash with the accent edge on its
  // left (docs/frontend-method.md §7A-4, §7B); the wash arrives on the
  // arrive curve and leaves on leave (§4-43).
  import Glyph from "../parts/glyph.svelte";
  import type { BoardLook } from "./board";
  import { PHASE_FILL, PHASE_MARK } from "./phase";

  const look: BoardLook = $props();

  const INK = { quiet: "text-text-quiet", live: "text-accent", alert: "text-alert" } as const;
</script>

<div class="flex min-w-0 flex-col gap-base">
  <header class="flex flex-wrap items-baseline gap-x-base gap-y-snug">
    <svelte:element this={look.level === 1 ? "h1" : "h2"} class="text-title font-title" tabindex="-1">{look.title}</svelte:element>
    <p class="figure text-note text-text-quiet">{look.counts}</p>
  </header>

  {#if look.asking !== null}
    <!-- The word stands above the runs rather than in their row, so a
    row that wraps starts every line at the same edge with a run. -->
    <nav class="flex flex-col gap-tight text-note" aria-label={look.asking.label}>
      <span class="text-alert">{look.asking.label}</span>
      <div class="flex flex-wrap items-center gap-snug">
        {#each look.asking.runs as run (run.key)}
          <button
            {...run.wire}
            class="inline-flex items-center gap-tight rounded-pill border border-edge-input px-snug font-mono text-text transition-colors ease-leave hover:bg-raised hover:ease-arrive"
          >
            <Glyph name="hand" size="sm" class="text-alert" />{run.text}
          </button>
        {/each}
      </div>
    </nav>
  {/if}

  <ul class="flex flex-wrap gap-x-wide gap-y-tight text-note text-text-quiet" aria-label={look.legend.label}>
    {#each look.legend.phases as each (each.phase)}
      <li class="flex items-center gap-snug"><span class={["inline-block h-snug w-base rounded-pill", PHASE_FILL[each.phase]]} aria-hidden="true"></span>{each.word}</li>
    {/each}
  </ul>

  <div class="flex min-w-0 items-end gap-snug border-b border-edge pb-tight font-mono figure text-note text-text-quiet" aria-hidden="true">
    <span class="min-w-0 flex-1"></span>
    <span class="relative h-base w-[40%] shrink-0">
      {#each look.folds as fold (fold.minutes)}
        <span class={["absolute top-0 border-l border-edge-input pl-tight", fold.wide ? "hidden @lg/page:inline" : ""]} style:left="{String(fold.at)}%">{fold.label}</span>
      {/each}
      <span class="absolute top-0 right-0 font-sans text-text">{look.now}</span>
    </span>
  </div>

  <ul
    {...look.tree}
    class="max-h-[70dvh] min-w-0 overflow-y-auto border-b border-edge"
    style:padding-top="{String(look.pad.above)}px"
    style:padding-bottom="{String(look.pad.below)}px"
  >
    {#each look.rows as row (row.key)}
      <li
        {...row.wire}
        class={[
          "flex h-step min-w-0 cursor-pointer items-center gap-snug pr-snug text-note transition-colors ease-leave hover:wash hover:ease-arrive",
          row.current ? "wash-strong shadow-[inset_2px_0_0_var(--color-accent)]" : "",
        ]}
      >
        <span class="shrink-0 pl-snug font-mono whitespace-pre text-text-quiet" aria-hidden="true">{row.guide}</span>
        {#if row.body.kind === "building"}
          <span class="min-w-0 flex-1 truncate font-mono font-label text-text">{row.body.name}</span>
          <span class={["inline-flex shrink-0 items-center gap-tight font-mono figure", row.body.asking === 0 ? "text-text-quiet" : "text-alert"]}><Glyph name="hand" size="sm" />{String(row.body.asking)}</span>
          <span class="shrink-0 font-mono figure text-text-quiet">{String(row.body.runs)}</span>
        {:else if row.body.kind === "room"}
          <span class="min-w-0 flex-1 truncate font-mono text-text-quiet">{row.body.name}</span>
        {:else if row.body.kind === "run"}
          {@const mark = PHASE_MARK[row.body.phase]}
          <Glyph name={mark.glyph} size="sm" class={["shrink-0", INK[mark.weight]]} />
          <span class="hidden shrink-0 font-mono text-text-quiet @lg/page:inline">{row.body.id}</span>
          <span class="min-w-0 flex-1 truncate text-text">{row.body.title}</span>
          <span class={["hidden max-w-[30%] shrink-0 truncate text-note @lg/page:inline", INK[mark.weight]]}>{row.body.said}</span>
          <span class="relative h-snug w-[40%] shrink-0" aria-hidden="true">
            <span class="absolute inset-y-0 rounded-pill bg-edge" style:left="{row.body.bar.left.toFixed(3)}%" style:width="{row.body.bar.width.toFixed(3)}%"></span>
            <span
              class={["absolute inset-y-0 w-snug rounded-pill", PHASE_FILL[row.body.phase]]}
              style:left="clamp(0px, calc({row.body.bar.left.toFixed(3)}% + {row.body.bar.width.toFixed(3)}% - var(--spacing-snug)), calc(100% - var(--spacing-snug)))"
            ></span>
          </span>
        {/if}
      </li>
    {/each}
  </ul>

  <p class="text-note text-text-quiet">{look.keys}</p>
</div>

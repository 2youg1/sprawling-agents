<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The performance panel: a row per counter, its latest reading, and
  // its curve under it, drawn in the blocks `sprawling top` draws in so
  // the page and the terminal show one history the same way.
  //
  // **The panel is what makes the city sample.** `watch` is called when
  // the panel opens and what it returns is called when it closes, so a
  // city nobody is looking at reads no counter at all (sprawling-SPEC
  // 8-94).
  //
  // **A curve is as long as the panel is wide.** Each point is one
  // glyph, so the panel measures one glyph and the width it has, and
  // asks for that many of the latest points; a narrow panel shows the
  // last minute, a wide one all five.
  //
  // **On a wide page the counters stand in cards**, as many to a row as
  // the page holds, so a reading sits beside its own curve instead of at
  // the far end of a line across the whole window. Each card measures
  // itself, so its curve is as long as the card.

  import type { Sample } from "../wire";

  export interface MonitorProps {
    readonly samples: readonly Sample[];
    readonly watch: () => () => void;
    // The shell draws this panel as a page; the gallery draws it as one
    // region among many, where it may not carry the page's heading.
    readonly rank?: "page" | "section" | undefined;
  }
</script>

<script lang="ts">
  import { say } from "../core/lang";
  import { rows } from "../core/monitor";
  import { ui } from "../ui";

  import Page from "./parts/page.svelte";

  const { samples, watch, rank = "page" }: MonitorProps = $props();

  const { lang } = ui();

  let panel = $state(0);
  let glyph = $state(0);

  $effect(() => watch());

  const shown = $derived(rows(samples, glyph > 0 ? Math.floor(panel / glyph) : 0));
</script>

<Page title={say($lang, "monitor_title")} {rank}>
<section
  class="relative flex min-w-0 flex-col gap-snug"
  aria-label={say($lang, "monitor_title")}
>
  <span aria-hidden="true" class="invisible absolute font-mono text-note" bind:clientWidth={glyph}
    >▁</span
  >
  {#if shown.length === 0}
    <p class="text-note text-text-faint">{say($lang, "monitor_waiting")}</p>
  {:else}
    <ul class="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-base">
      {#each shown as row, at (row.label)}
        <li class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-pane py-base">
          <div class="flex items-baseline justify-between gap-snug">
            <span class="truncate text-label text-text-quiet">{say($lang, row.label)}</span>
            <span class="shrink-0 font-mono text-note text-text tabular-nums">{row.reading}</span>
          </div>
          {#if at === 0}
            <div aria-hidden="true" class="w-full" bind:clientWidth={panel}></div>
          {/if}
          <div
            aria-hidden="true"
            class="overflow-hidden whitespace-nowrap font-mono text-note leading-none text-accent"
          >
            {row.curve}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>
</Page>

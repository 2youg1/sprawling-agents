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

  const CURVE =
    "col-span-full block min-w-0 overflow-hidden whitespace-nowrap pt-tight font-mono text-note leading-none text-accent @min-[40rem]:col-span-1 @min-[40rem]:pt-0";
</script>

<Page title={say($lang, "monitor_title")} {rank}>
<section
  class="@container relative flex min-w-0 flex-col"
  aria-label={say($lang, "monitor_title")}
>
  <span aria-hidden="true" class="invisible absolute font-mono text-note" bind:clientWidth={glyph}
    >▁</span
  >
  {#if shown.length === 0}
    <p class="text-note text-text-faint">{say($lang, "monitor_waiting")}</p>
  {:else}
    <!-- One counter a row, ruled like a table: its name, its latest
         reading set right on the figure column, then its curve to the
         end of the line. Under 40rem the curve takes a line of its own. -->
    <ul>
      {#each shown as row, at (row.label)}
        <li
          class="grid grid-cols-[minmax(0,1fr)_14ch] items-baseline gap-x-gutter border-b border-edge py-snug @min-[40rem]:grid-cols-[minmax(0,28ch)_14ch_minmax(0,1fr)]"
        >
          <span class="truncate text-note text-text-quiet">{say($lang, row.label)}</span>
          <span class="figure text-right text-text">{row.reading}</span>
          <!-- The first curve measures the room every curve has, so the
               panel asks for as many points as one line holds. -->
          {#if at === 0}
            <span aria-hidden="true" class={CURVE} bind:clientWidth={panel}>{row.curve}</span>
          {:else}
            <span aria-hidden="true" class={CURVE}>{row.curve}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
</Page>

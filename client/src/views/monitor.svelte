<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The performance panel: a row per counter, its latest reading, the
  // p50 and p99 of the window it shows, and its bars beside them.
  //
  // **The panel is what makes the city sample.** `watch` is called when
  // the panel opens and what it returns is called when it closes, so a
  // city nobody is looking at reads no counter at all (`crates/sprawling/Spec.lean`
  // §8-94).
  //
  // **A plot is as long as the panel is wide.** Each point is one bar
  // `POINT_PX` wide, so the panel measures the width a plot has and asks
  // for that many of the latest points; a narrow panel shows the last
  // minute, a wide one all five.
  //
  // **The bars scale to their own window** (`core/monitor.ts`'s `plot`,
  // D82 in `client/Spec.lean`): the lowest value of the window stands
  // low and the highest at full height, with a band from the window's
  // p50 to its p99 behind them, so a counter that moves by a few percent
  // still shows its shape. Two plots' heights do not compare; the
  // figures beside them do.

  import type { Sample } from "../wire";

  // The beats a User picks from, in milliseconds: the city's default,
  // finer ones for a short spike, and coarser ones for a long watch. The
  // city takes any beat from 10 to 1000 ms (`wire::BeatMs`); this list
  // is the page's offer, so changing it changes no rule.
  export const BEATS = ["50", "100", "250", "500", "1000"] as const;
  export type BeatChoice = (typeof BEATS)[number];

  export function beatOf(latest: Sample | undefined): BeatChoice | null {
    return BEATS.find((beat) => Number(beat) === latest?.beat_ms) ?? null;
  }

  export interface MonitorProps {
    readonly samples: readonly Sample[];
    readonly watch: () => () => void;
    // Sets the city's sampling beat; absent where nothing is watched
    // live, as in the gallery.
    readonly beat?: ((ms: number) => void) | undefined;
    // The shell draws this panel as a page; the gallery draws it as one
    // region among many, where it may not carry the page's heading.
    readonly rank?: "page" | "section" | undefined;
  }
</script>

<script lang="ts">
  import { fill, say } from "../core/lang";
  import { FULL, rows, type Plot } from "../core/monitor";
  import { ui } from "../ui";

  import Page from "./parts/page.svelte";
  import Segmented from "./parts/segmented.svelte";

  const { samples, watch, beat, rank = "page" }: MonitorProps = $props();

  const { lang } = ui();

  // The width of one point of a plot, its bar and the gap after it.
  const POINT_PX = 3;

  let panel = $state(0);

  $effect(() => watch());

  const points = $derived(Math.floor(panel / POINT_PX));
  const shown = $derived(rows(samples, points));

  // A plot's bars as one path in a box `bars.length` wide and `FULL`
  // high, one unit a point, so the browser stretches it to the plot.
  // The newest point stands at the right edge, so a history shorter
  // than the plot fills it from the right.
  function barsOf(plot: Plot, points: number): string {
    const first = points - plot.bars.length;
    return plot.bars
      .map((height, at) => `M${String(first + at)} ${String(FULL - height)}h0.7V${String(FULL)}h-0.7Z`)
      .join("");
  }

  const held = $derived(beatOf(samples.at(-1)));
  const beats = $derived(BEATS.map((value) => ({ value, label: fill(say($lang, "monitor_beat_ms"), { n: value }) })));

  const PLOT = "col-span-full block h-10 w-full min-w-0 pt-tight @min-[40rem]:col-span-1 @min-[40rem]:pt-0";
</script>

{#snippet beatControl()}
  {#if beat !== undefined}
    <span class="text-note text-text-quiet">{say($lang, "monitor_beat")}</span>
    <Segmented
      label={say($lang, "monitor_beat")}
      options={beats}
      {held}
      onPick={(value) => {
        beat(Number(value));
      }}
    />
  {/if}
{/snippet}

<Page title={say($lang, "monitor_title")} aside={beatControl} {rank}>
<section
  class="@container relative flex min-w-0 flex-col"
  aria-label={say($lang, "monitor_title")}
>
  {#if shown.length === 0}
    <p class="text-note text-text-faint">{say($lang, "monitor_waiting")}</p>
  {:else}
    <!-- One counter a row, ruled like a table: its name over its band,
         its latest reading set right on the figure column, then its
         bars to the end of the line. Under 40rem the bars take a line
         of their own. -->
    <ul>
      {#each shown as row (row.label)}
        {@const across = Math.max(points, row.plot.bars.length, 1)}
        <li
          class="grid grid-cols-[minmax(0,1fr)_14ch] items-center gap-x-gutter border-b border-edge py-snug @min-[40rem]:grid-cols-[minmax(0,28ch)_14ch_minmax(0,1fr)]"
        >
          <span class="flex min-w-0 flex-col">
            <span class="truncate text-note text-text-quiet">{say($lang, row.label)}</span>
            <span class="figure truncate text-note text-text-faint"
              >{fill(say($lang, "monitor_band"), { p50: row.p50, p99: row.p99 })}</span
            >
          </span>
          <span class="figure text-right text-text">{row.reading}</span>
          <!-- Every plot has the same room, so whichever measures it
               last tells the panel how many points one line holds. -->
          <span class={PLOT} bind:clientWidth={panel}>
            <svg aria-hidden="true" class="block h-full w-full" viewBox="0 0 {String(across)} {String(FULL)}" preserveAspectRatio="none">
              <rect x="0" y={FULL - row.plot.p99} width={across} height={Math.max(row.plot.p99 - row.plot.p50, 1)} class="fill-accent/15" />
              <path d={barsOf(row.plot, across)} class="fill-accent" />
            </svg>
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</section>
</Page>


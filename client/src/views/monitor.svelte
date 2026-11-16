<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  import type { Sample } from "../wire";

  export const BEATS = ["50", "100", "250", "500", "1000"] as const;
  export type BeatChoice = (typeof BEATS)[number];

  export function beatOf(latest: Sample | undefined): BeatChoice | null {
    return BEATS.find((beat) => Number(beat) === latest?.beat_ms) ?? null;
  }

  export interface MonitorProps {
    readonly samples: readonly Sample[];
    readonly watch: () => () => void;

    readonly beat?: ((ms: number) => void) | undefined;

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

  const POINT_PX = 3;

  let panel = $state(0);

  $effect(() => watch());

  const points = $derived(Math.floor(panel / POINT_PX));
  const shown = $derived(rows(samples, points));

  function barsOf(plot: Plot, points: number): string {
    const first = points - plot.bars.length;
    return plot.bars
      .map(
        (height, at) => `M${String(first + at)} ${String(FULL - height)}h0.7V${String(FULL)}h-0.7Z`,
      )
      .join("");
  }

  const held = $derived(beatOf(samples.at(-1)));
  const beats = $derived(
    BEATS.map((value) => ({ value, label: fill(say($lang, "monitor_beat_ms"), { n: value }) })),
  );

  const PLOT =
    "col-span-full block h-plot w-full min-w-0 pt-tight @min-[40rem]:col-span-1 @min-[40rem]:pt-0";
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

            <span class={PLOT} bind:clientWidth={panel}>
              <svg
                aria-hidden="true"
                class="block h-full w-full"
                viewBox="0 0 {String(across)} {String(FULL)}"
                preserveAspectRatio="none"
              >
                <rect
                  x="0"
                  y={FULL - row.plot.p99}
                  width={across}
                  height={Math.max(row.plot.p99 - row.plot.p50, 1)}
                  class="fill-accent/15"
                />
                <path d={barsOf(row.plot, across)} class="fill-accent" />
              </svg>
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</Page>

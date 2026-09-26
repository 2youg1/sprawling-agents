<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The time lens: one lane per share of the run's clock - the model
  // speaking, tools running, the person being waited on - drawn as
  // parallel tracks the way a profiler draws threads, and under them
  // every tool call the run made, in order. `lanes.ts` decides what is
  // drawn; this file only lays it out, so a ten-thousand-call run costs
  // the page one box per column and one row per visible line.

  import { POSTURE_WORD } from "../../core/doing";
  import type { Key } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import { lasted } from "../../core/time";
  import { ui } from "../../ui";
  import type { Turn } from "../../wire";
  import type { Share } from "./lanes";
  import { SHARES, callsOf, columnsOf, stretchesOf, tookOf, windowOf } from "./lanes";

  interface Props {
    readonly turns: readonly Turn[];
    // The run's first and last known moments, in ms.
    readonly from: number;
    readonly to: number;
    // What a live run is doing now; `null` once it is over.
    readonly tail: Share | null;
  }

  const { turns, from, to, tail }: Props = $props();
  const lang = ui().lang;

  const WORD: Record<Share, Key> = {
    model: POSTURE_WORD.thinking,
    tool: POSTURE_WORD.calling,
    person: POSTURE_WORD.waiting,
  };
  const FILL: Record<Share, string> = {
    model: "bg-accent",
    tool: "bg-accent-solid",
    person: "bg-alert",
  };

  // Measured, not assumed: the lane's width decides how many columns
  // it samples, and the drawn rows' height how many rows fit.
  let width = $state(0);
  let row = $state(0);
  let drawn = $state(0);
  let top = $state(0);
  let height = $state(0);

  const whole = $derived(Math.max(1, to - from));
  const stretches = $derived(stretchesOf(turns, to, tail));
  const columns = $derived(Math.max(1, Math.floor(width / 2)));
  const lanes = $derived(columnsOf(stretches, from, to, columns));
  const calls = $derived(callsOf(turns));
  const rows = $derived(windowOf(calls.length, row, top, height));

  // The first window is drawn before any row has been measured; its
  // height divided by its rows is the row height every later window
  // uses.
  $effect(() => {
    const shown = rows.end - rows.first;
    if (shown > 0 && drawn > 0) row = drawn / shown;
  });

  function spentOn(share: Share): number {
    return stretches
      .filter((each) => each.share === share)
      .reduce((sum, each) => sum + each.to - each.from, 0);
  }

  function percent(part: number): string {
    return `${String(Math.round((part / whole) * 100))}%`;
  }
</script>

<section class="flex flex-col gap-base pt-base">
  <div role="img" aria-label={say($lang, "run_river")} class="flex flex-col gap-snug">
    <div class="flex justify-between pl-figure font-mono text-note text-text-quiet">
      <span>{lasted(0)}</span>
      <span>{lasted(whole / 2)}</span>
      <span>{lasted(whole)}</span>
    </div>
    {#each SHARES as share (share)}
      <div class="flex items-center gap-snug">
        <div class="flex w-figure shrink-0 flex-col text-note">
          <span class="truncate text-text">{say($lang, WORD[share])}</span>
          <span class="font-mono text-text-quiet">{percent(spentOn(share))}</span>
        </div>
        <div class="relative h-control-sm flex-1 border-t border-edge" bind:clientWidth={width}>
          {#each lanes.filter((each) => each.share === share) as box (box.first)}
            <span
              class={["absolute top-1/2 h-dot -translate-y-1/2 rounded-pill", FILL[share]]}
              style:left="{(box.first / columns) * 100}%"
              style:width="{((box.end - box.first) / columns) * 100}%"
            ></span>
          {/each}
        </div>
      </div>
    {/each}
  </div>

  <h2 class="text-label font-label text-text-quiet">
    {say($lang, "run_calls")} <span class="font-mono">{calls.length}</span>
  </h2>
  {#if calls.length > 0}
    <div
      class="max-h-tree overflow-auto border-y border-edge"
      bind:clientHeight={height}
      onscroll={(event) => {
        top = event.currentTarget.scrollTop;
      }}
    >
      <div style:height="{rows.first * row}px"></div>
      <ol class="text-note" bind:offsetHeight={drawn}>
        {#each calls.slice(rows.first, rows.end) as placed, at (rows.first + at)}
          {@const took = tookOf(placed.call)}
          <li class="flex h-control-sm items-center gap-snug">
            <span class="w-figure shrink-0 font-mono text-text-disabled"
              >{fill(say($lang, "run_turn_n"), { n: String(placed.turn) })}</span
            >
            <span class={["shrink-0 font-mono", placed.call.outcome === "failed" ? "text-alert" : "text-text"]}
              >{placed.call.tool}</span
            >
            <span class="min-w-0 flex-1 truncate font-mono text-text-quiet">{placed.call.subject ?? ""}</span>
            <span class="shrink-0 font-mono text-text-disabled"
              >{took === null ? "" : lasted(took)}</span
            >
          </li>
        {/each}
      </ol>
      <div style:height="{(calls.length - rows.end) * row}px"></div>
    </div>
  {:else}
    <p class="text-note text-text-faint">{say($lang, "run_no_calls")}</p>
  {/if}
</section>


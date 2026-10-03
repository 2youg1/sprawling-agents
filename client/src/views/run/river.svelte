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
  import { isoInstant, isoTime, lasted } from "../../core/time";
  import { ui } from "../../ui";
  import type { Turn } from "../../wire";
  import type { Share } from "./lanes";
  import { SHARES, callsOf, columnsOf, stretchesOf, windowOf } from "./lanes";
  import { tookOf, tookWords } from "../talk/timing";

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
  // The height every call row takes (`h-control-sm`), which also sizes
  // the spacers standing in for the rows not drawn.
  const ROW_TOKEN = "--spacing-control-sm";
  const FILL: Record<Share, string> = {
    model: "bg-accent",
    tool: "bg-accent-solid",
    person: "bg-alert",
  };

  // One observer reports every size the lens reads, and what it reports
  // lands on the next frame. Written inside the report, a size would
  // redraw the lanes and the rows while the browser is still reporting
  // sizes, and the browser names that a ResizeObserver loop.
  let width = $state(0);
  let height = $state(0);
  let top = $state(0);
  // The row token, read once from the stylesheet that sizes each row:
  // the spacers are counted in the same token, so the list is as long
  // as its calls whichever rows are drawn, and nothing drawn feeds back
  // into the window.
  let row = $state(0);
  let list: Element | null = null;
  let frame = 0;
  const seen = { width: 0, height: 0 };
  const observer = new ResizeObserver((entries) => {
    for (const entry of entries) {
      if (entry.target === list) seen.height = entry.contentRect.height;
      else seen.width = entry.contentRect.width;
    }
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      width = seen.width;
      height = seen.height;
    });
  });
  $effect(() => () => {
    cancelAnimationFrame(frame);
    observer.disconnect();
  });

  function sizesTrack(node: Element): () => void {
    observer.observe(node);
    return () => {
      observer.unobserve(node);
    };
  }

  function sizesList(node: Element): () => void {
    list = node;
    const token = Number.parseFloat(getComputedStyle(node).getPropertyValue(ROW_TOKEN));
    row = Number.isFinite(token) ? token : 0;
    observer.observe(node);
    return () => {
      observer.unobserve(node);
      list = null;
    };
  }

  const whole = $derived(Math.max(1, to - from));
  const stretches = $derived(stretchesOf(turns, to, tail));
  const columns = $derived(Math.max(1, Math.floor(width / 2)));
  const lanes = $derived(columnsOf(stretches, from, to, columns));
  const calls = $derived(callsOf(turns));
  const rows = $derived(windowOf(calls.length, row, top, height));

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
        <div class="relative h-control-sm flex-1 border-t border-edge" {@attach sizesTrack}>
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

  <h2 class="flex justify-between text-note text-text-faint">
    <span>{say($lang, "run_calls")} <span class="figure text-text-quiet">{calls.length}</span></span>
    <span>{say($lang, "run_calls_axis")}</span>
  </h2>
  {#if calls.length > 0}
    <div
      class="max-h-tree overflow-auto border-y border-edge"
      {@attach sizesList}
      onscroll={(event) => {
        top = event.currentTarget.scrollTop;
      }}
    >
      <div style:height="calc(var({ROW_TOKEN}) * {rows.first})"></div>
      <ol class="@container text-note">
        {#each calls.slice(rows.first, rows.end) as placed, at (rows.first + at)}
          {@const took = tookOf(placed.call)}
          {@const landed = placed.call.answered ?? null}
          <li class="grid h-control-sm grid-cols-[14ch_minmax(0,1fr)_10ch] items-center gap-x-base whitespace-nowrap @min-[40rem]:grid-cols-[14ch_9ch_12ch_minmax(0,1fr)_10ch]">
            {#if landed === null}
              <span class="figure text-text-faint">{say($lang, "run_call_running")}</span>
            {:else}
              <time class="figure text-text-faint" datetime={isoInstant(landed)}>{isoTime(landed)}</time>
            {/if}
            <span class="hidden truncate text-text-faint @min-[40rem]:block"
              >{fill(say($lang, "run_turn_n"), { n: String(placed.turn) })}</span
            >
            <span class={["hidden truncate font-mono @min-[40rem]:block", placed.call.outcome === "failed" ? "text-alert" : "text-text"]}
              >{placed.call.tool}</span
            >
            <span class="min-w-0 truncate font-mono text-text-quiet">{placed.call.subject ?? placed.call.tool}</span>
            <span class="figure text-right text-text-faint">{took === null ? "" : tookWords(took, $lang)}</span>
          </li>
        {/each}
      </ol>
      <div style:height="calc(var({ROW_TOKEN}) * {calls.length - rows.end})"></div>
    </div>
  {:else}
    <p class="text-note text-text-faint">{say($lang, "run_no_calls")}</p>
  {/if}
</section>


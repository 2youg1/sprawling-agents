<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The building table the city page opens on: the ledger tree's first
  // level, one row a building, with the counts a person reads the city
  // by - waiting for them, at work, done, the last start - and one time
  // bar per building on the runs board's folded clock. A tick on the bar
  // is a run's start, coloured by what that run is doing now, and the
  // quiet stretch runs from the oldest run still going to now.
  //
  // Where the table is wide a row is one line; where it is narrow the
  // bar takes a line of its own under the counts, so neither the names
  // nor the bar shrink to a sliver. The table asks its own width rather
  // than the page's, because the picked building's panel can stand
  // beside it and take half the page. Picking a row opens the building's panel, as
  // picking its tower in the drawing does.
  import { fill, say } from "../../core/lang";
  import { ago } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, CityAnswer } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import { FOLDS, along } from "../runs/fold";
  import type { BoardRun } from "../runs/lineage";
  import { PHASE_FILL } from "../runs/phase";
  import { tableOf } from "./table";

  interface Props {
    readonly city: CityAnswer;
    readonly runs: readonly BoardRun[];
    readonly now: number;
    readonly picked: Address | null;
    readonly onPick: (addr: Address) => void;
  }

  const { city, runs, now, picked, onPick }: Props = $props();
  const { lang } = ui();

  const rows = $derived(tableOf(city.buildings, runs));

  function nameOf(addr: Address): string {
    return addr === "hall" ? say($lang, "city_hall") : addr;
  }

  function stretch(since: number): { readonly left: string; readonly width: string } {
    const from = along(since, now);
    return { left: `${from.toFixed(3)}%`, width: `${(100 - from).toFixed(3)}%` };
  }
</script>

<section class="@container/table flex w-full min-w-0 flex-col" aria-label={say($lang, "city_table")}>
  <div class="flex min-w-0 flex-wrap items-end gap-x-base border-b border-edge pb-tight font-mono figure text-note text-text-quiet" aria-hidden="true">
    <span class="hidden min-w-0 flex-1 @lg/table:block"></span>
    <span class="relative h-base w-full shrink-0 @lg/table:w-[40%]">
      {#each FOLDS.filter((fold) => fold.minutes > 0) as fold (fold.minutes)}
        <span class="absolute top-0 border-l border-edge-input pl-tight" style:left="{String(fold.at)}%">{fill(say($lang, "runs_minus"), { n: String(fold.minutes) })}</span>
      {/each}
      <span class="absolute top-0 right-0 font-sans text-text">{say($lang, "runs_now")}</span>
    </span>
  </div>
  <ul class="flex min-w-0 flex-col">
    {#each rows as row (row.addr)}
      <li class="border-b border-edge">
        <button
          type="button"
          class={[
            "flex w-full min-w-0 flex-wrap items-center gap-x-base gap-y-tight py-snug pr-snug text-left text-note hover:bg-raised",
            picked === row.addr ? "bg-raised shadow-[inset_2px_0_0_var(--color-accent)]" : "",
          ]}
          aria-pressed={picked === row.addr}
          aria-label={fill(say($lang, "city_table_row"), {
            name: nameOf(row.addr),
            waiting: String(row.waiting),
            working: String(row.working),
            done: String(row.done),
          })}
          onclick={() => {
            onPick(row.addr);
          }}
        >
          <span class="flex min-w-0 flex-1 items-center gap-base">
            <span class="shrink-0 pl-snug font-mono whitespace-pre text-text-quiet" aria-hidden="true">{row.guide}</span>
            <span class="min-w-0 flex-1 truncate font-mono font-label text-text">{nameOf(row.addr)}</span>
            <span class={["inline-flex shrink-0 items-center gap-tight font-mono figure", row.waiting === 0 ? "text-text-quiet" : "text-alert"]}>
              <Glyph name="hand" size="sm" />{String(row.waiting)}
            </span>
            <span class={["inline-flex shrink-0 items-center gap-tight font-mono figure", row.working === 0 ? "text-text-quiet" : "text-accent"]}>
              <Glyph name="pulse" size="sm" />{String(row.working)}
            </span>
            <span class="inline-flex shrink-0 items-center gap-tight font-mono figure text-text-quiet">
              <Glyph name="check" size="sm" />{String(row.done)}
            </span>
            <span class="hidden w-[12ch] shrink-0 truncate text-right figure text-text-quiet @lg/table:block">
              {row.latest === null ? say($lang, "city_table_none") : ago($lang, row.latest, now)}
            </span>
          </span>
          <span class="relative h-snug w-full shrink-0 @lg/table:w-[40%]" aria-hidden="true">
            <span class="absolute inset-0 border-r border-edge-input"></span>
            {#if row.since !== null}
              {@const live = stretch(row.since)}
              <span class="absolute inset-y-0 rounded-pill bg-edge" style:left={live.left} style:width={live.width}></span>
            {/if}
            {#each row.starts as start (start.run)}
              <span
                class={["absolute inset-y-0 w-tight rounded-pill", PHASE_FILL[start.phase]]}
                style:left="clamp(0px, calc({along(start.at, now).toFixed(3)}% - var(--spacing-tight) / 2), calc(100% - var(--spacing-tight)))"
              ></span>
            {/each}
          </span>
        </button>
      </li>
    {/each}
  </ul>
</section>

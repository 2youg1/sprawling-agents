<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city page: one information bar, the drawing, and under it the
  // runs board - every run in the city on the lineage tree, the runs
  // waiting for the person first - which is the city's list: a building
  // is a root of that tree, so a second list of buildings would say the
  // same thing twice. What was picked in the drawing stands beside it
  // when there is room and under it when there is not.
  //
  // The shell fills the viewport; `max-w-page` binds the legend, which
  // is prose, and never the drawing. The legend below the drawing is
  // the second half of the bar's job: a city drawn in glyphs that
  // nothing names is a picture; a legend is a set of labels, which is
  // what makes the picture readable. Its five marks are the drawing's
  // own art rather than icons, so they stay hand-drawn here (4-12).

  import { QUERIES } from "../core/asking";
  import { readAnswer } from "../core/answered";
  import { say } from "../core/lang";
  import { MAYOR, toFragment } from "../core/route";
  import type { Address } from "../wire";
  import { ui } from "../ui";
  import Bar from "./city/bar.svelte";
  import Panel from "./city/panel.svelte";
  import Skyline from "./city/skyline.svelte";
  import EmptyState from "./parts/empty.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import Board from "./runs/board.svelte";
  import { boardRuns } from "./runs/lineage";

  // The five marks the drawing carries, and the order a reader meets
  // them in.
  const MARKS = ["window", "figure", "flag", "lamp", "plinth"] as const;

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const answer = u.conn.asking.ask(QUERIES.city);

  let picked = $state.raw<Address | null>(null);

  const read = $derived(readAnswer($answer, (held) => ("city" in held ? held.city : undefined)));
  const city = $derived(read.kind === "held" ? read.value : undefined);

  // The time is sampled when the run table changes shape or a run
  // changes phase, which is when a bar's right-hand end moves.
  const board = $derived({ runs: boardRuns($belief.runs), now: u.now() });
</script>

<!-- The legend below the drawing: one glyph and the word for it. The
 drawing carries five marks, and a reader who has met none of them
 before is told what each one is. The marks are the drawing's own art
 rather than icons, so they are drawn here and not in `parts/glyph`. -->
<div class="flex min-h-0 flex-1 flex-col">
  <Bar />
  <div class="relative flex min-h-0 flex-1 flex-col @lg/page:flex-row">
    <section class="flex min-h-0 min-w-0 flex-1 flex-col overflow-auto px-pane py-base">
      {#if read.kind === "unavailable"}
        <Unanswered query={read.query} asked={QUERIES.city} />
      {:else if city === undefined}
        <p class="text-center text-text-faint">…</p>
      {:else if city.buildings.length > 0}
        <Skyline
          {city}
          {picked}
          onPick={(addr) => {
            picked = addr;
          }}
        />
        <ul
          class="mx-auto mt-base flex max-w-page flex-wrap items-center justify-center gap-wide text-note text-text-faint"
        >
          {#each MARKS as mark (mark)}
            <li class="flex items-center gap-tight">
              <svg viewBox="0 0 16 16" class="size-glyph shrink-0" aria-hidden="true">
                {#if mark === "window"}
                  <rect x="5" y="3" width="6" height="8" rx="1" class="fill-accent-solid" />
                {:else if mark === "figure"}
                  <circle cx="8" cy="5" r="2.4" class="fill-drawn-figure" />
                  <path d="M4.6 13 q3.4 -6 6.8 0 z" class="fill-drawn-figure" />
                {:else if mark === "flag"}
                  <line x1="5" y1="2" x2="5" y2="14" class="stroke-drawn-stem" stroke-width="1.2" />
                  <path d="M5 3 l7 1.8 l-7 2.2 z" class="fill-accent" />
                {:else if mark === "lamp"}
                  <line x1="8" y1="6" x2="8" y2="14" class="stroke-drawn-part" stroke-width="1.2" />
                  <circle cx="8" cy="4.4" r="2.6" class="fill-alert" />
                {:else if mark === "plinth"}
                  <rect x="1" y="6" width="14" height="4" rx="2" class="fill-drawn-line" />
                  <rect x="1" y="6" width="8" height="4" rx="2" class="fill-accent" />
                {/if}
              </svg>
              <span>{say($lang, `legend_${mark}`)}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <!-- A city with no buildings is a city nobody has asked for
        anything yet, and the Mayor is where a person asks: raising a
        building is a sentence in that conversation rather than a button
        this page could press. -->
        <EmptyState missing="city_no_buildings">
          {#snippet action()}
            <a
              href={toFragment({ kind: "talk", address: MAYOR })}
              class="inline-flex h-control items-center rounded-control bg-accent px-base text-label text-on-accent hover:bg-accent-hover"
            >
              {say($lang, "city_ask_mayor")}
            </a>
          {/snippet}
        </EmptyState>
      {/if}
      {#if board.runs.length > 0}
        <div class="mt-wide">
          <Board runs={board.runs} now={board.now} level={2} />
        </div>
      {/if}
    </section>
    {#if picked !== null && city !== undefined}
      <aside
        class="w-full shrink-0 overflow-y-auto border-t border-edge px-pane py-base @lg/page:absolute @lg/page:inset-y-0 @lg/page:right-0 @lg/page:z-10 @lg/page:w-tree @lg/page:border-t-0 @lg/page:border-l @lg/page:bg-page @lg/page:shadow-sheet @wide/page:static @wide/page:shadow-none"
        aria-label={say($lang, "city_panel")}
      >
        <Panel
          addr={picked}
          {city}
          onClose={() => {
            picked = null;
          }}
        />
      </aside>
    {/if}
  </div>
</div>

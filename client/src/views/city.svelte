<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city page: one information bar, the building table or the
  // drawing, and under them the runs board - every run in the city on
  // the lineage tree, the runs waiting for the person first. The table
  // opens first because it reads at any width, where the drawing at a
  // phone's width is a band too thin to read; the drawing is the second
  // view, one switch away. What was picked in the table or the drawing
  // stands beside it when there is room and under it when there is not.
  // In results mode all of it gives way to the runs' outcomes alone.
  //
  // The shell fills the viewport; `max-w-page` binds the legend, which
  // is prose, and never the drawing. The legend is folded behind one
  // control under the drawing: a city drawn in glyphs that nothing
  // names is a picture, and the legend is the set of labels that makes
  // it readable, but open it takes half of a narrow screen. Its five
  // marks are the drawing's own art rather than icons, so they stay
  // hand-drawn here (4-12).

  import { QUERIES } from "../core/asking";
  import { readAnswer } from "../core/answered";
  import { say } from "../core/lang";
  import { MAYOR, toFragment } from "../core/route";
  import type { Address } from "../wire";
  import { ui } from "../ui";
  import Bar from "./city/bar.svelte";
  import Panel from "./city/panel.svelte";
  import Results from "./city/results.svelte";
  import Skyline from "./city/skyline.svelte";
  import Table from "./city/table.svelte";
  import EmptyState from "./parts/empty.svelte";
  import Segmented from "./parts/segmented.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import Board from "./runs/board.svelte";
  import { boardRuns } from "./runs/lineage";
  import Showing from "./shared/showing.svelte";

  // The five marks the drawing carries, and the order a reader meets
  // them in.
  const MARKS = ["window", "figure", "flag", "lamp", "plinth"] as const;

  // The two ways the page draws the city's buildings, the first the one
  // it opens on.
  const VIEWS = ["table", "drawing"] as const;

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const answer = u.conn.asking.ask(QUERIES.city);
  const held = u.prefs.held;

  let picked = $state.raw<Address | null>(null);
  let view = $state<(typeof VIEWS)[number]>("table");

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
      <div class="flex flex-wrap justify-end gap-base pb-snug">
        {#if $held.showing !== "results"}
          <Segmented
            label={say($lang, "city_view")}
            options={VIEWS.map((value) => ({ value, label: say($lang, `city_view_${value}`) }))}
            held={view}
            onPick={(next: (typeof VIEWS)[number]) => {
              view = next;
            }}
          />
        {/if}
        <Showing />
      </div>
      {#if $held.showing === "results"}
        <Results />
      {:else if read.kind === "unavailable"}
        <Unanswered query={read.query} asked={QUERIES.city} />
      {:else if city === undefined}
        <p class="text-center text-text-faint">…</p>
      {:else if city.buildings.length > 0 && view === "table"}
        <Table
          {city}
          runs={board.runs}
          now={board.now}
          {picked}
          onPick={(addr) => {
            picked = addr;
          }}
        />
      {:else if city.buildings.length > 0}
        <Skyline
          {city}
          {picked}
          onPick={(addr) => {
            picked = addr;
          }}
        />
        <details class="mx-auto mt-base max-w-page text-note text-text-faint">
          <summary class="cursor-pointer text-center text-text-quiet">{say($lang, "city_legend")}</summary>
          <ul class="mt-snug flex flex-wrap items-center justify-center gap-wide">
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
        </details>
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
      {#if $held.showing !== "results" && board.runs.length > 0}
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

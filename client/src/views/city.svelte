<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city page, on the shell's column lines (client-SPEC 4-50): the
  // page header names the city and carries the one control that stops
  // or releases it; under it one row of figures; then the building
  // table or the drawing in columns 1-8 of the page, the picked
  // building beside it in 9-11, and under both the runs board - every
  // run in the city on the lineage tree, the runs waiting for the person
  // first. The table opens first because it reads at any width, where
  // the drawing at a phone's width is a band too thin to read. In
  // results mode all of it gives way to the runs' outcomes alone.
  //
  // The legend is folded behind one control under the drawing: a city
  // drawn in glyphs that nothing names is a picture, and the legend is
  // the set of labels that makes it readable. Its five marks are the
  // drawing's own art rather than icons, so they stay hand-drawn here
  // (4-12).

  import { QUERIES } from "../core/asking";
  import { readAnswer } from "../core/answered";
  import { say } from "../core/lang";
  import { halt, release } from "../core/commands";
  import { MAYOR, toFragment } from "../core/route";
  import { cityIsShut, CITY } from "../core/scope";
  import type { Address } from "../wire";
  import { ui } from "../ui";
  import Bar from "./city/bar.svelte";
  import Panel from "./city/panel.svelte";
  import Results from "./city/results.svelte";
  import Skyline from "./city/skyline.svelte";
  import Table from "./city/table.svelte";
  import Button from "./parts/button.svelte";
  import EmptyState from "./parts/empty.svelte";
  import Page from "./parts/page.svelte";
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

  const halted = $derived(cityIsShut($belief.halted));
  const pickedOpen = $derived(picked !== null && city !== undefined);
</script>

{#snippet aside()}
  <Button
    label={halted ? say($lang, "city_release") : say($lang, "city_stop")}
    tone={halted ? "secondary" : "quiet"}
    onPress={() => {
      u.send(halted ? release(CITY) : halt(CITY));
    }}
  />
{/snippet}

<Page title={$belief.city ?? say($lang, "nav_city")} {aside}>
  <Bar />
  <div class="grid grid-cols-11 items-start gap-x-gutter gap-y-wide narrow:grid-cols-1">
    <section
      class={["flex min-w-0 flex-col gap-base narrow:col-span-full", pickedOpen ? "col-[1/9]" : "col-[1/12]"]}
      aria-label={say($lang, "city_buildings")}
    >
      <div class="flex flex-wrap items-center justify-between gap-base">
        <h2 class="text-note text-text-faint">{say($lang, "city_buildings")}</h2>
        <div class="flex flex-wrap items-center gap-base">
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
      </div>
      {#if $held.showing === "results"}
        <Results />
      {:else if read.kind === "unavailable"}
        <Unanswered query={read.query} asked={QUERIES.city} />
      {:else if city === undefined}
        <p class="text-text-faint">…</p>
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
        <details class="text-note text-text-faint">
          <summary class="cursor-pointer text-text-quiet">{say($lang, "city_legend")}</summary>
          <ul class="mt-snug flex flex-wrap items-center gap-wide">
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
    </section>
    {#if picked !== null && city !== undefined}
      <aside class="col-[9/12] flex min-w-0 flex-col narrow:col-span-full" aria-label={say($lang, "city_panel")}>
        <Panel
          addr={picked}
          {city}
          onClose={() => {
            picked = null;
          }}
        />
      </aside>
    {/if}
    {#if $held.showing !== "results" && board.runs.length > 0}
      <div class="col-[1/12] min-w-0 narrow:col-span-full">
        <Board runs={board.runs} now={board.now} level={2} />
      </div>
    {/if}
  </div>
</Page>

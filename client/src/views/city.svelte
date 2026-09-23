<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city page: one information bar, then as many columns as the
  // screen affords. The list of buildings arrives at the widest step,
  // the drawing is always there, and what was picked stands beside the
  // drawing when there is room and under it when there is not.
  //
  // The shell fills the viewport; `max-w-page` binds the legend, which
  // is prose, and never the drawing. The legend below the drawing is
  // the second half of the bar's job: a city drawn in glyphs that
  // nothing names is a picture; a legend is a set of labels, which is
  // what makes the picture readable. Its five marks are the drawing's
  // own art rather than icons, so they stay hand-drawn here (4-12).

  import { QUERIES } from "../core/asking";
  import { fill, say } from "../core/lang";
  import { MAYOR, toFragment } from "../core/route";
  import { percent } from "../core/share";
  import type { Address, BuildingProgress, CityAnswer } from "../wire";
  import { ui } from "../ui";
  import Bar from "./city/bar.svelte";
  import Panel from "./city/panel.svelte";
  import Skyline from "./city/skyline.svelte";
  import EmptyState from "./parts/empty.svelte";

  // The five marks the drawing carries, and the order a reader meets
  // them in.
  const MARKS = ["window", "figure", "flag", "lamp", "plinth"] as const;

  interface Row {
    readonly addr: Address;
    // How far the plan has got, already in the person's language.
    // `null` for a building with no plan, which has no share to show.
    readonly done: string | null;
  }

  const u = ui();
  const { lang } = u;
  const answer = u.conn.asking.ask(QUERIES.city);

  let picked = $state.raw<Address | null>(null);

  const city = $derived.by((): CityAnswer | undefined => {
    const held = $answer;
    return held !== undefined && "city" in held ? held.city : undefined;
  });

  const rows = $derived.by((): Row[] =>
    [...(city?.buildings ?? [])]
      .sort((a, b) => a.addr.localeCompare(b.addr))
      .map((building) => ({ addr: building.addr, done: share(building) })),
  );

  function share(building: BuildingProgress): string | null {
    if (!("planned" in building.progress)) return null;
    const done = String(percent(building.progress.planned.done_ppb));
    return fill(say($lang, "city_done_percent"), { percent: done });
  }
</script>

<!-- The legend below the drawing: one glyph and the word for it. The
 drawing carries five marks, and a reader who has met none of them
 before is told what each one is. The marks are the drawing's own art
 rather than icons, so they are drawn here and not in `parts/glyph`. -->
<div class="flex min-h-0 flex-1 flex-col">
  <Bar />
  <div class="relative flex min-h-0 flex-1 flex-col @lg/page:flex-row">
    <nav
      class="hidden shrink-0 overflow-y-auto border-r border-edge px-snug py-base @wide/page:block @wide/page:w-rail-open"
      aria-label={say($lang, "city_buildings")}
    >
      {#each rows as row (row.addr)}
        <button
          type="button"
          class={[
            "flex h-control-sm w-full items-center gap-snug rounded-control px-snug text-left text-note leading-none",
            picked === row.addr ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome",
          ]}
          aria-current={picked === row.addr ? "true" : undefined}
          onclick={() => {
            picked = row.addr;
          }}
        >
          <span class="min-w-0 flex-1 truncate font-mono">{row.addr}</span>
          {#if row.done !== null}
            <span class="shrink-0 font-mono text-text-disabled">{row.done}</span>
          {/if}
        </button>
      {/each}
    </nav>
    <section class="flex min-h-0 min-w-0 flex-1 flex-col justify-center overflow-auto px-pane py-base">
      {#if city === undefined}
        <p class="text-center text-text-disabled">…</p>
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

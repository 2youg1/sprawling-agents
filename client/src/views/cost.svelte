<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Cost in five cuts of one authoritative total. Shares are drawn
// against `total`, never against the sum of the rows, so an
// unattributed remainder stays visible. A provider that reported no
// price leaves the total at zero, and the page says so rather than
// printing $0.00 as if it were a measurement.
//
// **The total and the by-run cut are two questions and stay apart**
// (client-SPEC 7D): one is what this city has spent, the other is who
// spent it. The figure beside the title is the total and never a sum of
// the rows below it.
//
// The bar under a row is that row's summary: it restates the figure
// beside the name, so a compact page draws the name and the figure and
// stops there. Which pages are compact is `theme.css`'s one density
// attribute, not a switch of this page's own.

import type { Key } from "../core/lang";
import type { UsdMicros } from "../wire";

type Cut = "by_run" | "by_actor" | "by_segment" | "by_tool" | "by_skill";
const CUTS: readonly Cut[] = ["by_run", "by_actor", "by_segment", "by_tool", "by_skill"];

// Each cut's heading, one phrase-table key per cut. A table rather than
// a spelled-out name so a key that `lang.json` drops fails to compile
// instead of failing to render.
const TITLES: Record<Cut, Key> = {
  by_run: "cost_by_run",
  by_actor: "cost_by_actor",
  by_segment: "cost_by_segment",
  by_tool: "cost_by_tool",
  by_skill: "cost_by_skill",
};
</script>

<script lang="ts">
  import { QUERIES } from "../core/asking";
  import { MAYOR, toFragment } from "../core/route";
  import { say } from "../core/lang";
  import { usd } from "../core/time";
  import { ui } from "../ui";
  import type { CostAnswer } from "../wire";
  import EmptyState from "./parts/empty.svelte";
  import { costReading } from "./cost";

  const u = ui();
  const lang = u.lang;
  const asked = u.conn.asking.ask(QUERIES.cost);

  const answer = $derived.by((): CostAnswer | undefined => {
    const held = $asked;
    return held !== undefined && "cost" in held ? held.cost : undefined;
  });
</script>

{#snippet cut(rows: readonly (readonly [string, UsdMicros])[], total: UsdMicros)}
  <ul class="text-note">
    {#each [...rows].sort((a, b) => b[1] - a[1]) as [name, amount] (name)}
      <li class="settled-row my-tight">
        <div class="flex justify-between gap-base">
          <span class="truncate font-mono text-text-quiet">{name}</span>
          <span class="shrink-0 text-text">{usd(amount)}</span>
        </div>
        <div class="summary mt-tight h-dot overflow-hidden rounded-pill bg-track">
          <div class="h-full bg-accent" style:width="{total > 0 ? (amount / total) * 100 : 0}%"></div>
        </div>
      </li>
    {/each}
  </ul>
{/snippet}

<div class="w-full max-w-page px-pane py-wide">
  <div class="mb-wide flex items-baseline justify-between">
    <h1 class="text-title font-title" tabindex="-1">{say($lang, "cost_title")}</h1>
    {#if answer !== undefined}
      <span class="text-figure font-figure">
        {answer.total > 0 ? usd(answer.total) : say($lang, "cost_none")}
      </span>
    {/if}
  </div>
  {#if answer === undefined}
    <p class="text-text-disabled">…</p>
  {:else if costReading(answer).kind === "idle"}
    <!-- Nothing has been spent, which reads exactly like a page that
    failed to load unless the page says which one it is. Spending starts
    with a run, and a run starts in the conversation with the Mayor. -->
    <EmptyState missing="cost_empty">
      {#snippet action()}
        <a
          href={toFragment({ kind: "talk", address: MAYOR })}
          class="rounded-control bg-accent px-base py-snug text-label text-on-accent hover:bg-accent-hover"
        >
          {say($lang, "city_ask_mayor")}
        </a>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="grid gap-wide grid-cols-[repeat(auto-fit,minmax(320px,1fr))]">
      {#each CUTS as each (each)}
        <section>
          <h2 class="mb-snug text-label font-label text-text-quiet">{say($lang, TITLES[each])}</h2>
          {#if answer[each].length > 0}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render cut(answer[each], answer.total)}
          {:else}
            <p class="text-note text-text-disabled">—</p>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>

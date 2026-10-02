<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Cost in five cuts of one authoritative total. Shares are drawn
// against `total`, never against the sum of the rows, so an
// unattributed remainder stays visible. A provider that reported no
// price leaves the total at zero; the server counts those calls and
// their tokens, and the page states that count rather than printing
// $0.00 as if it were a measurement or calling the city idle.
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
import type { UnpricedCalls, UsdMicros } from "../wire";

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
  import { readAnswer } from "../core/answered";
  import { MAYOR, toFragment } from "../core/route";
  import { fill, say } from "../core/lang";
  import { usd } from "../core/time";
  import { ui } from "../ui";
  import EmptyState from "./parts/empty.svelte";
  import Page from "./parts/page.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import { costReading } from "./pricing";

  interface Props {
    // Whether this is the page or a fixture inside one: a document may
    // have exactly one heading of the page's own rank.
    readonly rank?: "page" | "section" | undefined;
  }

  const { rank = "page" }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const asked = u.conn.asking.ask(QUERIES.cost);

  const read = $derived(readAnswer($asked, (held) => ("cost" in held ? held.cost : undefined)));
  const answer = $derived(read.kind === "held" ? read.value : undefined);
  const reading = $derived(answer === undefined ? undefined : costReading(answer));
</script>

{#snippet unpriced(count: UnpricedCalls)}
  <p class="text-note text-text-quiet">
    {fill(say($lang, "cost_unpriced"), { calls: String(count.calls), tokens: String(count.tokens) })}
  </p>
{/snippet}

{#snippet cut(rows: readonly (readonly [string, UsdMicros])[], total: UsdMicros)}
  <ul class="text-note">
    {#each [...rows].sort((a, b) => b[1] - a[1]) as [name, amount] (name)}
      <li class="settled-row py-tight">
        <div class="flex justify-between gap-base">
          <span class="truncate font-mono text-text-quiet">{name}</span>
          <span class="shrink-0 figure text-text">{usd(amount)}</span>
        </div>
        <div class="summary mt-tight h-hair overflow-hidden rounded-pill bg-track">
          <div class="h-full bg-accent" style:width="{total > 0 ? (amount / total) * 100 : 0}%"></div>
        </div>
      </li>
    {/each}
  </ul>
{/snippet}

{#snippet total()}
  <!-- An idle city states no figure at all: the empty state under the
  header already says nothing was spent, and a total of zero beside it
  would read as a measurement. -->
  {#if answer !== undefined && reading !== undefined && reading.kind !== "idle"}
    <span class="flex items-baseline gap-snug">
      <span class="text-note text-text-faint">{say($lang, "cost_total")}</span>
      <span class="text-figure font-figure figure">
        {reading.kind === "priced" ? usd(answer.total) : say($lang, "cost_none")}
      </span>
    </span>
  {/if}
{/snippet}

<!-- The total stands at the right end of the header line, the one place
a figure about the whole page goes; the cuts under it take the page's
width in as many columns as it holds. -->
<Page title={say($lang, "cost_title")} {rank} aside={total}>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={QUERIES.cost} />
  {:else if answer === undefined || reading === undefined}
    <p class="text-text-faint">…</p>
  {:else if reading.kind === "unpriced"}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render unpriced(answer.unpriced)}
  {:else if reading.kind === "idle"}
    <!-- Nothing has been spent, which reads exactly like a page that
    failed to load unless the page says which one it is. Spending starts
    with a run, and a run starts in the conversation with the Mayor. -->
    <EmptyState missing="cost_empty" seat="region">
      {#snippet action()}
        <a
          href={toFragment({ kind: "talk", address: MAYOR })}
          class="inline-flex h-control items-center rounded-control bg-accent px-base text-label text-on-accent hover:bg-accent-hover"
        >
          {say($lang, "city_ask_mayor")}
        </a>
      {/snippet}
    </EmptyState>
  {:else}
    {#if answer.unpriced.calls > 0}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render unpriced(answer.unpriced)}
    {/if}
    <!-- The cuts are columns of the page itself, parted by a rule rather
    than lifted onto cards (client-SPEC 7A-4, 4-50). -->
    <div class="grid grid-fit gap-x-gutter gap-y-wide">
      {#each CUTS as each (each)}
        <section class="flex min-w-0 flex-col gap-snug border-t border-edge pt-base" aria-label={say($lang, TITLES[each])}>
          <h2 class="text-note text-text-faint">{say($lang, TITLES[each])}</h2>
          {#if answer[each].length > 0}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render cut(answer[each], answer.total)}
          {:else}
            <p class="text-note text-text-faint">—</p>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</Page>

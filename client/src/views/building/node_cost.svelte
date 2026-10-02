<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // What one plan node has cost, and which runs spent it
  // (`Query::CostOf`, client-SPEC 4-50). The city keys the answer by
  // the node's number alone and sums every building's node of that
  // number, so the page draws the runs it can place inside this
  // building, adds those, and counts the rest in one line rather than
  // passing another building's money off as this node's.
  import { readAnswer } from "../../core/answered";
  import { within } from "../../core/belief/live";
  import { fill, say } from "../../core/lang";
  import { roomOf, toFragment } from "../../core/route";
  import { usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, NodeId, Query } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";

  interface Props {
    readonly building: Address;
    readonly node: NodeId;
  }

  const { building, node }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  const question = $derived<Query>({ cost_of: { node } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (held) => ("cost_of" in held ? held.cost_of : undefined)));

  const placed = $derived.by(() => {
    if (read.kind !== "held") return null;
    const here = read.value.runs.flatMap(([run, spent]) => {
      const known = $belief.runs[run];
      return known !== undefined && within(known, building) ? [{ run, spent, room: known.addr ?? building }] : [];
    });
    return {
      here,
      spent: here.reduce((sum, row) => sum + row.spent, 0),
      elsewhere: read.value.runs.length - here.length,
    };
  });

  // How long a run id is when a line names a run rather than lists it.
  const RUN_SHORT = 8;
</script>

{#if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={question} />
{:else if placed === null}
  <p class="text-note text-text-faint">…</p>
{:else}
  <div class="flex min-w-0 flex-col gap-tight text-note">
    <p class="flex items-baseline gap-base">
      <span class="text-text-faint">{say($lang, "plan_node_spent")}</span>
      <span class="figure text-text">{usd(placed.spent)}</span>
    </p>
    {#if placed.here.length > 0}
      <ul>
        {#each placed.here as row (row.run)}
          <li class="grid grid-cols-[10ch_minmax(0,1fr)_auto] items-center gap-x-base">
            <a href={toFragment({ kind: "run", run: row.run })} class="figure text-text-quiet hover:text-accent"
              >{row.run.slice(0, RUN_SHORT)}</a
            >
            <span class="truncate font-mono text-text-faint">{roomOf(row.room)}</span>
            <span class="figure text-text-quiet">{usd(row.spent)}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-text-faint">{say($lang, "plan_node_unclaimed")}</p>
    {/if}
    {#if placed.elsewhere > 0}
      <p class="text-text-faint">{fill(say($lang, "plan_node_elsewhere"), { n: String(placed.elsewhere) })}</p>
    {/if}
  </div>
{/if}

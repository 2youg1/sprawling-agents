<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Key } from "../../core/lang";
  import { heldIn } from "../../core/belief/rooms";
  import { roomOf } from "../../core/route";
  import type { Address, Entry } from "../../wire";

  const GOVERNANCE = ".sprawling";

  type Kind = "room" | "governance" | "ordinary";

  const KIND_WORD: Record<Kind, Key> = {
    room: "dir_room",
    governance: "dir_governance",
    ordinary: "dir_ordinary",
  };

  function kindOf(at: Address, entries: readonly Entry[]): Kind {
    if (roomOf(at) === GOVERNANCE) return "governance";
    const held = entries.some((entry) => entry.name === "JOB.md" || entry.name.endsWith(".jsonl"));
    return held ? "room" : "ordinary";
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import type { Doing } from "../../core/doing";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock, kib, usd } from "../../core/time";
  import { readable } from "svelte/store";
  import { ui } from "../../ui";
  import { Address as AddressSchema, type Query } from "../../wire";
  import EmptyState from "../parts/empty.svelte";
  import Path from "../parts/path.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import FactLink from "./fact_link.look.svelte";
  import RunRow from "./run_row.look.svelte";
  import TalkLink from "./talk_link.look.svelte";
  import type { Picked } from "./tree.svelte";
  import { transcriptOf } from "./transcript";

  interface Props {
    readonly at: Address;
    readonly root: Address;
    readonly onPick: (picked: Picked) => void;
  }

  const { at, root, onPick }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  const question = $derived<Query>({ listing: { at } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(
    readAnswer($asked, (held) => ("listing" in held ? held.listing.entries : undefined)),
  );
  const entries = $derived(read.kind === "held" ? read.value : undefined);

  const cost = u.conn.asking.ask(QUERIES.cost);

  const runs = $derived(heldIn($belief, at).reverse());

  const earlier = $derived.by(() => {
    return (entries ?? []).flatMap((entry) => {
      const run = transcriptOf(entry.name);
      return run === null || runs.some((held) => held.run === run) ? [] : [run];
    });
  });

  const cold = $derived.by(() => {
    const held = $cost;
    if (held === undefined || !("cost" in held)) return [];
    const warm = new Set(held.cost.by_run.map(([name]) => name));
    return runs.map((run) => run.run).filter((run) => !warm.has(run));
  });
  const coldCost = $derived(
    cold.length === 0 ? readable(undefined) : u.conn.asking.ask({ run_costs: { runs: cold } }),
  );

  function spent(run: string): number | null {
    const held = $cost;
    if (held === undefined || !("cost" in held)) return null;
    const warm = held.cost.by_run.find(([name]) => name === run)?.[1];
    if (warm !== undefined) return warm;
    const asked = $coldCost;
    if (asked === undefined || !("run_costs" in asked)) return null;
    return asked.run_costs.runs.find(([name]) => name === run)?.[1] ?? null;
  }

  function posture(doing: Doing): string {
    switch (doing.kind) {
      case "thinking":
        return say($lang, "run_doing_thinking");
      case "calling":
        return say($lang, "run_doing_calling");
      case "waiting":
        return say($lang, "talk_waiting_you");
      case "awaiting_reply":
        return say($lang, "run_doing_awaiting_reply");

      case "unknown":
        return say($lang, "city_at_work");
      case "frozen":
        switch (doing.completion) {
          case "done":
            return say($lang, "outcome_done");
          case "limit":
            return say($lang, "outcome_limit");
          case "cancelled":
            return say($lang, "outcome_cancelled");
          case null:
          default:
            return say($lang, "outcome_frozen");
        }
    }
  }
</script>

{#if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={question} />
{:else if entries === undefined}
  <p class="text-text-faint">…</p>
{:else}
  {@const kind = kindOf(at, entries)}
  <div>
    <div class="mb-base flex flex-wrap items-baseline gap-base">
      <h2 class="text-heading font-heading">{roomOf(at)}</h2>
      <span class="text-note text-text-faint">{say($lang, KIND_WORD[kind])}</span>
      <span class="flex-1"></span>
      {#if kind === "room"}
        <TalkLink
          label={say($lang, "bld_open_talk")}
          wire={{ href: toFragment({ kind: "talk", address: at }) }}
        />
      {/if}
    </div>
    {#if kind === "governance"}
      <p class="mb-base text-note text-text-quiet">{say($lang, "dir_governance_what")}</p>
    {/if}
    {#if kind === "room"}
      {#if runs.length > 0}
        <ul class="text-note">
          {#each runs as run (run.run)}
            {@const micros = spent(run.run)}
            <li class="border-b border-edge">
              <RunRow
                task={run.task ?? run.run}
                live={run.doing.kind !== "frozen"}
                posture={posture(run.doing)}
                spent={micros !== null && micros !== 0 ? usd(micros) : undefined}
                started={run.started ? clock($lang, run.started) : undefined}
                wire={{ href: toFragment({ kind: "run", run: run.run }) }}
              />
            </li>
          {/each}
        </ul>
      {:else if earlier.length === 0}
        <EmptyState missing="dir_room_empty">
          {#snippet action()}
            <TalkLink
              label={say($lang, "bld_open_talk")}
              wire={{ href: toFragment({ kind: "talk", address: at }) }}
            />
          {/snippet}
        </EmptyState>
      {/if}
      {#if earlier.length > 0}
        <p class="mt-base text-note text-text-faint">
          {fill(say($lang, "dir_earlier_runs"), { n: String(earlier.length) })}
        </p>
        <ul class="flex flex-wrap gap-x-base text-note">
          {#each earlier as run (run)}
            <li>
              <FactLink
                label={run.slice(0, 8)}
                face="figure"
                ink="quiet"
                wire={{ href: toFragment({ kind: "run", run }) }}
              />
            </li>
          {/each}
        </ul>
      {/if}
    {:else if entries.length > 0}
      <ul class="text-note">
        {#each entries as entry (entry.name)}
          {@const here = AddressSchema.make(`${at}/${entry.name}`)}
          <li class="flex items-center gap-base border-b border-edge py-snug">
            <Path
              path={here}
              base={root}
              onOpen={() => {
                onPick({ at: here, kind: entry.kind === "directory" ? "directory" : "file" });
              }}
            />
            <span class="flex-1"></span>
            {#if entry.kind !== "directory"}
              <span class="shrink-0 font-mono text-text-faint">{kib(entry.kind.file.bytes)}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <EmptyState missing="dir_empty" />
    {/if}
  </div>
{/if}

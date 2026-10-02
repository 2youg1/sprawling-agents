<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A directory is one of three things, and the page says which. A
  // **room** has held work, so it is read as the runs that worked there.
  // The **governance** directory holds the rules a building runs under,
  // so it is read as those documents. Anything else is a directory, read
  // as its files. Treating all three as rooms is what put "no runs"
  // under every folder in the tree.
  //
  // The kind is decided from the listing itself rather than from a name
  // the city does not promise: a room is where a job was written down or
  // a transcript was left.
  import type { Key } from "../../core/lang";
  import { heldIn } from "../../core/belief/rooms";
  import { roomOf } from "../../core/route";
  import type { Address, Entry } from "../../wire";

  // The directory a building keeps its own rules in.
  const GOVERNANCE = ".sprawling";

  type Kind = "room" | "governance" | "ordinary";

  // What each kind is called, one table: `Kind` is closed, so a new
  // kind cannot be drawn without a word for it.
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
  const read = $derived(readAnswer($asked, (held) => ("listing" in held ? held.listing.entries : undefined)));
  const entries = $derived(read.kind === "held" ? read.value : undefined);

  const cost = u.conn.asking.ask(QUERIES.cost);

  const runs = $derived(heldIn($belief, at).reverse());

  // The runs this room's listing has a transcript for and the page holds
  // no run for: older than the hot view, so the page names them and
  // their number rather than calling the room empty (client/Spec.lean §4-50).
  const earlier = $derived.by(() => {
    return (entries ?? []).flatMap((entry) => {
      const run = transcriptOf(entry.name);
      return run === null || runs.some((held) => held.run === run) ? [] : [run];
    });
  });

  // The cost view names the active runs and the few billed most; the
  // rest of this building's runs are asked for by name, newest first,
  // in one question, and the server answers only the newest
  // RUN_COSTS_MAX of them; older cold runs show no figure.
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
      // A phase this page was never told: the run is working and the
      // page cannot say at what, which is what the one word states.
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
  {const kind = kindOf(at, entries)}
  <div>
    <div class="mb-base flex flex-wrap items-baseline gap-base">
      <h2 class="text-heading font-heading">{roomOf(at)}</h2>
      <span class="text-note text-text-faint">{say($lang, KIND_WORD[kind])}</span>
      <span class="flex-1"></span>
      {#if kind === "room"}
        <a
          href={toFragment({ kind: "talk", address: at })}
          class="rounded-control bg-raised px-base py-tight text-label hover:bg-raised-hover"
        >
          {say($lang, "bld_open_talk")}
        </a>
      {/if}
    </div>
    {#if kind === "governance"}
      <p class="mb-base text-note text-text-quiet">{say($lang, "dir_governance_what")}</p>
    {/if}
    {#if kind === "room"}
      {#if runs.length > 0}
        <ul class="text-note">
          {#each runs as run (run.run)}
            {const micros = spent(run.run)}
            <li class="border-b border-edge py-snug">
              <a
                href={toFragment({ kind: "run", run: run.run })}
                class="flex items-center gap-base hover:text-text"
              >
                <span
                  class={[
                    "inline-block size-dot shrink-0 rounded-pill",
                    run.doing.kind === "frozen" ? "bg-mark" : "bg-accent",
                  ]}
                ></span>
                <span class="min-w-0 flex-1 truncate text-text-quiet">{run.task ?? run.run}</span>
                <span class="shrink-0 text-text-faint">{posture(run.doing)}</span>
                {#if micros !== null && micros !== 0}
                  <span class="shrink-0 text-text-faint">{usd(micros)}</span>
                {/if}
                {#if run.started}
                  <span class="shrink-0 text-text-faint">{clock($lang, run.started)}</span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
      {:else if earlier.length === 0}
        <EmptyState missing="dir_room_empty">
          {#snippet action()}
            <a
              href={toFragment({ kind: "talk", address: at })}
              class="rounded-control bg-raised px-base py-tight text-label hover:bg-raised-hover"
            >
              {say($lang, "bld_open_talk")}
            </a>
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
              <a href={toFragment({ kind: "run", run })} class="figure text-text-quiet hover:text-accent"
                >{run.slice(0, 8)}</a
              >
            </li>
          {/each}
        </ul>
      {/if}
    {:else if entries.length > 0}
      <ul class="text-note">
        {#each entries as entry (entry.name)}
          {const here = AddressSchema.make(`${at}/${entry.name}`)}
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

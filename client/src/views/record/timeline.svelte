<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The record's diagnostics as one timeline (refrain S7.6 6c): what the
  // ledger wrote and what the process logged, in one column on one axis,
  // the ledger position. `timeline.ts` decides what stands in the column;
  // `entry` draws each row the way the panorama's timeline is drawn - the
  // day once at the head, then a row per entry: the time of day to the
  // millisecond in UTC, the ledger position, what happened, and where.
  // This file places the choices above the column and the column itself.
  //
  // **The process log is a source, not a lens.** It used to be a tab of
  // its own, which put a line and the record it explains on two screens;
  // now it is one of three sources, and `#/record/log` still opens the
  // timeline with only the log showing. Choosing a source moves the
  // address bar the same way, so a link to what the machine was writing
  // is still a link.

  import type { Key } from "../../core/lang";
  import type { LogLine } from "../../wire";
  import type { Source } from "./timeline";

  const SOURCE_NAMES: Record<Source, Key> = {
    every: "rec_source_every",
    ledger: "rec_ledger",
    log: "rec_log",
  };

  // Every module that has spoken, sorted: the modules worth offering as
  // a filter are the ones that wrote lines.
  function modulesIn(lines: readonly LogLine[]): readonly string[] {
    return [...new Set(lines.map((line) => line.module))].sort();
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { LogLevel, Query, RunId, Seq } from "../../wire";
  import Button from "../parts/button.svelte";
  import EmptyState from "../parts/empty.svelte";
  import { RowList } from "../parts/row.svelte";
  import Segmented from "../parts/segmented.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Entry from "./entry.svelte";
  import Narrowing from "./narrowing.svelte";
  import { LEVELS, LEVEL_NAMES, SOURCES, daysOf, entriesOf } from "./timeline";

  interface Props {
    readonly source: Source;
    readonly onSource: (source: Source) => void;
  }

  const { source, onSource }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  let before = $state<Seq | null>(null);
  let open = $state<Seq | null>(null);
  let run = $state<RunId | null>(null);
  let level = $state<LogLevel | null>(null);
  let moduleName = $state<string | null>(null);

  const question = $derived<Query>({ history: { before, limit: 100 } });
  const history = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($history, (held) => ("history" in held ? held.history : undefined)));
  const records = $derived(read.kind === "held" ? read.value.records : []);
  const earlier = $derived(read.kind === "held" ? (read.value.earlier ?? null) : null);
  const lines = $derived($belief.logs);
  const entries = $derived(
    entriesOf({ records, head: before === null }, lines, source, { run, level, module: moduleName }),
  );
  const days = $derived(daysOf(entries));
  const runs = $derived([...new Set([...records.map((each) => each.run), ...lines.flatMap((each) => each.run ?? [])])]);
  const modules = $derived(modulesIn(lines));
  const sources = $derived(SOURCES.map((value) => ({ value, label: say($lang, SOURCE_NAMES[value]) })));

  function toggle(seq: Seq): void {
    open = open === seq ? null : seq;
  }
</script>

<section class="@container flex min-w-0 flex-col gap-base" aria-label={say($lang, "rec_timeline")}>
  <div class="flex flex-wrap items-center gap-x-wide gap-y-snug">
    <Segmented label={say($lang, "rec_source")} options={sources} held={source} onPick={onSource} />
    <Narrowing
      label={say($lang, "log_runs")}
      options={runs.map((each) => ({ value: each, label: each.slice(0, 8) }))}
      current={run}
      onPick={(value) => {
        run = runs.find((each) => each === value) ?? null;
      }}
    />
    {#if source !== "ledger"}
      <Narrowing
        label={say($lang, "log_levels")}
        options={LEVELS.map((each) => ({ value: each, label: say($lang, LEVEL_NAMES[each]) }))}
        current={level}
        onPick={(value) => {
          level = LEVELS.find((each) => each === value) ?? null;
        }}
      />
      <Narrowing
        label={say($lang, "log_modules")}
        options={modules.map((each) => ({ value: each, label: each }))}
        current={moduleName}
        onPick={(value) => {
          moduleName = modules.find((each) => each === value) ?? null;
        }}
      />
    {/if}
  </div>
  {#if source !== "ledger"}
    <p class="text-note text-text-faint">{say($lang, "log_window")}</p>
  {/if}

  {#if read.kind === "unavailable" && source !== "log"}
    <Unanswered query={read.query} asked={question} />
  {:else if read.kind === "asking" && source !== "log"}
    <p class="text-note text-text-faint">…</p>
  {:else if entries.length === 0}
    <EmptyState missing={source === "log" ? "log_empty" : "rec_nothing"} seat="region" />
  {:else}
    {#snippet rows()}
      {#each entries as entry, at (entry.key)}
        <Entry {entry} day={days[at] ?? null} open={entry.kind === "record" && open === entry.seq} onToggle={toggle} />
      {/each}
    {/snippet}
    <!-- eslint-disable-next-line @typescript-eslint/no-unsafe-call (RowList is the keyboard walk `parts/row` exports as a snippet; svelte-check resolves its type where the lint's type graph does not) -->
    {@render RowList({ label: say($lang, "rec_timeline"), rows })}
    {#if source !== "log"}
      <div class="flex gap-base">
        {#if earlier !== null}
          <Button
            label={say($lang, "rec_earlier")}
            tone="secondary"
            onPress={() => {
              before = earlier;
            }}
          />
        {/if}
        {#if before !== null}
          <Button
            label={say($lang, "rec_newest")}
            tone="quiet"
            onPress={() => {
              before = null;
            }}
          />
        {/if}
      </div>
    {/if}
  {/if}
</section>

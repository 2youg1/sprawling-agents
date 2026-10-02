<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The record's diagnostics as one timeline (refrain S7.6 6c): what the
  // ledger wrote and what the process logged, in one column on one axis,
  // the ledger position. `timeline.ts` decides what stands in the column;
  // this file draws it the way the panorama's timeline is drawn - the
  // day once at the head, then a row per entry: the time of day to the
  // millisecond in UTC, the ledger position, what happened, and where.
  //
  // **The process log is a source, not a lens.** It used to be a tab of
  // its own, which put a line and the record it explains on two screens;
  // now it is one of three sources, and `#/record/log` still opens the
  // timeline with only the log showing. Choosing a source moves the
  // address bar the same way, so a link to what the machine was writing
  // is still a link.
  //
  // A ledger record opens to the record as it was written, the chain's
  // own fields included; a log line is one line and opens nothing.

  import type { Key } from "../../core/lang";
  import type { LogLevel, LogLine } from "../../wire";
  import type { Source } from "./timeline";

  const SOURCE_NAMES: Record<Source, Key> = {
    every: "rec_source_every",
    ledger: "rec_ledger",
    log: "rec_log",
  };

  // The five levels `docs/logging.md` names, in its order.
  const LEVELS: readonly LogLevel[] = ["refuse", "effect", "decide", "trace", "wire"];
  const LEVEL_NAMES: Record<LogLevel, Key> = {
    refuse: "log_refuse",
    effect: "log_effect",
    decide: "log_decide",
    trace: "log_trace",
    wire: "log_wire",
  };

  // Every module that has spoken, sorted: the modules worth offering as
  // a filter are the ones that wrote lines.
  function modulesIn(lines: readonly LogLine[]): readonly string[] {
    return [...new Set(lines.map((line) => line.module))].sort();
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { isoDay, isoInstant, isoTime } from "../../core/time";
  import { ui } from "../../ui";
  import type { EventRecord, Query, RunId, Seq } from "../../wire";
  import Button from "../parts/button.svelte";
  import EmptyState from "../parts/empty.svelte";
  import { RowList } from "../parts/row.svelte";
  import Segmented from "../parts/segmented.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import { factsOf, whatHappened } from "./event";
  import type { Entry } from "./timeline";
  import { SOURCES, entriesOf } from "./timeline";

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
  const runs = $derived([...new Set([...records.map((each) => each.run), ...lines.flatMap((each) => each.run ?? [])])]);
  const modules = $derived(modulesIn(lines));
  const sources = $derived(SOURCES.map((value) => ({ value, label: say($lang, SOURCE_NAMES[value]) })));

  function dayOf(entry: Entry): string | null {
    return entry.t === null ? null : isoDay(entry.t);
  }

  // The day is written once, above the first row of each day.
  function opensDay(at: number): string | null {
    const here = entries[at];
    if (here === undefined) return null;
    const day = dayOf(here);
    const above = entries.slice(0, at).filter((each) => each.t !== null).at(-1);
    return day !== null && (above === undefined || dayOf(above) !== day) ? day : null;
  }

  // The level a log line was written at; a ledger record has none.
  function levelWord(entry: Entry): string {
    return entry.kind === "log" ? say($lang, LEVEL_NAMES[entry.line.level]) : "";
  }

  function factLine(record: EventRecord): string {
    return factsOf(record.data)
      .map((fact) => `${fact.name}: ${fact.value}`)
      .join(" · ");
  }

  // Time, position, what happened, where. Under 30rem the line becomes
  // two: the time and the position, then what happened, whole.
  const ROW =
    "grid grid-cols-[14ch_6ch_minmax(0,1fr)] items-baseline gap-x-base py-tight text-note @min-[48rem]:grid-cols-[14ch_6ch_minmax(0,1fr)_minmax(0,24ch)] @max-[30rem]:grid-cols-[14ch_minmax(0,1fr)]";
  const SEQ = "figure text-right text-text-faint @max-[30rem]:text-left";
  const WHAT = "min-w-0 truncate @max-[30rem]:col-span-full @max-[30rem]:whitespace-normal";
</script>

{#snippet narrow(props: {
  label: string;
  options: readonly { value: string; label: string }[];
  current: string | null;
  onPick: (value: string | null) => void;
})}
  <label class="flex items-center gap-snug text-note text-text-faint">
    {props.label}
    <select
      class="h-control-sm max-w-[24ch] rounded-control bg-raised px-snug font-mono text-note text-text"
      value={props.current ?? ""}
      onchange={(event) => {
        const value = event.currentTarget.value;
        props.onPick(value === "" ? null : value);
      }}
    >
      <option value="">{say($lang, "log_every")}</option>
      {#each props.options as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </label>
{/snippet}

<section class="@container flex min-w-0 flex-col gap-base" aria-label={say($lang, "rec_timeline")}>
  <div class="flex flex-wrap items-center gap-x-wide gap-y-snug">
    <Segmented label={say($lang, "rec_source")} options={sources} held={source} onPick={onSource} />
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
    {@render narrow({
      label: say($lang, "log_runs"),
      options: runs.map((each) => ({ value: each, label: each.slice(0, 8) })),
      current: run,
      onPick: (value) => {
        run = runs.find((each) => each === value) ?? null;
      },
    })}
    {#if source !== "ledger"}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
      {@render narrow({
        label: say($lang, "log_levels"),
        options: LEVELS.map((each) => ({ value: each, label: say($lang, LEVEL_NAMES[each]) })),
        current: level,
        onPick: (value) => {
          level = LEVELS.find((each) => each === value) ?? null;
        },
      })}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself) -->
      {@render narrow({
        label: say($lang, "log_modules"),
        options: modules.map((each) => ({ value: each, label: each })),
        current: moduleName,
        onPick: (value) => {
          moduleName = modules.find((each) => each === value) ?? null;
        },
      })}
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
        {@const day = opensDay(at)}
        {#if day !== null}
          <li class="flex justify-between border-b border-edge pt-base pb-tight text-note text-text-faint">
            <span class="figure">{day}</span>
            <span>{say($lang, "rec_utc")}</span>
          </li>
        {/if}
        {#if entry.kind === "record"}
          {@const record = entry.record}
          <li>
            <button
              type="button"
              class="{ROW} w-full rounded-card px-snug text-left transition-colors hover:wash"
              aria-expanded={open === record.seq}
              onclick={() => {
                open = open === record.seq ? null : record.seq;
              }}
            >
              <time class="figure text-text-faint" datetime={isoInstant(record.t)}>{isoTime(record.t)}</time>
              <span class={SEQ}>#{record.seq}</span>
              <span class={WHAT}>
                <span class="text-text">{say($lang, whatHappened(record.kind))}</span>
                <span class="text-text-quiet">{factLine(record)}</span>
              </span>
              <span class="hidden min-w-0 truncate text-right font-mono text-text-quiet @min-[48rem]:block">
                {record.addr ?? fill(say($lang, "rec_by"), { who: record.who })}
              </span>
            </button>
            {#if open === record.seq}
              <div class="flex flex-col gap-tight pt-tight pb-base pl-[calc(20ch+2*var(--spacing-base))] @max-[30rem]:pl-0">
                <span class="text-note text-text-faint">{say($lang, "rec_raw")}</span>
                <pre class="max-h-output overflow-auto rounded-card border border-edge p-base text-note text-text-quiet">{JSON.stringify(record, null, 2)}</pre>
              </div>
            {/if}
          </li>
        {:else}
          {@const logged = entry.line}
          <li class="{ROW} font-mono">
            {#if entry.t === null}
              <span class="text-text-faint"></span>
            {:else}
              <time class="figure text-text-faint" datetime={isoInstant(entry.t)}>{isoTime(entry.t)}</time>
            {/if}
            <span class={SEQ}>#{logged.seq}</span>
            <span class={WHAT}>
              <span class="text-text-faint">{levelWord(entry)}</span>
              <span class="text-text-quiet">{logged.line}</span>
            </span>
            <span class="hidden min-w-0 truncate text-right text-text-faint @min-[48rem]:block">{logged.module}</span>
          </li>
        {/if}
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

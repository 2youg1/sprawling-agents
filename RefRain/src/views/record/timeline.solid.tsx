// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";
import { readAnswer } from "../../core/answered";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { LogLevel, Query, RunId, Seq } from "../../wire";
import Button from "../parts/button.solid";
import EmptyState from "../parts/empty.solid";
import { RowList } from "../parts/row.solid";
import Segmented from "../parts/segmented.solid";
import Unanswered from "../parts/unanswered.solid";
import { RecordEntry } from "./history_views";
import Narrowing from "./narrowing.solid";
import {
  LEVELS,
  LEVEL_NAMES,
  SOURCES,
  daysOf,
  entriesOf,
  type Entry,
  type Source,
} from "./timeline";

const SOURCE_NAMES: Record<Source, Key> = {
  every: "rec_source_every",
  ledger: "rec_ledger",
  log: "rec_log",
};

export default function Timeline(props: {
  readonly source: Source;
  readonly onSource: (source: Source) => void;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const [before, setBefore] = createSignal<Seq | null>(null);
  const [opened, setOpened] = createSignal<Seq | null>(null);
  const [run, setRun] = createSignal<RunId | null>(null);
  const [level, setLevel] = createSignal<LogLevel | null>(null);
  const [moduleName, setModule] = createSignal<string | null>(null);
  const question = createMemo<Query>(() => ({ history: { before: before(), limit: 100 } }));
  const history = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(history(), (held) => ("history" in held ? held.history : undefined)),
  );
  const held = createMemo(() => {
    const item = read();
    return item.kind === "held" ? item.value : undefined;
  });
  const refusal = createMemo(() => {
    const item = read();
    return item.kind === "unavailable" ? item.query : undefined;
  });
  const records = createMemo(() => held()?.records ?? []);
  const earlier = createMemo(() => held()?.earlier ?? undefined);
  const lines = createMemo(() => belief().logs);
  const entries = createMemo(() =>
    entriesOf({ records: records(), head: before() === null }, lines(), props.source, {
      run: run(),
      level: level(),
      module: moduleName(),
    }),
  );
  const days = createMemo(() => daysOf(entries()));
  const runs = createMemo(() => [
    ...new Set([
      ...records().map((each) => each.run),
      ...lines().flatMap((each) => each.run ?? []),
    ]),
  ]);
  const modules = createMemo(() => [...new Set(lines().map((line) => line.module))].sort());
  const sources = createMemo(() =>
    SOURCES.map((value) => ({ value, label: say(lang(), SOURCE_NAMES[value]) })),
  );
  const isOpen = (entry: Entry): boolean => entry.kind === "record" && opened() === entry.seq;
  const toggle = (seq: Seq): void => {
    setOpened((previous) => (previous === seq ? null : seq));
  };

  return (
    <section
      class="@container flex min-w-0 flex-col gap-base"
      aria-label={say(lang(), "rec_timeline")}
    >
      <div class="flex flex-wrap items-center gap-x-wide gap-y-snug">
        <Segmented
          label={say(lang(), "rec_source")}
          options={sources()}
          held={props.source}
          onPick={props.onSource}
        />
        <Narrowing
          label={say(lang(), "log_runs")}
          options={runs().map((value) => ({ value, label: value.slice(0, 8) }))}
          current={run()}
          onPick={(value) => {
            setRun(runs().find((id) => id === value) ?? null);
          }}
        />
        <Show when={props.source !== "ledger"}>
          <Narrowing
            label={say(lang(), "log_levels")}
            options={LEVELS.map((value) => ({ value, label: say(lang(), LEVEL_NAMES[value]) }))}
            current={level()}
            onPick={(value) => {
              setLevel(LEVELS.find((id) => id === value) ?? null);
            }}
          />
          <Narrowing
            label={say(lang(), "log_modules")}
            options={modules().map((value) => ({ value, label: value }))}
            current={moduleName()}
            onPick={(value) => {
              setModule(modules().find((id) => id === value) ?? null);
            }}
          />
        </Show>
      </div>
      <Show when={props.source !== "ledger"}>
        <p class="text-note text-text-faint">{say(lang(), "log_window")}</p>
      </Show>
      <Switch>
        <Match when={props.source === "log" ? undefined : refusal()}>
          {(query) => <Unanswered query={query()} asked={question()} />}
        </Match>
        <Match when={read().kind === "asking" && props.source !== "log"}>
          <p class="text-note text-text-faint">…</p>
        </Match>
        <Match when={entries().length === 0}>
          <EmptyState
            missing={props.source === "log" ? "log_empty" : "rec_nothing"}
            seat="region"
          />
        </Match>
        <Match when={true}>
          <RowList label={say(lang(), "rec_timeline")}>
            <For each={entries()} keyed={(entry) => entry.key}>
              {(entry, index) => (
                <RecordEntry
                  entry={entry()}
                  day={days()[index()] ?? null}
                  open={isOpen(entry())}
                  onToggle={toggle}
                />
              )}
            </For>
          </RowList>
          <Show when={props.source !== "log"}>
            <div class="flex gap-base">
              <Show when={earlier()}>
                {(since) => (
                  <Button
                    label={say(lang(), "rec_earlier")}
                    tone="secondary"
                    onPress={() => {
                      setBefore(since());
                    }}
                  />
                )}
              </Show>
              <Show when={before() !== null}>
                <Button
                  label={say(lang(), "rec_newest")}
                  tone="quiet"
                  onPress={() => {
                    setBefore(null);
                  }}
                />
              </Show>
            </div>
          </Show>
        </Match>
      </Switch>
    </section>
  );
}

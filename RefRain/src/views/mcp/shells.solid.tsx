// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createMemo } from "solid-js";
import { readAnswer } from "../../core/answered";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Query, ShellCalls } from "../../wire";
import Table, { type Column } from "../parts/table.solid";
import Unanswered from "../parts/unanswered.solid";

const CLASSES: readonly (readonly [string, Key])[] = [
  ["command_not_found", "shells_command_not_found"],
  ["syntax", "shells_syntax"],
  ["encoding", "shells_encoding"],
];
const QUESTION: Query = "shells";

export default function Shells() {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUESTION));
  const read = createMemo(() =>
    readAnswer(asked(), (held) => ("shells" in held ? held.shells.interpreters : undefined)),
  );
  const refusal = createMemo(() => {
    const value = read();
    return value.kind === "unavailable" ? value : undefined;
  });
  const rows = createMemo(() => {
    const value = read();
    return value.kind === "held" ? value.value : undefined;
  });
  const columns = createMemo<readonly Column<ShellCalls>[]>(() => [
    {
      key: "interpreter",
      header: say(lang(), "shells_interpreter"),
      min: "12ch",
      render: (row) => <span class="font-mono text-text-quiet">{row.interpreter}</span>,
    },
    {
      key: "calls",
      header: say(lang(), "shells_calls"),
      min: "figure",
      compare: (left, right) => left.calls - right.calls,
      render: (row) => <span class="tabular-nums">{row.calls}</span>,
    },
    {
      key: "failures",
      header: say(lang(), "shells_failures"),
      min: "24ch",
      render: (row) => (
        <Show
          when={Object.keys(row.failures).length > 0}
          fallback={<span class="text-text-faint">{say(lang(), "shells_no_failures")}</span>}
        >
          <span class="flex flex-wrap gap-base">
            <For each={Object.entries(row.failures)}>
              {([name, total]) => {
                const key = CLASSES.find(([spelled]) => spelled === name)?.[1];
                return (
                  <span>
                    <span class="text-text-faint">
                      {key === undefined ? name : say(lang(), key)}
                    </span>
                    <span class="tabular-nums">{total}</span>
                  </span>
                );
              }}
            </For>
          </span>
        </Show>
      ),
    },
  ]);
  return (
    <section class="flex min-w-0 flex-col gap-base">
      <h2 class="text-note text-text-faint">{say(lang(), "shells_title")}</h2>
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={refusal()}>
          {(unavailable) => (
            <Unanswered
              query={unavailable().query}
              reason={unavailable().reason}
              asked={QUESTION}
            />
          )}
        </Match>
        <Match when={rows()}>
          {(held) => (
            <Table
              caption={say(lang(), "shells_title")}
              columns={columns()}
              rows={held()}
              keyOf={(row) => row.interpreter}
              empty={() => <p class="text-note text-text-faint">{say(lang(), "shells_none")}</p>}
            />
          )}
        </Match>
      </Switch>
    </section>
  );
}

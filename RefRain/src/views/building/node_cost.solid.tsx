// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { within } from "../../core/belief/live";
import { fill, say } from "../../core/lang";
import { roomOf, toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { usd } from "../../core/time";
import { ui } from "../../ui";
import type { Address, NodeId, Query } from "../../wire";
import Unanswered from "../parts/unanswered.solid";
import FactLink from "./fact_link.look.solid";

interface Props {
  readonly building: Address;
  readonly node: NodeId;
}

const RUN_SHORT = 8;

export default function NodeCost(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const question = createMemo<Query>(() => ({ cost_of: { node: props.node } }));
  const asked = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(asked(), (held) => ("cost_of" in held ? held.cost_of : undefined)),
  );
  const lost = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const placed = createMemo(() => {
    const answer = read();
    if (answer.kind !== "held") return null;
    const here = answer.value.runs.flatMap(([run, spent]) => {
      const known = belief().runs[run];
      return known !== undefined && within(known, props.building)
        ? [{ run, spent, room: known.addr ?? props.building }]
        : [];
    });
    return {
      here,
      spent: here.reduce((sum, row) => sum + row.spent, 0),
      elsewhere: answer.value.runs.length - here.length,
    };
  });

  return (
    <Switch fallback={<p class="text-note text-text-faint">…</p>}>
      <Match when={lost()}>
        {(answer) => <Unanswered query={answer().query} asked={question()} />}
      </Match>
      <Match when={placed()}>
        {(cost) => (
          <div class="flex min-w-0 flex-col gap-tight text-note">
            <p class="flex items-baseline gap-base">
              <span class="text-text-faint">{say(lang(), "plan_node_spent")}</span>
              <span class="figure text-text">{usd(cost().spent)}</span>
            </p>
            <Show
              when={cost().here.length > 0}
              fallback={<p class="text-text-faint">{say(lang(), "plan_node_unclaimed")}</p>}
            >
              <ul>
                <For each={cost().here} keyed={(row) => row.run}>
                  {(row) => (
                    <li class="grid grid-cols-[10ch_minmax(0,1fr)_auto] items-center gap-x-base">
                      <FactLink
                        label={row().run.slice(0, RUN_SHORT)}
                        face="figure"
                        ink="quiet"
                        wire={{ href: toFragment({ kind: "run", run: row().run }) }}
                      />
                      <span class="truncate font-mono text-text-faint">{roomOf(row().room)}</span>
                      <span class="figure text-text-quiet">{usd(row().spent)}</span>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
            <Show when={cost().elsewhere > 0}>
              <p class="text-text-faint">
                {fill(say(lang(), "plan_node_elsewhere"), { n: String(cost().elsewhere) })}
              </p>
            </Show>
          </div>
        )}
      </Match>
    </Switch>
  );
}

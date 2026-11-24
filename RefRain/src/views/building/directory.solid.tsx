// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Match, Show, Switch } from "solid-js";
import type { Key } from "../../core/lang";
import { heldIn } from "../../core/belief/rooms";
import { roomOf } from "../../core/route";
import type { Address, Entry } from "../../wire";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import type { Doing } from "../../core/doing";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { clock, kib, usd } from "../../core/time";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { Address as AddressSchema, type Query } from "../../wire";
import EmptyState from "../parts/empty.solid";
import Path from "../parts/path.solid";
import Unanswered from "../parts/unanswered.solid";
import FactLink from "./fact_link.look.solid";
import RunRow from "./run_row.look.solid";
import TalkLink from "./talk_link.look.solid";
import type { Picked } from "./tree.solid";
import { transcriptOf } from "./transcript";

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
interface Props {
  readonly at: Address;
  readonly root: Address;
  readonly onPick: (picked: Picked) => void;
}

export default function Directory(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const question = createMemo<Query>(() => ({ listing: { at: props.at } }));
  const asked = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("listing" in answer ? answer.listing.entries : undefined)),
  );
  const unavailable = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const entries = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  const kind = () => kindOf(props.at, entries() ?? []);
  const cost = storeValue(u.conn.asking.ask(QUERIES.cost));
  const runs = createMemo(() => heldIn(belief(), props.at).reverse());
  const earlier = createMemo(() =>
    (entries() ?? []).flatMap((entry) => {
      const run = transcriptOf(entry.name);
      return run === null || runs().some((held) => held.run === run) ? [] : [run];
    }),
  );
  const cold = createMemo(() => {
    const held = cost();
    if (held === undefined || !("cost" in held)) return [];
    const warm = new Set(held.cost.by_run.map(([name]) => name));
    return runs()
      .map((run) => run.run)
      .filter((run) => !warm.has(run));
  });
  const coldCost = storeValue(() => {
    const names = cold();
    return names.length === 0 ? undefined : u.conn.asking.ask({ run_costs: { runs: names } });
  });

  function spent(run: string): number | null {
    const held = cost();
    if (held === undefined || !("cost" in held)) return null;
    const warm = held.cost.by_run.find(([name]) => name === run)?.[1];
    if (warm !== undefined) return warm;
    const asked = coldCost();
    if (asked === undefined || !("run_costs" in asked)) return null;
    return asked.run_costs.runs.find(([name]) => name === run)?.[1] ?? null;
  }

  function posture(doing: Doing): string {
    switch (doing.kind) {
      case "thinking":
        return say(lang(), "run_doing_thinking");
      case "calling":
        return say(lang(), "run_doing_calling");
      case "waiting":
        return say(lang(), "talk_waiting_you");
      case "awaiting_reply":
        return say(lang(), "run_doing_awaiting_reply");

      case "unknown":
        return say(lang(), "city_at_work");
      case "frozen":
        switch (doing.completion) {
          case "done":
            return say(lang(), "outcome_done");
          case "limit":
            return say(lang(), "outcome_limit");
          case "cancelled":
            return say(lang(), "outcome_cancelled");
          case null:
          default:
            return say(lang(), "outcome_frozen");
        }
    }
  }

  function OpenTalk() {
    return (
      <TalkLink
        label={say(lang(), "bld_open_talk")}
        wire={{ href: toFragment({ kind: "talk", address: props.at }) }}
      />
    );
  }

  return (
    <Switch fallback={<p class="text-text-faint">…</p>}>
      <Match when={unavailable()}>
        {(answer) => <Unanswered query={answer().query} asked={question()} />}
      </Match>
      <Match when={entries()}>
        {(listing) => (
          <div>
            <div class="mb-base flex flex-wrap items-baseline gap-base">
              <h2 class="text-heading font-heading">{roomOf(props.at)}</h2>
              <span class="text-note text-text-faint">{say(lang(), KIND_WORD[kind()])}</span>
              <span class="flex-1" />
              <Show when={kind() === "room"}>
                <OpenTalk />
              </Show>
            </div>
            <Show when={kind() === "governance"}>
              <p class="mb-base text-note text-text-quiet">{say(lang(), "dir_governance_what")}</p>
            </Show>
            <Switch fallback={<EmptyState missing="dir_empty" />}>
              <Match when={kind() === "room"}>
                <Show
                  when={runs().length > 0}
                  fallback={
                    <Show when={earlier().length === 0}>
                      <EmptyState missing="dir_room_empty" action={<OpenTalk />} />
                    </Show>
                  }
                >
                  <ul class="text-note">
                    <For each={runs()} keyed={(run) => run.run}>
                      {(run) => {
                        const fee = () => {
                          const micros = spent(run().run);
                          return micros === null || micros === 0 ? undefined : usd(micros);
                        };
                        const started = () => {
                          const time = run().started;
                          return time ? clock(lang(), time) : undefined;
                        };
                        return (
                          <li class="border-b border-edge">
                            <RunRow
                              task={run().task ?? run().run}
                              live={run().doing.kind !== "frozen"}
                              posture={posture(run().doing)}
                              spent={fee()}
                              started={started()}
                              wire={{ href: toFragment({ kind: "run", run: run().run }) }}
                            />
                          </li>
                        );
                      }}
                    </For>
                  </ul>
                </Show>
                <Show when={earlier().length > 0}>
                  <p class="mt-base text-note text-text-faint">
                    {fill(say(lang(), "dir_earlier_runs"), { n: String(earlier().length) })}
                  </p>
                  <ul class="flex flex-wrap gap-x-base text-note">
                    <For each={earlier()} keyed>
                      {(run) => (
                        <li>
                          <FactLink
                            label={run.slice(0, 8)}
                            face="figure"
                            ink="quiet"
                            wire={{ href: toFragment({ kind: "run", run }) }}
                          />
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </Match>
              <Match when={listing().length > 0}>
                <ul class="text-note">
                  <For each={listing()} keyed={(entry) => entry.name}>
                    {(entry) => {
                      const at = () => AddressSchema.make(`${props.at}/${entry().name}`);
                      const file = () => {
                        const kind = entry().kind;
                        return kind === "directory" ? undefined : kind.file;
                      };
                      return (
                        <li class="flex items-center gap-base border-b border-edge py-snug">
                          <Path
                            path={at()}
                            base={props.root}
                            onOpen={() =>
                              props.onPick({
                                at: at(),
                                kind: file() === undefined ? "directory" : "file",
                              })
                            }
                          />
                          <span class="flex-1" />
                          <Show when={file()}>
                            {(info) => (
                              <span class="shrink-0 font-mono text-text-faint">
                                {kib(info().bytes)}
                              </span>
                            )}
                          </Show>
                        </li>
                      );
                    }}
                  </For>
                </ul>
              </Match>
            </Switch>
          </div>
        )}
      </Match>
    </Switch>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, For, Match, Show, Switch } from "solid-js";
import { fill, say, type Key } from "../../core/lang";
import { percent } from "../../core/share";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { BuildingAnswer, PlanRow, RoadmapStatus } from "../../wire";
import Badge from "../parts/badge.solid";
import type { Weight } from "../parts/glyph";
import Progress from "../parts/progress.solid";
import { disclosureWire } from "./disclosure";
import Disclosure from "./disclosure.look.solid";
import NodeCost from "./node_cost.solid";

const STATUS_WORD: Record<RoadmapStatus, Key> = {
  not_started: "status_not_started",
  in_progress: "status_in_progress",
  done: "status_done",
  blocked: "status_blocked",
  awaiting_approval: "status_awaiting_approval",
};

function weightOf(row: PlanRow): Weight {
  switch (row.status) {
    case "in_progress":
      return "live";
    case "blocked":
    case "awaiting_approval":
      return "alert";
    case "not_started":
      return row.ready ? "live" : "quiet";
    case "done":
      return "quiet";
  }
}

function indentOf(node: string): string {
  return `calc(var(--spacing-pane) * ${String(node.split(".").length - 1)})`;
}

export default function Plan(props: { readonly answer: BuildingAnswer }) {
  const lang = storeValue(ui().lang);
  const [open, setOpen] = createSignal<string | null>(null);
  const planned = () => {
    const progress = props.answer.progress;
    return "planned" in progress && progress.planned.total > 0 ? progress.planned : undefined;
  };
  const unplanned = () => {
    const progress = props.answer.progress;
    return "unplanned" in progress ? progress.unplanned : undefined;
  };
  return (
    <div>
      <h2 class="sr-only">{say(lang(), "bld_plan")}</h2>
      <Switch>
        <Match when={planned()}>
          {(progress) => (
            <div class="mb-base flex items-center gap-base">
              <Progress
                label={say(lang(), "plan_progress")}
                done={progress().done}
                total={progress().total}
              />
              <span class="shrink-0 font-mono text-note text-text-faint">
                {fill(say(lang(), "plan_share"), {
                  percent: String(percent(progress().done_ppb)),
                })}
              </span>
            </div>
          )}
        </Match>
        <Match when={unplanned()}>
          {(progress) => (
            <p class="mb-base text-note text-text-faint">
              {fill(say(lang(), "plan_unplanned"), { steps: String(progress().steps) })}
            </p>
          )}
        </Match>
      </Switch>
      <Show when={props.answer.problems.length > 0}>
        <ul class="mb-base rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
          <For each={props.answer.problems} keyed>
            {(problem) => <li>{problem}</li>}
          </For>
        </ul>
      </Show>
      <Show
        when={props.answer.plan.length > 0}
        fallback={<p class="text-text-faint">{say(lang(), "plan_empty")}</p>}
      >
        <ul class="border-t border-edge text-note">
          <For each={props.answer.plan} keyed={(row) => row.node}>
            {(row) => {
              const expanded = () => open() === row().node;
              const status = () => {
                const current = row();
                return current.status === "not_started" && current.ready
                  ? "status_ready"
                  : STATUS_WORD[current.status];
              };
              return (
                <li class="border-b border-edge">
                  <Disclosure
                    layout="plan"
                    open={expanded()}
                    wire={disclosureWire(expanded(), () => setOpen(expanded() ? null : row().node))}
                  >
                    <span
                      class="flex min-w-0 items-baseline gap-base"
                      style={{ "padding-left": indentOf(row().node) }}
                    >
                      <span class="w-figure shrink-0 figure text-text-faint">{row().node}</span>
                      <span class={row().status === "done" ? "text-text-faint" : "text-text"}>
                        {row().item}
                        <Show when={row().needs.length > 0}>
                          <span class="ml-snug text-text-faint">← {row().needs.join(", ")}</span>
                        </Show>
                      </span>
                    </span>
                    <Badge text={say(lang(), status())} weight={weightOf(row())} dot />
                  </Disclosure>
                  <Show when={expanded()}>
                    <div class="pb-base pl-[calc(var(--spacing-glyph-sm)+var(--spacing-snug)+var(--spacing-figure)+var(--spacing-base))]">
                      <NodeCost building={props.answer.addr} node={row().node} />
                    </div>
                  </Show>
                </li>
              );
            }}
          </For>
        </ul>
      </Show>
      <Show when={props.answer.blocked.length > 0}>
        <ul class="mt-base text-note text-text-quiet">
          <For each={props.answer.blocked} keyed={(line) => line.source}>
            {(line) => (
              <li class="my-tight">
                <span class="text-alert">{line().source}</span> · {line().line}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

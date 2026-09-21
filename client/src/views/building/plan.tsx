// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The plan as rows: what the building set out to do, how deep each item
// sits, what it waits for, and where it stands. What the city could not
// read is stated above the plan rather than swallowed, and what is stuck
// is stated below it with the line that said so.
//
// The rows are still drawn here rather than by `parts/table.tsx`: that
// component gives every column a header, and the three words this plan
// would need - the node number, the item, its state - are not in
// `lang.json` yet. The move is one edit behind those three entries.
//
// The state on the right is `parts/badge.tsx`. It was five hand-drawn
// paints here, which is the same thing that component is for, and the
// nested conditional that chose between them had no arm for a row
// awaiting approval - that row was drawn as one nobody had started.

import { For, Show } from "solid-js";

import type { BuildingAnswer, PlanRow } from "../../wire";
import { useSay } from "../../ui";
import { Badge } from "../parts/badge";
import type { Weight } from "../parts/badge";

function statusWord(say: ReturnType<typeof useSay>, row: PlanRow): string {
  if (row.status === "not_started" && row.ready) return say("status_ready");
  return say(`status_${row.status}`);
}

// How loudly one row's state is drawn. Exhaustive over the five states
// the wire carries, so a sixth is a compile error rather than a row
// that quietly looks like an idle one. The colour only repeats the
// word beside it, which is why two states may share a weight: `ready`
// and `in_progress` are both the city working, and `blocked` and
// `awaiting_approval` are both the city stopped until somebody acts.
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

export function Plan(props: { readonly answer: BuildingAnswer }) {
  const say = useSay();
  return (
    <div>
      <Show when={props.answer.problems.length > 0}>
        <ul class="mb-base rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
          <For each={props.answer.problems}>{(problem) => <li>{problem}</li>}</For>
        </ul>
      </Show>
      <Show when={props.answer.plan.length > 0} fallback={<p class="text-text-faint">{say("plan_empty")}</p>}>
        <table class="w-full border-collapse text-note">
          <tbody>
            <For each={props.answer.plan}>
              {(row) => {
                // One pane of indent per level of the node number, so
                // the plan tightens with the rest of the page instead of
                // holding a width of its own.
                const indent = () => `calc(var(--spacing-pane) * ${String(row.node.split(".").length - 1)})`;
                return (
                  <tr class="border-b border-edge">
                    <td class="w-figure py-snug pr-snug font-mono text-text-faint" style={{ "padding-left": indent() }}>
                      {row.node}
                    </td>
                    <td class={`py-snug pr-snug ${row.status === "done" ? "text-text-faint" : "text-text"}`}>
                      {row.item}
                      <Show when={row.needs.length > 0}>
                        <span class="ml-snug text-text-disabled">← {row.needs.join(", ")}</span>
                      </Show>
                    </td>
                    <td class="py-snug text-right whitespace-nowrap">
                      <Badge text={statusWord(say, row)} weight={weightOf(row)} dot />
                    </td>
                  </tr>
                );
              }}
            </For>
          </tbody>
        </table>
      </Show>
      <Show when={props.answer.blocked.length > 0}>
        <ul class="mt-base text-note text-text-quiet">
          <For each={props.answer.blocked}>
            {(line) => (
              <li class="my-tight">
                <span class="text-alert">{line.source}</span> · {line.line}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

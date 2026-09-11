// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The plan as rows: what the building set out to do, how deep each item
// sits, what it waits for, and where it stands. What the city could not
// read is stated above the plan rather than swallowed, and what is stuck
// is stated below it with the line that said so.

import { For, Show } from "solid-js";

import type { BuildingAnswer, PlanRow } from "../../wire";
import { useSay } from "../../ui";

function statusWord(say: ReturnType<typeof useSay>, row: PlanRow): string {
  if (row.status === "not_started" && row.ready) return say("status_ready");
  return say(`status_${row.status}`);
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
                const depth = () => row.node.split(".").length - 1;
                return (
                  <tr class="border-b border-g1">
                    <td class="w-figure py-snug pr-snug font-mono text-text-faint" style={{ "padding-left": `${String(depth() * 16)}px` }}>
                      {row.node}
                    </td>
                    <td class={`py-snug pr-snug ${row.status === "done" ? "text-text-faint" : "text-text"}`}>
                      {row.item}
                      <Show when={row.needs.length > 0}>
                        <span class="ml-snug text-text-disabled">← {row.needs.join(", ")}</span>
                      </Show>
                    </td>
                    <td class="py-snug text-right whitespace-nowrap">
                      <span
                        class={`rounded-pill px-snug py-tight ${
                          row.status === "done"
                            ? "bg-g2 text-text-faint"
                            : row.status === "blocked"
                              ? "bg-alert text-g0"
                              : row.status === "in_progress"
                                ? "bg-accent text-g0"
                                : row.ready
                                  ? "bg-g3 text-text"
                                  : "text-text-disabled"
                        }`}
                      >
                        {statusWord(say, row)}
                      </span>
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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import { Option, Schema } from "effect";
import { fill, say } from "../../core/lang";
import { buildingOf } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { count } from "../../core/time";
import { ui } from "../../ui";
import { Address, type Turn, type Used } from "../../wire";
import Path from "../parts/path.solid";

const PARTS = [
  ["run_cached", "bg-mark"],
  ["run_input", "bg-accent"],
  ["run_output", "bg-accent-solid"],
] as const;
const decode = Schema.decodeOption(Address);

function filesOf(turns: readonly Turn[]) {
  const files = new Map<string, number>();
  for (const turn of turns)
    for (const call of turn.calls) {
      const file = call.subject;
      if ((call.tool === "read" || call.tool === "search") && file != null)
        files.set(file, (files.get(file) ?? 0) + 1);
    }
  return [...files].sort((left, right) => right[1] - left[1]);
}

export default function RunContext(props: { readonly turns: readonly Turn[] }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const peak = createMemo(() =>
    Math.max(1, ...props.turns.map((turn) => (turn.used?.input ?? 0) + (turn.used?.output ?? 0))),
  );
  const files = createMemo(() => filesOf(props.turns));
  const width = (used: Used | null | undefined, part: 0 | 1 | 2): string => {
    if (used == null) return "0%";
    const cached = Math.min(used.cached ?? 0, used.input);
    const total = part === 0 ? cached : part === 1 ? used.input - cached : used.output;
    return `${String((total / peak()) * 100)}%`;
  };
  const open = (file: string): (() => void) | undefined => {
    const target = Option.getOrUndefined(decode(file));
    return target === undefined
      ? undefined
      : () => {
          u.go({ kind: "building", address: buildingOf(target) });
        };
  };
  return (
    <div class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-wide">
      <section>
        <h2 class="mb-base text-note text-text-faint">{say(lang(), "run_window")}</h2>
        <Show
          when={props.turns.some((turn) => turn.used != null)}
          fallback={<p class="text-text-faint">{say(lang(), "run_no_usage")}</p>}
        >
          <ul class="grid grid-cols-[max-content_minmax(0,1fr)_max-content] gap-x-snug text-note">
            <For each={props.turns} keyed={(turn) => turn.number}>
              {(turn) => (
                <li class="col-span-full my-tight grid grid-cols-subgrid items-center">
                  <span class="whitespace-nowrap text-text-faint">
                    {fill(say(lang(), "run_turn_n"), { n: String(turn().number) })}
                  </span>
                  <span
                    class="flex h-dot flex-1 overflow-hidden rounded-pill bg-track"
                    aria-hidden="true"
                  >
                    <span class="bg-mark" style={{ width: width(turn().used, 0) }} />
                    <span class="bg-accent" style={{ width: width(turn().used, 1) }} />
                    <span class="bg-accent-solid" style={{ width: width(turn().used, 2) }} />
                  </span>
                  <span class="figure min-w-figure text-right text-text-faint">
                    <Show when={turn().used} fallback="—">
                      {(used) => count(used().input + used().output)}
                    </Show>
                  </span>
                </li>
              )}
            </For>
          </ul>
          <p class="mt-snug flex flex-wrap gap-x-base text-note text-text-faint">
            <For each={PARTS}>
              {([word, style]) => (
                <span class="inline-flex items-center gap-tight">
                  <span class={["size-dot rounded-pill", style]} />
                  {say(lang(), word)}
                </span>
              )}
            </For>
          </p>
        </Show>
      </section>
      <section>
        <h2 class="mb-base text-note text-text-faint">{say(lang(), "run_read_files")}</h2>
        <Show
          when={files().length > 0}
          fallback={<p class="text-text-faint">{say(lang(), "run_read_nothing")}</p>}
        >
          <ul class="text-note">
            <For each={files()} keyed={(row) => row[0]}>
              {(row) => (
                <li class="my-tight flex items-center justify-between gap-base">
                  <Path path={row()[0]} onOpen={open(row()[0])} />
                  <span class="figure shrink-0 text-text-faint">
                    {row()[1] > 1 ? `×${String(row()[1])}` : ""}
                  </span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
    </div>
  );
}

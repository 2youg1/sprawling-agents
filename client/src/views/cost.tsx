// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Cost in five cuts of one authoritative total. Shares are drawn
// against `total`, never against the sum of the rows, so an
// unattributed remainder stays visible. A provider that reported no
// price leaves the total at zero, and the page says so rather than
// printing $0.00 as if it were a measurement.

import { For, Show, createMemo } from "solid-js";

import { usd } from "../core/time";
import type { CostAnswer, UsdMicros } from "../wire";
import { useSay, useUi } from "../ui";

type Cut = "by_run" | "by_actor" | "by_segment" | "by_tool" | "by_skill";
const CUTS: readonly Cut[] = ["by_run", "by_actor", "by_segment", "by_tool", "by_skill"];

function Table(props: { readonly rows: readonly (readonly [string, UsdMicros])[]; readonly total: number }) {
  return (
    <ul class="text-note">
      <For each={[...props.rows].sort((a, b) => b[1] - a[1])}>
        {([name, amount]) => (
          <li class="my-tight">
            <div class="flex justify-between gap-base">
              <span class="truncate font-mono text-text-quiet">{name}</span>
              <span class="shrink-0 text-text">{usd(amount)}</span>
            </div>
            <div class="mt-tight h-dot overflow-hidden rounded-pill bg-g1">
              <div class="h-full bg-accent" style={{ width: `${String(props.total > 0 ? (amount / props.total) * 100 : 0)}%` }} />
            </div>
          </li>
        )}
      </For>
    </ul>
  );
}

export function Cost() {
  const ui = useUi();
  const say = useSay();
  const cost = ui.conn.asking.ask("cost_view");
  const answer = createMemo<CostAnswer | undefined>(() => {
    const held = cost();
    return held !== undefined && "cost" in held ? held.cost : undefined;
  });
  return (
    <div class="mx-auto w-full max-w-page px-pane py-wide">
      <div class="mb-wide flex items-baseline justify-between">
        <h1 class="text-title font-title">{say("cost_title")}</h1>
        <Show when={answer()}>
          {(held) => (
            <span class="text-figure font-figure">{held().total > 0 ? usd(held().total) : say("cost_none")}</span>
          )}
        </Show>
      </div>
      <Show when={answer()} fallback={<p class="text-text-disabled">…</p>}>
        {(held) => (
          <div class="grid gap-wide md:grid-cols-2">
            <For each={CUTS}>
              {(cut) => (
                <section>
                  <h2 class="mb-snug text-label font-label text-text-quiet">{say(`cost_${cut}`)}</h2>
                  <Show when={held()[cut].length > 0} fallback={<p class="text-note text-text-disabled">—</p>}>
                    <Table rows={held()[cut]} total={held().total} />
                  </Show>
                </section>
              )}
            </For>
          </div>
        )}
      </Show>
    </div>
  );
}

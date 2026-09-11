// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The city page: one information bar, then as many columns as the
// screen affords. The list of buildings arrives at the widest step,
// the drawing is always there, and what was picked stands beside the
// drawing when there is room and under it when there is not.
//
// The shell fills the viewport; `max-w-page` binds the legend and the
// panel, which are prose, and never the drawing.

import { For, Show, createMemo, createSignal } from "solid-js";

import type { Address, BuildingProgress } from "../wire";
import { useSay, useUi } from "../ui";
import { CityBar, CityLegend } from "./city/bar";
import { Panel } from "./city/panel";
import { Skyline } from "./city/skyline";
import { EmptyState } from "./parts/empty";

function share(building: BuildingProgress): string | null {
  if (!("planned" in building.progress)) return null;
  return String(Math.round(building.progress.planned.done_ppb / 1e7));
}

export function City() {
  const ui = useUi();
  const say = useSay();
  const city = ui.conn.asking.ask("city_view");
  const answer = createMemo(() => {
    const held = city();
    return held !== undefined && "city" in held ? held.city : undefined;
  });
  const [picked, setPicked] = createSignal<Address | null>(null);
  const buildings = createMemo(() => [...(answer()?.buildings ?? [])].sort((a, b) => a.addr.localeCompare(b.addr)));

  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <CityBar city={answer()} />
      <div class="relative flex min-h-0 flex-1 flex-col lg:flex-row">
        <nav
          class="hidden shrink-0 overflow-y-auto border-r border-g2 px-snug py-base wide:block wide:w-rail-open"
          aria-label={say("city_buildings")}
        >
          <For each={buildings()}>
            {(building) => (
              <button
                type="button"
                class={`flex h-step w-full items-center gap-snug rounded-control px-snug text-left text-note leading-none ${
                  picked() === building.addr ? "bg-g2 text-text" : "text-text-quiet hover:bg-g1"
                }`}
                aria-current={picked() === building.addr ? "true" : undefined}
                onClick={() => {
                  setPicked(building.addr);
                }}
              >
                <span class="min-w-0 flex-1 truncate font-mono">{building.addr}</span>
                <Show when={share(building)}>
                  {(percent) => (
                    <span class="shrink-0 font-mono text-text-disabled">
                      {say("city_done_percent", { percent: percent() })}
                    </span>
                  )}
                </Show>
              </button>
            )}
          </For>
        </nav>
        <section class="flex min-h-0 min-w-0 flex-1 flex-col justify-center overflow-auto px-pane py-base">
          <Show when={answer()} fallback={<p class="text-center text-text-disabled">…</p>}>
            {(held) => (
              <Show when={held().buildings.length > 0} fallback={<EmptyState text={say("city_no_buildings")} />}>
                <Skyline city={held()} picked={picked()} onPick={setPicked} />
                <CityLegend />
              </Show>
            )}
          </Show>
        </section>
        <Show when={picked()}>
          {(addr) => (
            <Show when={answer()}>
              {(held) => (
                <aside
                  class="w-full shrink-0 overflow-y-auto border-t border-g2 px-pane py-base lg:absolute lg:inset-y-0 lg:right-0 lg:z-10 lg:w-tree lg:border-t-0 lg:border-l lg:bg-g0 lg:shadow-composer wide:static wide:shadow-none"
                  aria-label={say("city_panel")}
                >
                  <Panel addr={addr()} city={held()} onClose={() => setPicked(null)} />
                </aside>
              )}
            </Show>
          )}
        </Show>
      </div>
    </div>
  );
}

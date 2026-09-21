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

import { QUERIES } from "../core/asking";
import { MAYOR, toFragment } from "../core/route";
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
  const city = ui.conn.asking.ask(QUERIES.city);
  const answer = createMemo(() => {
    const held = city();
    return held !== undefined && "city" in held ? held.city : undefined;
  });
  const [picked, setPicked] = createSignal<Address | null>(null);
  const buildings = createMemo(() => [...(answer()?.buildings ?? [])].sort((a, b) => a.addr.localeCompare(b.addr)));

  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <CityBar />
      <div class="relative flex min-h-0 flex-1 flex-col @lg/page:flex-row">
        <nav
          class="hidden shrink-0 overflow-y-auto border-r border-edge px-snug py-base @wide/page:block @wide/page:w-rail-open"
          aria-label={say("city_buildings")}
        >
          <For each={buildings()}>
            {(building) => (
              <button
                type="button"
                class={`flex h-step w-full items-center gap-snug rounded-control px-snug text-left text-note leading-none ${
                  picked() === building.addr ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome"
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
              <Show
                when={held().buildings.length > 0}
                fallback={
                  // A city with no buildings is a city nobody has asked
                  // for anything yet, and the Mayor is where a person
                  // asks: raising a building is a sentence in that
                  // conversation rather than a button this page could
                  // press.
                  <EmptyState
                    text={say("city_no_buildings")}
                    action={
                      <a
                        href={toFragment({ kind: "talk", address: MAYOR })}
                        class="rounded-control bg-accent px-base py-snug text-label text-on-accent hover:bg-accent-hover"
                      >
                        {say("city_ask_mayor")}
                      </a>
                    }
                  />
                }
              >
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
                  class="w-full shrink-0 overflow-y-auto border-t border-edge px-pane py-base @lg/page:absolute @lg/page:inset-y-0 @lg/page:right-0 @lg/page:z-10 @lg/page:w-tree @lg/page:border-t-0 @lg/page:border-l @lg/page:bg-page @lg/page:shadow-composer @wide/page:static @wide/page:shadow-none"
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

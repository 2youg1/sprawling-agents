// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { QUERIES } from "../../core/asking";
import { say } from "../../core/lang";
import { buildingDestination, toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import { buildingsOf } from "./buildings";

export interface WorkspaceNavProps {
  readonly current?: Address | undefined;
}

export default function WorkspaceNav(props: WorkspaceNavProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const city = storeValue(u.conn.asking.ask(QUERIES.city));
  const buildings = createMemo(() => {
    const held = city();
    return held !== undefined && "city" in held ? held.city.buildings : [];
  });
  const options = createMemo(() =>
    buildingsOf(city()).flatMap((address) => {
      const building = buildings().find((each) => each.addr === address);
      return building ? [building] : [];
    }),
  );
  function pick(raw: string): void {
    const available = buildings();
    const chosen = available.find((each) => each.addr === raw);
    if (chosen) u.go(buildingDestination(chosen.addr, available).view);
  }
  return (
    <div class="flex flex-wrap items-center gap-base">
      <Show when={options().length > 0 && props.current}>
        {(current) => (
          <select
            class="h-control max-w-full rounded-control border border-edge-input bg-raised px-base text-body text-text"
            aria-label={say(lang(), "workspace_switch")}
            value={current()}
            onChange={(event) => pick(event.currentTarget.value)}
          >
            <For each={options()} keyed={(building) => building.addr}>
              {(building) => (
                <option value={building().addr}>
                  {building().addr}
                  {building().adoptable ? ` · ${say(lang(), "city_adoptable")}` : ""}
                </option>
              )}
            </For>
          </select>
        )}
      </Show>
      <a
        class="inline-flex h-control items-center rounded-control bg-accent px-base text-label text-on-accent transition-colors ease-leave hover:bg-accent-hover hover:ease-arrive still:transition-none"
        href={toFragment({ kind: "raise" })}
      >
        {say(lang(), "workspace_new_import")}
      </a>
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which buildings this city has, and the column a person picks one in.
//
// Two screens ask the same question - the tool servers a building may
// reach, and the skills its reading room admits - so the order is
// decided once: the hall first, because it is the building every city
// has and the one a person opening either screen means, then the rest
// by name. A city that has not answered yet is the hall alone rather
// than an empty column, so neither screen has a state where nothing
// can be picked.

import { For } from "solid-js";
import type { Accessor } from "solid-js";
import { createMemo } from "solid-js";

import { QUERIES } from "../../core/asking";
import { MAYOR, buildingOf } from "../../core/route";
import type { Address } from "../../wire";
import { useUi } from "../../ui";

export const HALL = buildingOf(MAYOR);

export function useBuildings(): Accessor<readonly Address[]> {
  const ui = useUi();
  const city = ui.conn.asking.ask(QUERIES.city);
  return createMemo<readonly Address[]>(() => {
    const answer = city();
    if (answer === undefined || !("city" in answer)) return [HALL];
    const all = answer.city.buildings.map((each) => each.addr);
    return [HALL, ...all.filter((addr) => addr !== HALL).sort((a, b) => a.localeCompare(b))];
  });
}

export interface BuildingColumnProps {
  // The accessible name of the list, already in the person's language:
  // the screen says what it is a list of, because only the screen
  // knows what picking one of them changes.
  readonly label: string;
  readonly buildings: readonly Address[];
  readonly chosen: Address;
  readonly onPick: (addr: Address) => void;
  // What the hall is called on screen, already in the person's
  // language. Every other building is called by its address.
  readonly hall: string;
}

export function BuildingColumn(props: BuildingColumnProps) {
  return (
    <nav class="w-tree shrink-0" aria-label={props.label}>
      <ul class="flex flex-col gap-tight">
        <For each={props.buildings}>
          {(addr) => (
            <li>
              <button
                type="button"
                class={`w-full rounded-control px-base py-tight text-left text-note ${
                  props.chosen === addr ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome"
                }`}
                onClick={() => {
                  props.onPick(addr);
                }}
                aria-current={props.chosen === addr ? "true" : undefined}
              >
                {addr === HALL ? props.hall : addr}
              </button>
            </li>
          )}
        </For>
      </ul>
    </nav>
  );
}

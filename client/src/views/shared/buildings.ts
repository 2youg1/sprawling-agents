// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which buildings this city has, and the order a person meets them in.
// `buildings.svelte` draws the column; this file decides what stands in
// it, so the two screens that share it cannot order it two ways.
//
// Two screens ask the same question - the tool servers a building may
// reach, and the skills its reading room admits - so the order is
// decided once: the hall first, because it is the building every city
// has and the one a person opening either screen means, then the rest
// by name. A city that has not answered yet is the hall alone rather
// than an empty column, so neither screen has a state where nothing
// can be picked.

import { derived } from "svelte/store";
import type { Readable } from "svelte/store";

import { QUERIES } from "../../core/asking";
import { MAYOR, buildingOf } from "../../core/route";
import { ui } from "../../ui";
import type { Address, Answer } from "../../wire";

export const HALL = buildingOf(MAYOR);

// The list above as one reading the screens share. A store rather than
// a value, because the city's answer arrives after this is called; a
// template consumes it as `$buildings`.
export function useBuildings(): Readable<readonly Address[]> {
  return derived(ui().conn.asking.ask(QUERIES.city), buildingsOf);
}

// The order itself, read from whatever the city question last answered.
export function buildingsOf(answer: Answer | undefined): readonly Address[] {
  if (answer === undefined || !("city" in answer)) return [HALL];
  const all = answer.city.buildings.map((each) => each.addr);
  return [HALL, ...all.filter((addr) => addr !== HALL).sort((a, b) => a.localeCompare(b))];
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

// The component is re-exported beside its decisions so a caller takes
// the column and what stands in it through one specifier, the shape
// `parts/table.ts` established.
export { default as BuildingColumn } from "./buildings.svelte";

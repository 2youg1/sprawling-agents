// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { derived } from "../../core/store";
import type { Readable } from "../../core/store";

import { QUERIES } from "../../core/asking";
import { MAYOR, buildingOf } from "../../core/route";
import { ui } from "../../ui";
import type { Address, Answer } from "../../wire";

export const HALL = buildingOf(MAYOR);

export function useBuildings(): Readable<readonly Address[]> {
  return derived(ui().conn.asking.ask(QUERIES.city), buildingsOf);
}

export function buildingsOf(answer: Answer | undefined): readonly Address[] {
  if (answer === undefined || !("city" in answer)) return [HALL];
  const all = answer.city.buildings.map((each) => each.addr);
  return [HALL, ...all.filter((addr) => addr !== HALL).sort((a, b) => a.localeCompare(b))];
}

export interface BuildingColumnProps {
  readonly label: string;
  readonly buildings: readonly Address[];
  readonly chosen: Address;
  readonly onPick: (addr: Address) => void;
  readonly hall: string;
}

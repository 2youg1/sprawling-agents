// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The city's buildings as the table the city page opens on: one row a
// building, what its runs are doing counted into waiting, working and
// done, and the moments its runs began, which the row's time bar draws
// on the board's folded clock.
//
// **A row is a fixed-size summary.** It keeps counts and the start of
// each run it holds, never a run's history, so a table of a city that
// has run for a week costs what its runs' starts cost.

import type { Address, BuildingProgress } from "../../wire";
import type { BoardRun, Phase } from "../runs/lineage";
import { guide, phaseOf, placeByNeed, placeOf } from "../runs/lineage";

// One run's start on a building's bar, coloured by what the run is
// doing now.
export interface Start {
  readonly run: string;
  readonly at: number;
  readonly phase: Phase;
}

export interface BuildingRow {
  readonly addr: Address;
  readonly guide: string;
  readonly waiting: number;
  readonly working: number;
  readonly done: number;
  // The newest start among the building's runs; null while it has none.
  readonly latest: number | null;
  // The oldest start among the runs still going, where the bar's live
  // stretch begins; null when every run has ended.
  readonly since: number | null;
  readonly starts: readonly Start[];
}

// The table's rows, ordered as the board orders its buildings: one
// holding a waiting run first, then by name. A run whose building the
// city does not name is left out, because the board below lists it.
export function tableOf(buildings: readonly BuildingProgress[], runs: readonly BoardRun[]): readonly BuildingRow[] {
  const held = new Map<string, BoardRun[]>(buildings.map((each) => [each.addr, []]));
  for (const run of runs) held.get(placeOf(run.addr).building)?.push(run);
  const ordered = buildings.map((each) => ({ addr: each.addr, name: each.addr, runs: held.get(each.addr) ?? [] })).sort(placeByNeed);
  return ordered.map((building, at) => rowOf(building.addr, building.runs, guide([], at === ordered.length - 1)));
}

function rowOf(addr: Address, runs: readonly BoardRun[], place: string): BuildingRow {
  const starts = runs
    .flatMap((run) => (run.started === null ? [] : [{ run: run.run, at: run.started, phase: phaseOf(run.doing) }]))
    .sort((a, b) => a.at - b.at || a.run.localeCompare(b.run));
  const phases = runs.map((run) => phaseOf(run.doing));
  const going = starts.filter((start) => start.phase !== "done");
  return {
    addr,
    guide: place,
    waiting: phases.filter((phase) => phase === "person").length,
    working: phases.filter((phase) => phase !== "person" && phase !== "done").length,
    done: phases.filter((phase) => phase === "done").length,
    latest: starts.at(-1)?.at ?? null,
    since: going[0]?.at ?? null,
    starts,
  };
}

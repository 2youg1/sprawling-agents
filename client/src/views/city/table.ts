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

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { ago } from "../../core/time";
import type { Address, BuildingProgress, CityAnswer } from "../../wire";
import { FOLDS, along } from "../runs/fold";
import type { BoardRun, Phase } from "../runs/lineage";
import { ended, guide, phaseOf, placeByNeed, placeOf } from "../runs/lineage";

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
  const going = starts.filter((start) => !ended(start.phase));
  return {
    addr,
    guide: place,
    waiting: phases.filter((phase) => phase === "person").length,
    working: phases.filter((phase) => phase !== "person" && !ended(phase)).length,
    done: phases.filter(ended).length,
    latest: starts.at(-1)?.at ?? null,
    since: going[0]?.at ?? null,
    starts,
  };
}

// What `table.look.svelte` is handed: the scale over the bars, and each
// row with its words said and its moments placed along the folded
// clock, in per cent of the bar (client D95).
export interface TableLook {
  readonly label: string;
  readonly folds: readonly { readonly minutes: number; readonly at: number; readonly label: string }[];
  readonly now: string;
  readonly rows: readonly TableRowLook[];
}

export interface TableRowLook {
  readonly key: string;
  readonly guide: string;
  readonly name: string;
  readonly picked: boolean;
  readonly waiting: number;
  readonly working: number;
  readonly done: number;
  readonly latest: string;
  // The live stretch from the oldest run still going to now.
  readonly live: { readonly left: number; readonly width: number } | null;
  readonly starts: readonly { readonly key: string; readonly at: number; readonly phase: Phase }[];
  // Spread on the row's button: picking a row opens the building's
  // panel, as picking its tower in the drawing does.
  readonly wire: {
    readonly type: "button";
    readonly "aria-pressed": boolean;
    readonly "aria-label": string;
    readonly onclick: () => void;
  };
}

export interface TableSeat {
  readonly city: CityAnswer;
  readonly runs: readonly BoardRun[];
  readonly now: number;
  readonly picked: Address | null;
  readonly lang: Lang;
}

export function lookOf(seat: TableSeat, onPick: (addr: Address) => void): TableLook {
  const { lang, now } = seat;
  return {
    label: say(lang, "city_table"),
    folds: FOLDS.filter((fold) => fold.minutes > 0).map((fold) => ({
      minutes: fold.minutes,
      at: fold.at,
      label: fill(say(lang, "runs_minus"), { n: String(fold.minutes) }),
    })),
    now: say(lang, "runs_now"),
    rows: tableOf(seat.city.buildings, seat.runs).map((row) => {
      const name = row.addr === "hall" ? say(lang, "city_hall") : row.addr;
      const from = row.since === null ? null : along(row.since, now);
      return {
        key: row.addr,
        guide: row.guide,
        name,
        picked: row.addr === seat.picked,
        waiting: row.waiting,
        working: row.working,
        done: row.done,
        latest: row.latest === null ? say(lang, "city_table_none") : ago(lang, row.latest, now),
        live: from === null ? null : { left: from, width: 100 - from },
        starts: row.starts.map((start) => ({ key: start.run, at: along(start.at, now), phase: start.phase })),
        wire: {
          type: "button",
          "aria-pressed": row.addr === seat.picked,
          "aria-label": fill(say(lang, "city_table_row"), {
            name,
            waiting: String(row.waiting),
            working: String(row.working),
            done: String(row.done),
          }),
          onclick: () => {
            onPick(row.addr);
          },
        },
      };
    }),
  };
}

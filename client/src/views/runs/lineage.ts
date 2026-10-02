// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The city's runs as the lineage tree `sprawling view` prints - city,
// building, room, run - flattened into the rows the board draws, with
// the guide a terminal would print before each one.
//
// **What needs the person leads, at every level.** A run waiting for
// the person sorts first in its room, a room holding one sorts first in
// its building, and a building holding one sorts first in the city, so
// the first screen of a board of a thousand runs is the part a person
// has to act on. The address is the directory tree, so its first
// segment is the building and the rest is the room.

import { Option } from "effect";

import type { RunBelief } from "../../core/belief";
import type { Doing } from "../../core/doing";
import type { View } from "../../core/route";

// One run as the board needs it: what the belief already holds about
// it. `ended` is null until the wire carries a run's last moment.
export interface BoardRun {
  readonly run: string;
  readonly addr: string | null;
  readonly task: string | null;
  readonly goal: string | null;
  readonly started: number | null;
  readonly ended: number | null;
  readonly doing: Doing;
}

// The board's runs, read from the belief's run table. A run's last
// moment is not on the wire yet, so every run's bar reaches now. The
// fields are named rather than spread: a streamed token writes the run's
// `saying`, and a board that read it would be rebuilt for every token.
export function boardRuns(runs: Readonly<Record<string, RunBelief>>): BoardRun[] {
  return Object.values(runs).map(({ run, addr, task, goal, started, doing }) => ({
    run,
    addr,
    task,
    goal,
    started,
    ended: null,
    doing,
  }));
}

// The phase a run's bar is coloured by: whose turn it is, and for a
// run that has ended, how it ended. A run somebody stopped and a run
// that ran out of budget are not a run that finished, and drawing all
// three with one check mark told a person their cancelled work was done.
export type Phase = "model" | "tool" | "person" | "idle" | "done" | "stopped" | "capped";

export function phaseOf(doing: Doing): Phase {
  switch (doing.kind) {
    case "thinking":
      return "model";
    case "calling":
      return "tool";
    case "waiting":
      return "person";
    case "unknown":
      return "idle";
    case "frozen":
      return doing.completion === "cancelled" ? "stopped" : doing.completion === "limit" ? "capped" : "done";
  }
}

// Whether a phase is one a run ends in.
export function ended(phase: Phase): boolean {
  return phase === "done" || phase === "stopped" || phase === "capped";
}

export type Row =
  | { readonly kind: "building"; readonly key: string; readonly guide: string; readonly name: string; readonly asking: number; readonly runs: number }
  | { readonly kind: "room"; readonly key: string; readonly guide: string; readonly name: string }
  | { readonly kind: "run"; readonly key: string; readonly guide: string; readonly run: BoardRun };

interface Room {
  readonly name: string;
  readonly runs: BoardRun[];
}

interface Building {
  readonly name: string;
  readonly rooms: Map<string, Room>;
}

function asking(run: BoardRun): boolean {
  return phaseOf(run.doing) === "person";
}

export function guide(trail: readonly boolean[], last: boolean): string {
  return trail.map((more) => (more ? "│ " : "  ")).join("") + (last ? "└─" : "├─");
}

export function placeOf(addr: string | null): { readonly building: string; readonly room: string } {
  const [building = "", ...room] = (addr ?? "").split("/");
  return { building, room: room.join("/") };
}

function grouped(runs: readonly BoardRun[]): Building[] {
  const buildings = new Map<string, Building>();
  for (const run of runs) {
    const place = placeOf(run.addr);
    const building = buildings.get(place.building) ?? { name: place.building, rooms: new Map<string, Room>() };
    buildings.set(place.building, building);
    const room = building.rooms.get(place.room) ?? { name: place.room, runs: [] };
    building.rooms.set(place.room, room);
    room.runs.push(run);
  }
  return [...buildings.values()];
}

// Waiting runs first, then newest first, then by id, so two runs
// started in one millisecond still hold their places between renders.
function byNeed(a: BoardRun, b: BoardRun): number {
  return Number(asking(b)) - Number(asking(a)) || (b.started ?? 0) - (a.started ?? 0) || a.run.localeCompare(b.run);
}

// A place holding a waiting run first, then by name.
export function placeByNeed(a: { readonly name: string; readonly runs: readonly BoardRun[] }, b: { readonly name: string; readonly runs: readonly BoardRun[] }): number {
  return Number(b.runs.some(asking)) - Number(a.runs.some(asking)) || a.name.localeCompare(b.name);
}

// The board's rows. A key in `folded` closes that building or room: its
// own row stays and everything beneath it leaves.
export function rowsOf(runs: readonly BoardRun[], folded: ReadonlySet<string>): readonly Row[] {
  const rows: Row[] = [];
  const buildings = grouped(runs)
    .map((building) => ({ name: building.name, rooms: [...building.rooms.values()], runs: [...building.rooms.values()].flatMap((room) => room.runs) }))
    .sort(placeByNeed);
  buildings.forEach((building, b) => {
    const lastBuilding = b === buildings.length - 1;
    const rooms = building.rooms.slice().sort(placeByNeed);
    const all = building.runs;
    rows.push({ kind: "building", key: building.name, guide: guide([], lastBuilding), name: building.name, asking: all.filter(asking).length, runs: all.length });
    if (folded.has(building.name)) return;
    rooms.forEach((room, r) => {
      const lastRoom = r === rooms.length - 1;
      const key = `${building.name}/${room.name}`;
      rows.push({ kind: "room", key, guide: guide([!lastBuilding], lastRoom), name: room.name });
      if (folded.has(key)) return;
      const ordered = room.runs.slice().sort(byNeed);
      ordered.forEach((run, n) => {
        rows.push({ kind: "run", key: run.run, guide: guide([!lastBuilding, !lastRoom], n === ordered.length - 1), run });
      });
    });
  });
  return rows;
}

// The rows a scrolled list draws: those the viewport shows and a margin
// either side, so a thousand runs cost the rows on screen. Before the
// first row has been measured `rowPx` is zero and the first `margin`
// rows are drawn.
export interface Window {
  readonly from: number;
  readonly to: number;
}

export function windowOf(total: number, scrolled: number, viewport: number, rowPx: number, margin: number): Window {
  if (rowPx <= 0) return { from: 0, to: Math.min(total, margin) };
  const from = Math.max(0, Math.floor(scrolled / rowPx) - margin);
  const to = Math.min(total, Math.ceil((scrolled + viewport) / rowPx) + margin);
  return { from, to };
}

// Where a row leads when it is opened: a run to its page, a room to its
// conversation, a building to its page.
export function viewOf(row: Row): Option.Option<View> {
  return Option.none();
}

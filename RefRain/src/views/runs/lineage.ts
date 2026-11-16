// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option, Schema } from "effect";

import type { RunBelief } from "../../core/belief";
import type { Doing } from "../../core/doing";
import type { View } from "../../core/route";
import { readRunId } from "../../core/run_id";
import { Address } from "../../wire";

export interface BoardRun {
  readonly run: string;
  readonly addr: string | null;
  readonly task: string | null;
  readonly goal: string | null;
  readonly started: number | null;
  readonly ended: number | null;
  readonly doing: Doing;
}

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

export type Phase = "model" | "tool" | "person" | "reply" | "idle" | "done" | "stopped" | "capped";

export function phaseOf(doing: Doing): Phase {
  switch (doing.kind) {
    case "thinking":
      return "model";
    case "calling":
      return "tool";
    case "waiting":
      return "person";
    case "awaiting_reply":
      return "reply";
    case "unknown":
      return "idle";
    case "frozen":
      return doing.completion === "cancelled"
        ? "stopped"
        : doing.completion === "limit"
          ? "capped"
          : "done";
  }
}

export function ended(phase: Phase): boolean {
  return phase === "done" || phase === "stopped" || phase === "capped";
}

export type Row =
  | {
      readonly kind: "building";
      readonly key: string;
      readonly guide: string;
      readonly name: string;
      readonly asking: number;
      readonly runs: number;
    }
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
    const building = buildings.get(place.building) ?? {
      name: place.building,
      rooms: new Map<string, Room>(),
    };
    buildings.set(place.building, building);
    const room = building.rooms.get(place.room) ?? { name: place.room, runs: [] };
    building.rooms.set(place.room, room);
    room.runs.push(run);
  }
  return [...buildings.values()];
}

function byNeed(a: BoardRun, b: BoardRun): number {
  return (
    Number(asking(b)) - Number(asking(a)) ||
    (b.started ?? 0) - (a.started ?? 0) ||
    a.run.localeCompare(b.run)
  );
}

export function placeByNeed(
  a: { readonly name: string; readonly runs: readonly BoardRun[] },
  b: { readonly name: string; readonly runs: readonly BoardRun[] },
): number {
  return Number(b.runs.some(asking)) - Number(a.runs.some(asking)) || a.name.localeCompare(b.name);
}

export function rowsOf(runs: readonly BoardRun[], folded: ReadonlySet<string>): readonly Row[] {
  const rows: Row[] = [];
  const buildings = grouped(runs)
    .map((building) => ({
      name: building.name,
      rooms: [...building.rooms.values()],
      runs: [...building.rooms.values()].flatMap((room) => room.runs),
    }))
    .sort(placeByNeed);
  buildings.forEach((building, b) => {
    const lastBuilding = b === buildings.length - 1;
    const rooms = building.rooms.slice().sort(placeByNeed);
    const all = building.runs;
    rows.push({
      kind: "building",
      key: building.name,
      guide: guide([], lastBuilding),
      name: building.name,
      asking: all.filter(asking).length,
      runs: all.length,
    });
    if (folded.has(building.name)) return;
    rooms.forEach((room, r) => {
      const lastRoom = r === rooms.length - 1;
      const key = `${building.name}/${room.name}`;
      rows.push({ kind: "room", key, guide: guide([!lastBuilding], lastRoom), name: room.name });
      if (folded.has(key)) return;
      const ordered = room.runs.slice().sort(byNeed);
      ordered.forEach((run, n) => {
        rows.push({
          kind: "run",
          key: run.run,
          guide: guide([!lastBuilding, !lastRoom], n === ordered.length - 1),
          run,
        });
      });
    });
  });
  return rows;
}

export interface Window {
  readonly from: number;
  readonly to: number;
}

export function windowOf(
  total: number,
  scrolled: number,
  viewport: number,
  rowPx: number,
  margin: number,
): Window {
  if (rowPx <= 0) return { from: 0, to: Math.min(total, margin) };
  const from = Math.max(0, Math.floor(scrolled / rowPx) - margin);
  const to = Math.min(total, Math.ceil((scrolled + viewport) / rowPx) + margin);
  return { from, to };
}

export function viewOf(row: Row): Option.Option<View> {
  switch (row.kind) {
    case "building":
      return Option.map(readAddress(row.key), (address): View => ({ kind: "building", address }));
    case "room":
      return Option.map(readAddress(row.key), (address): View => ({ kind: "talk", address }));
    case "run":
      return Option.map(readRunId(row.run.run), (run): View => ({ kind: "run", run }));
  }
}

const readAddress = Schema.decodeOption(Address);

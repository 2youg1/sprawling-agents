// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the skyline decides, with no DOM and no runes: where every
// tower stands, how tall it is, which windows are lit, what its labels
// say and how much of them fits, and the wiring that makes a tower a
// link. `skyline.look.svelte` is handed `SkylineLook` and draws it; a
// look built from another library draws the same city by taking the
// same value (client D95).

import type { RunBelief } from "../../core/belief";
import type { Doing } from "../../core/doing";
import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { fraction } from "../../core/share";
import type { Address, BuildingProgress, CityAnswer } from "../../wire";
import { squircle } from "./shape";

// The narrowest the skyline draws; a pane that holds it scrolls past this.
export const MIN_WIDTH = 880;

const W = 128;
const HALL_W = 196;
const GAP = 40;
const FLOOR = 24;
const COLS = 4;
const HALL_COLS = 6;
const SKY = 72;
const GROUND_DEPTH = 64;

// The size the two label lines under a tower are set in, in the
// drawing's own units; `fitted` counts glyphs against it.
const NAME_SIZE = 12;
const LINE_SIZE = 11;

// What one window says about the run behind it: none, ended, waiting
// for a person, calling a tool, or at work. A window's slot in the
// facade is its identity, because a window is built with the tower and
// is never replaced by its neighbour.
export type Pane = "dark" | "ended" | "waiting" | "calling" | "lit";

export interface CellLook {
  readonly slot: number;
  readonly x: number;
  readonly y: number;
  readonly pane: Pane;
}

// A mark the skyline draws about one run or one goal rather than about
// the building: a figure at the door for each run still moving, and a
// flag on the roof where a pursuit stands. The words are already said.
export type MarkLook =
  | {
      readonly kind: "figure";
      readonly x: number;
      readonly y: number;
      // Stagger between several figures at one door, in milliseconds.
      readonly delay: number;
      readonly posture: Doing["kind"];
      readonly label: string;
    }
  | { readonly kind: "flag"; readonly x: number; readonly y: number; readonly running: boolean; readonly label: string };

// A label under a tower: the whole of it, for the accessible name and
// the `<title>` a pointer shows, what of it fits under the tower, and
// the size it was fitted at, which is the size it is set in.
export interface Fitted {
  readonly whole: string;
  readonly shown: string;
  readonly size: number;
}

// Spread on the tower's group: a link a pointer or Enter follows.
export interface TowerWire {
  readonly role: "link";
  readonly tabindex: 0;
  readonly "aria-label": string;
  readonly onclick: () => void;
  readonly onkeydown: (event: { readonly key: string }) => void;
}

export interface TowerLook {
  readonly key: string;
  readonly hall: boolean;
  readonly picked: boolean;
  // A run is moving inside: the glow behind it and the lit door.
  readonly lit: boolean;
  readonly x: number;
  readonly w: number;
  readonly h: number;
  readonly top: number;
  readonly outline: string;
  readonly door: { readonly x: number; readonly y: number; readonly w: number; readonly h: number };
  readonly cells: readonly CellLook[];
  // Something is blocked: the lamp by the door, and its words.
  readonly lamp: string | null;
  readonly marks: readonly MarkLook[];
  // How much of the plan is done and how much is stuck, as the plinth
  // bar draws it. `null` for a building with no plan: no denominator,
  // so no share - rather than zero, which would read as a plan nobody
  // has started.
  readonly bar: { readonly done: number; readonly blocked: number } | null;
  readonly name: Fitted;
  readonly line: Fitted | null;
  readonly wire: TowerWire;
}

export interface StarLook {
  readonly slot: number;
  readonly x: number;
  readonly y: number;
  readonly r: number;
  readonly delay: number;
}

export interface SkylineLook {
  readonly label: string;
  readonly width: number;
  readonly height: number;
  readonly ground: number;
  readonly stars: readonly StarLook[];
  readonly towers: readonly TowerLook[];
}

// What the seat knows that this module cannot read for itself: the
// city's answer, the pick, the runs held under a building, the
// language, and the corner exponent the stylesheet states.
export interface SkylineSeat {
  readonly city: CityAnswer;
  readonly picked: Address | null;
  readonly runsIn: (addr: Address) => readonly RunBelief[];
  readonly lang: Lang;
  readonly power: number;
}

interface Slot {
  readonly building: BuildingProgress;
  readonly hall: boolean;
  readonly x: number;
  readonly w: number;
  readonly h: number;
  readonly floors: number;
  readonly cols: number;
  readonly runs: readonly RunBelief[];
}

export function lookOf(seat: SkylineSeat, onPick: (addr: Address) => void): SkylineLook {
  const placed = slotsOf(seat);
  const width = Math.max(MIN_WIDTH, ...placed.map((each) => each.x + each.w + 24));
  const tallest = Math.max(120, ...placed.map((each) => each.h));
  const ground = SKY + tallest + 48;
  return {
    label: say(seat.lang, "city_drawing"),
    width,
    height: ground + GROUND_DEPTH,
    ground,
    stars: starsOf(width),
    towers: placed.map((each) => towerOf(each, ground, seat, onPick)),
  };
}

// The hall in the middle, the others alternating to its right and left
// in name order, so the avenue reads outward from the centre.
function slotsOf(seat: SkylineSeat): Slot[] {
  const hall = seat.city.buildings.find((each) => each.addr === "hall");
  const others = seat.city.buildings
    .filter((each) => each.addr !== "hall")
    .sort((a, b) => a.addr.localeCompare(b.addr));
  const left: BuildingProgress[] = [];
  const right: BuildingProgress[] = [];
  others.forEach((each, index) => (index % 2 === 0 ? right : left).push(each));
  const row: { readonly building: BuildingProgress; readonly hall: boolean }[] = [
    ...left.reverse().map((building) => ({ building, hall: false })),
    ...(hall === undefined ? [] : [{ building: hall, hall: true }]),
    ...right.map((building) => ({ building, hall: false })),
  ];
  const span = row.reduce((sum, each) => sum + (each.hall ? HALL_W : W) + GAP, -GAP);
  let x = Math.max(24, (MIN_WIDTH - span) / 2);
  return row.map((each) => {
    const runs = seat.runsIn(each.building.addr);
    const floors = floorsFor(runs.length, each.hall);
    const w = each.hall ? HALL_W : W;
    const slot = {
      building: each.building,
      hall: each.hall,
      x,
      w,
      h: floors * FLOOR + 30,
      floors,
      cols: each.hall ? HALL_COLS : COLS,
      runs,
    };
    x += w + GAP;
    return slot;
  });
}

// How tall a building stands: two floors for an empty one, a floor per
// four runs it has held, never past eight.
function floorsFor(runs: number, hall: boolean): number {
  const base = hall ? 3 : 2;
  return Math.min(8, base + Math.ceil(runs / 4));
}

function towerOf(place: Slot, ground: number, seat: SkylineSeat, onPick: (addr: Address) => void): TowerLook {
  const { lang } = seat;
  const top = ground - place.h;
  const addr = place.building.addr;
  const active = place.runs.filter((run) => run.doing.kind !== "frozen");
  const bar = ratio(place.building);
  const stuck = place.building.blocked.length;
  const door = { x: place.x + place.w / 2 - 8, y: ground - 22, w: 16, h: 22 };
  const working = String(active.length);
  const name = place.hall ? say(lang, "city_hall") : String(addr);
  const line = lineOf(lang, place.runs.length, working, bar);
  const pursuit = seat.city.pursuits.find((each) => each.addr === addr);
  const room = place.w + GAP - 8;
  return {
    key: addr,
    hall: place.hall,
    picked: addr === seat.picked,
    lit: active.length > 0,
    x: place.x,
    w: place.w,
    h: place.h,
    top,
    outline: squircle({ x: place.x, y: top, w: place.w, h: place.h }, place.hall ? 10 : 6, seat.power),
    door,
    cells: cellsOf(place, top),
    lamp: stuck > 0 ? `${String(stuck)} ${say(lang, "status_blocked")}` : null,
    marks: [
      ...(pursuit === undefined
        ? []
        : [
            {
              kind: "flag" as const,
              x: place.x + place.w - 14,
              y: top - (place.hall ? 2 : 0),
              running: pursuit.state === "running",
              label: pursuit.goal,
            },
          ]),
      ...active.slice(0, 3).map((run, index) => ({
        kind: "figure" as const,
        x: door.x + door.w + 12 + index * 16,
        y: ground,
        delay: index * 450,
        posture: run.doing.kind,
        label: `${run.addr ?? ""} · ${postureWord(lang, run.doing)}`,
      })),
    ],
    bar,
    name: { whole: name, shown: fitted(name, room, NAME_SIZE), size: NAME_SIZE },
    line: line === null ? null : { whole: line, shown: fitted(line, room, LINE_SIZE), size: LINE_SIZE },
    wire: {
      role: "link",
      tabindex: 0,
      "aria-label": `${String(addr)} · ${working} ${say(lang, "city_at_work")}`,
      onclick: () => {
        onPick(addr);
      },
      onkeydown: (event) => {
        if (event.key === "Enter") onPick(addr);
      },
    },
  };
}

function cellsOf(place: Slot, top: number): CellLook[] {
  const cellW = (place.w - 24) / place.cols;
  // The hall's ground floor is its colonnade, so its windows stop a
  // floor short.
  const slots = (place.floors - (place.hall ? 1 : 0)) * place.cols;
  return Array.from({ length: slots }, (_, slot) => ({
    slot,
    x: place.x + 12 + (slot % place.cols) * cellW + cellW / 2 - 6,
    y: top + 18 + Math.floor(slot / place.cols) * FLOOR,
    pane: paneOf(place.runs[place.runs.length - 1 - slot]),
  }));
}

function paneOf(run: RunBelief | undefined): Pane {
  if (run === undefined) return "dark";
  switch (run.doing.kind) {
    case "frozen":
      return "ended";
    case "waiting":
      return "waiting";
    case "calling":
      return "calling";
    // A live run whose phase this page was never told, or one waiting
    // for another room's reply: the window is lit, without the blink
    // that says a model is calling.
    case "thinking":
    case "unknown":
    case "awaiting_reply":
      return "lit";
  }
}

function ratio(building: BuildingProgress): { done: number; blocked: number } | null {
  if ("planned" in building.progress) {
    const plan = building.progress.planned;
    return { done: fraction(plan.done_ppb), blocked: fraction(plan.blocked_ppb) };
  }
  return null;
}

// The second line under a tower: how many runs are at work and how far
// its plan has got, as much of that as the building has.
function lineOf(
  lang: Lang,
  runs: number,
  working: string,
  bar: { readonly done: number } | null,
): string | null {
  const counts = { active: working, runs: String(runs) };
  if (bar === null) return runs > 0 ? fill(say(lang, "city_tower_runs"), counts) : null;
  const percent = String(Math.round(bar.done * 100));
  if (runs === 0) return fill(say(lang, "city_done_percent"), { percent });
  return fill(say(lang, "city_tower_line"), { ...counts, percent });
}

// The posture a figure stands in, in words: the drawing encodes it in
// shapes and the label names it, so the state survives a forced-colours
// mode and a screen reader alike. Exhaustive over `Doing`, which is why
// a new posture cannot arrive unlabelled.
function postureWord(lang: Lang, held: Doing): string {
  switch (held.kind) {
    case "unknown":
      return say(lang, "city_at_work");
    case "thinking":
      return say(lang, "run_doing_thinking");
    case "calling":
      return say(lang, "run_doing_calling");
    case "waiting":
      return say(lang, "run_doing_waiting");
    case "awaiting_reply":
      return say(lang, "run_doing_awaiting_reply");
    case "frozen":
      return say(lang, "run_doing_frozen");
  }
}

// What of a label fits under its tower. A monospace glyph advances
// 0.6 em, so the room in glyphs is the tower's slot - the tower and the
// gap it shares with its neighbours - over that advance; a longer label
// would run into the next tower's name. A cut label ends in an ellipsis,
// and the whole of it stays in the tower's accessible name and in the
// `<title>` a pointer shows.
export function fitted(text: string, room: number, size: number): string {
  const fits = Math.max(1, Math.floor(room / (size * 0.6)));
  return text.length <= fits ? text : `${text.slice(0, fits - 1)}…`;
}

// A handful of stars, placed by a fixed hash so the sky is the same sky
// on every visit.
function starsOf(width: number): StarLook[] {
  const n = Math.max(12, Math.floor(width / 40));
  return Array.from({ length: n }, (_, i) => ({
    slot: i,
    x: 12 + (((i * 7919) % 1000) / 1000) * (width - 24),
    y: 8 + (((i * 104729) % 1000) / 1000) * (SKY - 20),
    r: 0.8 + ((i * 31) % 3) * 0.4,
    delay: (i * 137) % 1800,
  }));
}

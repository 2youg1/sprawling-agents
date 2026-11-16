// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunId } from "../../wire";
import type { Belief, RunBelief } from "./shape";
import { inside } from "./live";

export type Rooms = ReadonlyMap<string, readonly RunId[]>;

function earlier(left: RunBelief, right: RunBelief): number {
  return (left.started ?? 0) - (right.started ?? 0);
}

export function roomsOf(runs: Readonly<Record<string, RunBelief>>): Rooms {
  const rooms = new Map<string, RunBelief[]>();
  for (const run of Object.values(runs)) {
    if (run.addr === null) continue;
    const held = rooms.get(run.addr);
    if (held === undefined) rooms.set(run.addr, [run]);
    else held.push(run);
  }
  return new Map(
    [...rooms].map(([room, held]) => [room, held.sort(earlier).map((run) => run.run)]),
  );
}

export function roomed(held: Belief, was: RunBelief | undefined, run: RunBelief): Rooms {
  if (was?.addr === run.addr && was.started === run.started) return held.rooms;
  const rooms = new Map(held.rooms);
  const from = was?.addr ?? null;
  if (from !== null) {
    const left = (rooms.get(from) ?? []).filter((id) => id !== run.run);
    if (left.length === 0) rooms.delete(from);
    else rooms.set(from, left);
  }
  if (run.addr === null) return rooms;
  const others = rooms.get(run.addr) ?? [];
  const at = others.findIndex((id) => {
    const other = held.runs[id];
    return other !== undefined && earlier(run, other) < 0;
  });
  rooms.set(
    run.addr,
    at === -1 ? [...others, run.run] : [...others.slice(0, at), run.run, ...others.slice(at)],
  );
  return rooms;
}

export function heldIn(belief: Belief, room: string): RunBelief[] {
  return (belief.rooms.get(room) ?? []).flatMap((id) => {
    const run = belief.runs[id];
    return run === undefined ? [] : [run];
  });
}

export function heldWithin(belief: Belief, room: string): RunBelief[] {
  return [...belief.rooms.keys()]
    .filter((each) => inside(each, room))
    .flatMap((each) => heldIn(belief, each))
    .sort(earlier);
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which runs are working, as the one index every view reads. The runs
// that have not frozen are bounded by how many the city drives at once,
// far fewer than the runs a long city has held, so the index is kept
// forward one run at a time and a reader pays for the working runs
// rather than for the table (client D3).

import type { Readable } from "svelte/store";

import type { RunId } from "../../wire";
import type { Belief, RunBelief } from "./shape";

// Oldest start first, the order every reader wants: the newest working
// run is the last one.
function earlier(left: RunBelief, right: RunBelief): number {
  return (left.started ?? 0) - (right.started ?? 0);
}

function working(run: RunBelief): boolean {
  return run.doing.kind !== "frozen";
}

// The index built from the whole table, for a write that replaces the
// table.
export function liveOf(runs: Readonly<Record<string, RunBelief>>): readonly RunBelief[] {
  return Object.values(runs).filter(working).sort(earlier);
}

// The index after one run took a new reading: that run leaves it, and
// comes back in its place when it is still working.
export function livened(live: readonly RunBelief[], run: RunBelief): readonly RunBelief[] {
  const others = live.filter((each) => each.run !== run.run);
  if (!working(run)) return others.length === live.length ? live : others;
  const at = others.findIndex((each) => earlier(run, each) < 0);
  return at === -1 ? [...others, run] : [...others.slice(0, at), run, ...others.slice(at)];
}

// Whether a run is in this room or a room below it.
export function within(run: RunBelief, room: string): boolean {
  return run.addr !== null && inside(run.addr, room);
}

// Whether an address is this room or a room below it.
export function inside(addr: string, room: string): boolean {
  return addr === room || addr.startsWith(`${room}/`);
}

// The newest working run of one room: the run a message typed there
// steers, a `/stop` there reaches, and the room's page shows as going,
// so the box and the page cannot name two runs.
export function newestWorking(belief: Belief, room: string): RunBelief | undefined {
  return belief.live.reduce<RunBelief | undefined>(
    (newest, run) => (run.addr === room ? run : newest),
    undefined,
  );
}

// Calls `then` once, the first time the belief holds `run` frozen, and
// stops listening: what waits on a run's end - `/compact` opening the
// session that carries its handoff - acts on the record, not on a guess.
export function onceFrozen(belief: Readable<Belief>, run: RunId, then: () => void): void {
  // A store answers at once when subscribed, before `subscribe` has
  // returned the way to stop: an end already held is stopped after it.
  const waiting: { done: boolean; stop: (() => void) | null } = { done: false, stop: null };
  const stop = belief.subscribe((held) => {
    if (waiting.done || held.runs[run]?.doing.kind !== "frozen") return;
    waiting.done = true;
    waiting.stop?.();
  });
  if (waiting.done) stop();
  else waiting.stop = stop;
}

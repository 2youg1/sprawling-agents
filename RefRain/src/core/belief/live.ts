// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Readable } from "../store";

import type { RunId } from "../../wire";
import type { Belief, RunBelief } from "./shape";

function earlier(left: RunBelief, right: RunBelief): number {
  return (left.started ?? 0) - (right.started ?? 0);
}

function working(run: RunBelief): boolean {
  return run.doing.kind !== "frozen";
}

export function liveOf(runs: Readonly<Record<string, RunBelief>>): readonly RunBelief[] {
  return Object.values(runs).filter(working).sort(earlier);
}

export function livened(live: readonly RunBelief[], run: RunBelief): readonly RunBelief[] {
  const others = live.filter((each) => each.run !== run.run);
  if (!working(run)) return others.length === live.length ? live : others;
  const at = others.findIndex((each) => earlier(run, each) < 0);
  return at === -1 ? [...others, run] : [...others.slice(0, at), run, ...others.slice(at)];
}

export function within(run: RunBelief, room: string): boolean {
  return run.addr !== null && inside(run.addr, room);
}

export function inside(addr: string, room: string): boolean {
  return addr === room || addr.startsWith(`${room}/`);
}

export function newestWorking(belief: Belief, room: string): RunBelief | undefined {
  return belief.live.reduce<RunBelief | undefined>(
    (newest, run) => (run.addr === room ? run : newest),
    undefined,
  );
}

export function onceFrozen(belief: Readable<Belief>, run: RunId, then: () => void): void {
  const waiting: { done: boolean; stop: (() => void) | null } = { done: false, stop: null };
  const stop = belief.subscribe((held) => {
    if (waiting.done || held.runs[run]?.doing.kind !== "frozen") return;
    waiting.done = true;
    waiting.stop?.();
    then();
  });
  if (waiting.done) stop();
  else waiting.stop = stop;
}

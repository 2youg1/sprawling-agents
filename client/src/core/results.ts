// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What results-only mode keeps, decided once for the room and the city.
//
// **The room keeps what a person reads for.** Their own lines, the
// answers, the dividers where a run ended and what it produced; the
// calls and the reasoning that led there are the work, not the result,
// and in this mode they are not mounted at all rather than folded, so a
// thread of a thousand calls costs the page nothing for them.
//
// **The city keeps three outcomes and only the first of each.** What
// waits for the person, what failed and what finished, newest first,
// each cut to `first` rows with the whole count beside it: a city of
// thousands of runs draws the same few dozen rows as a city of ten.

import type { RunBelief } from "./belief/shape";

// How much of a conversation or a city is drawn. `whole` is the
// posture this client ships with.
export type Showing = "whole" | "results";
export const SHOWINGS: readonly Showing[] = ["whole", "results"];

// The three outcomes a person acts on, in the order they deal with
// them. A run still working, and a run the person cancelled themselves,
// is none of these: the first has no result yet and the second is a
// result the person already knows.
export type Outcome = "waiting" | "failed" | "done";
export const OUTCOMES: readonly Outcome[] = ["waiting", "failed", "done"];

// How many runs of one outcome the city draws before "n more".
export const FIRST = 5;

export interface Group {
  readonly outcome: Outcome;
  // The newest `first` runs of this outcome, newest first.
  readonly first: readonly RunBelief[];
  readonly total: number;
}

export function drawsCalls(showing: Showing): boolean {
  return showing === "whole";
}

// Which of the three a run is in, or nothing when it is in none.
export function outcomeOf(run: RunBelief): Outcome | null {
  const doing = run.doing;
  switch (doing.kind) {
    case "waiting":
      return "waiting";
    case "frozen":
      switch (doing.completion) {
        case "done":
          return "done";
        case "cancelled":
          return null;
        // A run that ended without naming how did not report done.
        case null:
        default:
          return "failed";
      }
    case "unknown":
    case "thinking":
    case "calling":
      return null;
  }
}

// One pass sorts the city into the three outcomes; only the runs of an
// outcome are then ordered, and only its first `first` are kept.
export function resultsOf(runs: Iterable<RunBelief>, first: number): readonly Group[] {
  const held: Record<Outcome, RunBelief[]> = { waiting: [], failed: [], done: [] };
  for (const run of runs) {
    const outcome = outcomeOf(run);
    if (outcome !== null) held[outcome].push(run);
  }
  return OUTCOMES.map((outcome) => ({
    outcome,
    first: held[outcome].sort(newestFirst).slice(0, first),
    total: held[outcome].length,
  }));
}

// A run with no start time is placed after every run that has one.
function newestFirst(a: RunBelief, b: RunBelief): number {
  return (b.started ?? -1) - (a.started ?? -1);
}

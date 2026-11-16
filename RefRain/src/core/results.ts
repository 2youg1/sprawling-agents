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
import type { FileChange } from "../wire";

// How much of a conversation or a city is drawn. `whole` is the
// posture this client ships with.
export type Showing = "whole" | "results";
export const SHOWINGS: readonly Showing[] = ["whole", "results"];

// The three outcomes a person acts on, in the order they deal with
// them. A run still working, and a run the person cancelled themselves,
// is none of these: the first has no result yet and the second is a
// result the person already knows.
// A run that froze without naming how is "ended": neither the stream nor
// the city answer saw its `run_frozen` record, and calling it failed
// would claim what nobody reported.
export type Outcome = "waiting" | "failed" | "done" | "ended";
export const OUTCOMES: readonly Outcome[] = ["waiting", "failed", "done", "ended"];

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
        case null:
          return "ended";
        default:
          return "failed";
      }
    case "unknown":
    case "thinking":
    case "calling":
    case "awaiting_reply":
      return null;
  }
}

// One pass sorts the city into the four outcomes; only the runs of an
// outcome are then ordered, and only its first `first` are kept.
export function resultsOf(runs: Iterable<RunBelief>, first: number): readonly Group[] {
  const held: Record<Outcome, RunBelief[]> = { waiting: [], failed: [], done: [], ended: [] };
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

// What a finished run changed, as the one line under its outcome
// divider states it. A binary file is a file that changed but adds no
// lines: counting it as `+0 −0` would report a measurement nobody made.
export interface Produced {
  readonly files: number;
  readonly added: number;
  readonly removed: number;
}

export function producedOf(files: readonly FileChange[]): Produced {
  let added = 0;
  let removed = 0;
  for (const file of files) {
    if (typeof file.lines === "string") continue;
    added += file.lines.counted.added;
    removed += file.lines.counted.removed;
  }
  return { files: files.length, added, removed };
}

// How recent a row of the results city is. The list is read by time,
// newest first, and cut where a person's sense of "just now" changes:
// the last ten minutes, the last hour, everything before.
export type Recency = "minutes" | "hour" | "earlier";
export const RECENCIES: readonly Recency[] = ["minutes", "hour", "earlier"];

export interface Band {
  readonly recency: Recency;
  readonly runs: readonly RunBelief[];
}

const MINUTES_MS = 10 * 60_000;
const HOUR_MS = 60 * 60_000;

// The rows sorted newest first, then cut into bands. A run with no
// start time has no age to read and falls in "earlier", after the rest.
export function bandsOf(runs: readonly RunBelief[], now: number): readonly Band[] {
  const held: Record<Recency, RunBelief[]> = { minutes: [], hour: [], earlier: [] };
  for (const run of [...runs].sort(newestFirst)) held[recencyOf(run.started, now)].push(run);
  return RECENCIES.filter((recency) => held[recency].length > 0).map((recency) => ({
    recency,
    runs: held[recency],
  }));
}

function recencyOf(started: number | null, now: number): Recency {
  if (started === null) return "earlier";
  const age = now - started;
  if (age < MINUTES_MS) return "minutes";
  return age < HOUR_MS ? "hour" : "earlier";
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunBelief } from "./belief/shape";
import type { FileChange } from "../wire";

export type Showing = "whole" | "results";
export const SHOWINGS: readonly Showing[] = ["whole", "results"];

export type Outcome = "waiting" | "failed" | "done" | "ended";
export const OUTCOMES: readonly Outcome[] = ["waiting", "failed", "done", "ended"];

export const FIRST = 5;

export interface Group {
  readonly outcome: Outcome;
  readonly first: readonly RunBelief[];
  readonly total: number;
}

export function drawsCalls(showing: Showing): boolean {
  return showing === "whole";
}

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

function newestFirst(a: RunBelief, b: RunBelief): number {
  return (b.started ?? -1) - (a.started ?? -1);
}

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

export type Recency = "minutes" | "hour" | "earlier";
export const RECENCIES: readonly Recency[] = ["minutes", "hour", "earlier"];

export interface Band {
  readonly recency: Recency;
  readonly runs: readonly RunBelief[];
}

const MINUTES_MS = 10 * 60_000;
const HOUR_MS = 60 * 60_000;

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

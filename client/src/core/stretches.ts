// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The sessions this page can list, one row each: every stretch of every
// room the city answered for (`Query::Sessions`, wire §8-71), which runs
// of it this page holds, which one is the room's current session, and
// how the sessions pane groups them - the pinned first, then by building.
//
// **A room has one current session, its newest stretch**: the city only
// goes on with that one (`docs/glossary.md`, Session), and opening a new
// one ends it, so at most one session of the Mayor is ever current. An
// older stretch is a record, and going on from it is a branch from its
// tail (`tailOf`), which is what `/fork` sends.

import type { RunBelief } from "./belief";
import { MAYOR, buildingOf } from "./route";
import { PIN } from "./tags";
import type { Address, Origin, Seq, SessionLine, SessionsAnswer, Tag } from "../wire";

export interface Stretch {
  readonly room: Address;
  readonly line: SessionLine;
  // The room's newest stretch: the one a message there goes on with.
  readonly current: boolean;
  // The runs of this stretch this page holds, oldest first.
  readonly runs: readonly RunBelief[];
}

// Every stretch the answers name, newest activity first. A run belongs to
// the stretch its last line falls in: no stretch opens under a working
// run, so a run never straddles two.
export function stretchesOf(
  answers: readonly SessionsAnswer[],
  runsIn: (room: Address) => readonly RunBelief[],
): Stretch[] {
  const all = answers.flatMap((answer) => {
    const runs = runsIn(answer.room);
    return answer.sessions.map((line, at): Stretch => {
      const next = answer.sessions[at - 1];
      return {
        room: answer.room,
        line,
        current: at === 0,
        runs: next === undefined ? runs : [],
      };
    });
  });
  return all.sort((a, b) => b.line.at - a.line.at);
}

// The stretch a route names: the room's current one when it names none or
// names the current one, and `null` while the answer has not arrived or
// does not hold it.
export function lineIn(answer: SessionsAnswer | undefined, asked: Seq | undefined): SessionLine | null {
  const lines = answer?.sessions ?? [];
  return lines[0] ?? null;
}

// Why a row stands in the pinned group: the Mayor's current session
// always does, and that pin is derived rather than stored; any other row
// because the person tagged it `pin`.
export type Pinning = "mayor" | "tagged" | "none";

export function pinningOf(stretch: Stretch, tags: readonly Tag[]): Pinning {
  return tags.includes(PIN) ? "tagged" : "none";
}

export type Group =
  | { readonly kind: "pinned"; readonly rows: readonly Stretch[] }
  | { readonly kind: "building"; readonly building: Address; readonly rows: readonly Stretch[] };

// The pane's groups: the pinned first, then each building in the order of
// its newest session, every group newest first. With a filter only the
// rows carrying that tag stay, and a group left empty is not drawn.
export function grouped(
  stretches: readonly Stretch[],
  tagsFor: (stretch: Stretch) => readonly Tag[],
  filter: Tag | null,
): Group[] {
  const kept = stretches;
  const pinned = kept.filter((stretch) => pinningOf(stretch, tagsFor(stretch)) !== "none");
  const rest = kept.filter((stretch) => !pinned.includes(stretch));
  const buildings = [...new Set(rest.map((stretch) => buildingOf(stretch.room)))];
  return [
    ...(pinned.length === 0 ? [] : [{ kind: "pinned" as const, rows: pinned }]),
    ...buildings.map((building) => ({
      kind: "building" as const,
      building,
      rows: rest.filter((stretch) => buildingOf(stretch.room) === building),
    })),
  ];
}

// Where going on from a stretch branches: the last line of its last run.
// `null` when this page holds none of its runs.
export function tailOf(stretch: Stretch): Origin | null {
  const last = stretch.runs.at(-1);
  return last === undefined ? null : { run: last.run, at_seq: stretch.line.began };
}

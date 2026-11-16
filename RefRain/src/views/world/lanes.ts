// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { GitOid } from "../../wire";

export interface Commit {
  readonly oid: GitOid;
  readonly parents?: readonly GitOid[] | null | undefined;
}

export interface Line {
  readonly from: number;
  readonly to: number;
  readonly part: "through" | "in" | "out";

  readonly owner: number;
}

export interface Placed {
  readonly lane: number;
  readonly lines: readonly Line[];
}

export interface Graph {
  readonly rows: readonly Placed[];

  readonly lanes: number;
}

interface Waiting {
  readonly oid: GitOid;
  readonly owner: number;
}

export function graphOf(commits: readonly Commit[]): Graph {
  let open: (Waiting | null)[] = [];
  let lanes = 0;
  const rows = commits.map((commit, row): Placed => {
    const top = open;
    const found = top.findIndex((slot) => slot?.oid === commit.oid);
    const lane = found >= 0 ? found : freeIn(top);
    const lines: Line[] = top.flatMap((slot, at): Line[] => {
      if (slot === null) return [];
      return slot.oid === commit.oid
        ? [{ from: at, to: lane, part: "in", owner: slot.owner }]
        : [{ from: at, to: at, part: "through", owner: slot.owner }];
    });
    const next = top.map((slot) => (slot?.oid === commit.oid ? null : slot));
    const [first, ...others] = commit.parents ?? [];
    setAt(next, lane, first === undefined ? null : { oid: first, owner: row });
    if (first !== undefined) lines.push({ from: lane, to: lane, part: "out", owner: row });
    for (const parent of others) {
      const waiting = next.findIndex((slot) => slot?.oid === parent);
      const to = waiting >= 0 ? waiting : freeIn(next);
      setAt(next, to, { oid: parent, owner: row });
      lines.push({ from: lane, to, part: "out", owner: row });
    }
    open = trimmed(next);
    lanes = Math.max(lanes, lane + 1, top.length, open.length);
    return { lane, lines };
  });
  return { rows, lanes };
}

function freeIn(slots: readonly (Waiting | null)[]): number {
  const free = slots.indexOf(null);
  return free >= 0 ? free : slots.length;
}

function setAt(slots: (Waiting | null)[], at: number, value: Waiting | null): void {
  while (slots.length <= at) slots.push(null);
  slots.splice(at, 1, value);
}

function trimmed(slots: readonly (Waiting | null)[]): (Waiting | null)[] {
  let end = slots.length;
  while (end > 0 && slots[end - 1] === null) end -= 1;
  return slots.slice(0, end);
}

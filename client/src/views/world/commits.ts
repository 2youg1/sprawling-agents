// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the commits pane's look is given (`commits.look.svelte`), and how
// the seat (`commits.svelte`) reads a page of commits into it
// (client/Spec.lean §7K): one row per commit with its piece of the
// swimlane graph (`./lanes.ts`) in the drawing's own pixels, the node's
// phase, and - under the picked row - the commit's identity written
// whole.

import type { Phase } from "../runs/lineage";
import type { CommitAnswer, GitOid } from "../../wire";
import type { Graph, Line } from "./lanes";

// An oid as git prints it short.
export const SHORT = 7;
// One lane's width and the row's height, in the drawing's own pixels.
const LANE = 14;
export const HEIGHT = 44;
const MID = HEIGHT / 2;

const x = (lane: number): number => LANE / 2 + lane * LANE;

// One line of a row's piece of the graph, as an SVG path.
export function pathOf(line: Line): string {
  const [x1, x2] = [x(line.from), x(line.to)];
  switch (line.part) {
    case "through":
      return `M${String(x1)} 0V${String(HEIGHT)}`;
    case "in":
      return `M${String(x1)} 0C${String(x1)} ${String(MID / 2)} ${String(x2)} ${String(MID / 2)} ${String(x2)} ${String(MID)}`;
    case "out":
      return `M${String(x1)} ${String(MID)}C${String(x1)} ${String(MID * 1.5)} ${String(x2)} ${String(MID * 1.5)} ${String(x2)} ${String(HEIGHT)}`;
  }
}

// The bag spread on a row's button: it picks the commit, and says
// whether its identity is open under it.
export interface CommitWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

// The picked commit's identity: each value whole, so a copy takes it.
export interface Facts {
  readonly oid: GitOid;
  readonly b3: string | undefined;
  readonly parents: string;
}

export interface CommitRow {
  readonly key: GitOid;
  readonly wire: CommitWire;
  readonly picked: boolean;
  // Whether the room the conversation is in made it; the others are
  // quieter.
  readonly here: boolean;
  // The row's piece of the graph: each line, and whether it is the
  // chosen session's lane.
  readonly lines: readonly { readonly d: string; readonly mine: boolean }[];
  // The node: where it stands, the phase of the run that made it, and
  // whether that run is the chosen session's.
  readonly node: { readonly left: number; readonly phase: Phase; readonly mine: boolean };
  readonly short: string;
  readonly message: string;
  readonly room: string;
  readonly facts: Facts | undefined;
}

export interface CommitsLook {
  // The graph's width in the drawing's own pixels, and a row's height.
  readonly width: number;
  readonly height: number;
  readonly rows: readonly CommitRow[];
  // The words of the identity's three labels, already in the person's
  // language.
  readonly labels: { readonly oid: string; readonly b3: string; readonly parents: string };
  // The link to the building's page while older commits are left out.
  readonly more: { readonly href: string; readonly word: string } | undefined;
}

export interface CommitInput {
  readonly commit: CommitAnswer;
  readonly graph: Graph;
  readonly row: number;
  readonly picked: boolean;
  readonly here: boolean;
  readonly phase: Phase;
  // Whether a line's owner, or the commit itself, is the chosen
  // session's run.
  readonly mine: (row: number) => boolean;
  readonly room: string;
  readonly pick: (commit: CommitAnswer) => void;
}

export function rowOf(input: CommitInput): CommitRow {
  const { commit, picked } = input;
  const placed = input.graph.rows[input.row];
  return {
    key: commit.oid,
    wire: {
      type: "button",
      "aria-expanded": picked,
      onclick: () => {
        input.pick(commit);
      },
    },
    picked,
    here: input.here,
    lines: (placed?.lines ?? []).map((line) => ({ d: pathOf(line), mine: input.mine(line.owner) })),
    node: { left: x(placed?.lane ?? 0), phase: input.phase, mine: input.mine(input.row) },
    short: commit.oid.slice(0, SHORT),
    message: commit.message ?? "",
    room: input.room,
    facts: picked
      ? {
          oid: commit.oid,
          b3: commit.b3 ?? undefined,
          parents: (commit.parents ?? []).map((oid) => oid.slice(0, SHORT)).join(" · ") || "—",
        }
      : undefined,
  };
}

export function widthOf(graph: Graph): number {
  return Math.max(1, graph.lanes) * LANE;
}

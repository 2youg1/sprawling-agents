// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { isoInstant, moment } from "../../core/time";
import type { Phase } from "../runs/lineage";
import type { CommitAnswer, GitOid } from "../../wire";
import type { Graph, Line } from "./lanes";

export const SHORT = 7;

const LANE = 14;
export const HEIGHT = 44;
const MID = HEIGHT / 2;

const x = (lane: number): number => LANE / 2 + lane * LANE;

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

export interface CommitWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

export interface Facts {
  readonly oid: GitOid;
  readonly b3: string | undefined;
  readonly parents: string;
  readonly at: { readonly text: string; readonly iso: string };
}

export interface CommitRow {
  readonly key: GitOid;
  readonly wire: CommitWire;
  readonly picked: boolean;

  readonly here: boolean;

  readonly lines: readonly { readonly d: string; readonly mine: boolean }[];

  readonly node: { readonly left: number; readonly phase: Phase; readonly mine: boolean };
  readonly short: string;
  readonly message: string;
  readonly room: string;
  readonly facts: Facts | undefined;
}

export interface CommitsLook {
  readonly width: number;
  readonly height: number;
  readonly rows: readonly CommitRow[];

  readonly labels: {
    readonly oid: string;
    readonly b3: string;
    readonly parents: string;
    readonly at: string;
  };

  readonly more: { readonly href: string; readonly word: string } | undefined;
}

export interface CommitInput {
  readonly commit: CommitAnswer;
  readonly graph: Graph;
  readonly row: number;
  readonly picked: boolean;
  readonly here: boolean;
  readonly phase: Phase;

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
          at: { text: moment(commit.at), iso: isoInstant(commit.at) },
        }
      : undefined,
  };
}

export function widthOf(graph: Graph): number {
  return Math.max(1, graph.lanes) * LANE;
}

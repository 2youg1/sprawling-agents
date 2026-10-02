// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the panorama tier's workbench is arranged (client-SPEC 7K, 12-24):
// which of its three panes stands where, and how many of the shell's
// twelve columns each one takes.
//
// **Widths are counted in grid columns, never in pixels.** A pane edge
// that falls between two column lines is an edge no other region of the
// page can line up with, so a drag snaps to a column line and the value
// kept is the count. Three counts that always add up to twelve, each at
// least `NARROWEST`, is the whole invariant, and every change below
// keeps it rather than a reader checking it afterwards.

export type Pane = "sessions" | "session" | "commits";

export interface Column {
  readonly pane: Pane;
  // How many of the shell's columns the pane takes.
  readonly span: number;
}

// The three panes, from left to right.
export type Workbench = readonly [Column, Column, Column];

const COLUMNS = 12;

// No pane is narrower than two columns: one column is about a hundred
// pixels at the narrowest window that draws the workbench, which is a
// status dot and half a name.
export const NARROWEST = 2;

export const WORKBENCH: Workbench = [
  { pane: "sessions", span: 3 },
  { pane: "session", span: 5 },
  { pane: "commits", span: 4 },
];

// The line between the first and second pane, or between the second
// and third.
export type Divider = 0 | 1;

export type Side = "left" | "right";

// The most columns the pane before `divider` may take: the pair it
// shares with the pane after, less the narrowest that one may be.
export function widest(bench: Workbench, divider: Divider): number {
  const [before, after] = pairAt(bench, divider);
  return before.span + after.span - NARROWEST;
}

// The pane before `divider` set to `span` columns, rounded to a column
// line and clamped so neither pane of the pair is narrower than
// `NARROWEST`; the pane after takes the rest of the pair, so the third
// pane never moves.
export function resized(bench: Workbench, divider: Divider, span: number): Workbench {
  const [before, after] = pairAt(bench, divider);
  const pair = before.span + after.span;
  const held = Math.min(widest(bench, divider), Math.max(NARROWEST, Math.round(span)));
  const next: readonly [Column, Column] = [
    { pane: before.pane, span: held },
    { pane: after.pane, span: pair - held },
  ];
  return divider === 0 ? [next[0], next[1], bench[2]] : [bench[0], next[0], next[1]];
}

// `pane` swapped with its neighbour on `side`, its width going with it.
// A pane already at that edge stays where it is.
export function moved(bench: Workbench, pane: Pane, side: Side): Workbench {
  const at = bench.findIndex((column) => column.pane === pane);
  // The left pane of the pair that trades places.
  const left = side === "left" ? at - 1 : at;
  if (left === 0) return [bench[1], bench[0], bench[2]];
  if (left === 1) return [bench[0], bench[2], bench[1]];
  return bench;
}

function pairAt(bench: Workbench, divider: Divider): readonly [Column, Column] {
  return divider === 0 ? [bench[0], bench[1]] : [bench[1], bench[2]];
}

// The row a browser keeps: each pane and its width, left to right.
export function spelledWorkbench(bench: Workbench): string {
  return bench.map((column) => `${column.pane}:${String(column.span)}`).join(" ");
}

const PANES: readonly Pane[] = ["sessions", "session", "commits"];

// A kept row read back, or the shipped arrangement when the row is
// anything `spelledWorkbench` could not have written: a guess this
// build cannot read is dropped rather than repaired.
export function readWorkbench(raw: string | null): Workbench {
  const words = (raw ?? "").split(" ");
  const read = words.flatMap((word): Column[] => {
    const [name, width] = word.split(":");
    const pane = PANES.find((each) => each === name);
    const span = Number(width);
    return pane === undefined || !Number.isInteger(span) || span < NARROWEST ? [] : [{ pane, span }];
  });
  const [first, second, third] = read;
  if (words.length !== 3 || first === undefined || second === undefined || third === undefined) return WORKBENCH;
  const whole = new Set(read.map((column) => column.pane)).size === 3;
  return whole && first.span + second.span + third.span === COLUMNS ? [first, second, third] : WORKBENCH;
}

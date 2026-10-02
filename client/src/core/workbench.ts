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

export function widest(bench: Workbench, divider: Divider): number {
  return NARROWEST;
}

export function resized(bench: Workbench, divider: Divider, span: number): Workbench {
  return bench;
}

export function moved(bench: Workbench, pane: Pane, side: Side): Workbench {
  return bench;
}

export function spelledWorkbench(bench: Workbench): string {
  return "";
}

export function readWorkbench(raw: string | null): Workbench {
  return WORKBENCH;
}

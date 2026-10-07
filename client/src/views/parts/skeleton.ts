// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The shape of what is coming, decided before it is drawn: how many
// bars, how wide each one is, and the one thing a screen reader is told.
// The bars mean nothing to a screen reader, so the block announces
// itself once with the caller's word and hides the rest
// (`client/spec/Views/Parts.lean`, the skeleton row of §7-1).

export interface SkeletonProps {
  // What a screen reader is told is loading, in the person's language.
  readonly label: string;
  readonly rows?: number;
  // A block whose rows are one taller line each, the way a list of rows
  // loads, rather than a paragraph.
  readonly tall?: boolean;
}

// The widths a line of text takes, so a block of them reads as prose
// rather than as a chart. Repeated for as many rows as are asked for.
export type Width = "whole" | "five-sixths" | "two-thirds" | "three-quarters";

const WIDTHS: readonly [Width, ...Width[]] = ["whole", "five-sixths", "two-thirds", "three-quarters"];

// Spread on the block.
export interface BlockWire {
  readonly role: "status";
  readonly "aria-label": string;
  readonly "aria-busy": "true";
}

// Spread on each bar.
export interface BarWire {
  readonly "aria-hidden": "true";
}

export interface BarLook {
  // A skeleton bar's whole identity is the position it stands in for -
  // bar three stands for row three - so the slot is the honest key: it
  // is what makes a change in `rows` grow and shrink the block from its
  // far end instead of rebuilding it.
  readonly slot: number;
  readonly width: Width;
  readonly wire: BarWire;
}

export interface SkeletonLook {
  readonly block: BlockWire;
  readonly tall: boolean;
  readonly bars: readonly BarLook[];
}

export function lookOf({ label, rows = 3, tall = false }: SkeletonProps): SkeletonLook {
  return {
    block: { role: "status", "aria-label": label, "aria-busy": "true" },
    tall,
    bars: Array.from({ length: Math.max(rows, 1) }, (_unused, slot) => ({
      slot,
      width: WIDTHS[slot % WIDTHS.length] ?? WIDTHS[0],
      wire: { "aria-hidden": "true" },
    })),
  };
}

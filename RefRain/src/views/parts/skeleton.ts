// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface SkeletonProps {
  readonly label: string;
  readonly rows?: number;

  readonly tall?: boolean;
}

export type Width = "whole" | "five-sixths" | "two-thirds" | "three-quarters";

const WIDTHS: readonly [Width, ...Width[]] = [
  "whole",
  "five-sixths",
  "two-thirds",
  "three-quarters",
];

export interface BlockWire {
  readonly role: "status";
  readonly "aria-label": string;
  readonly "aria-busy": "true";
}

export interface BarWire {
  readonly "aria-hidden": "true";
}

export interface BarLook {
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

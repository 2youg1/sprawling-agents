// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type Pane = "sessions" | "session" | "commits";

export interface Column {
  readonly pane: Pane;

  readonly span: number;
}

export type Workbench = readonly [Column, Column, Column];

const COLUMNS = 12;

export const NARROWEST = 2;

export const WORKBENCH: Workbench = [
  { pane: "sessions", span: 3 },
  { pane: "session", span: 6 },
  { pane: "commits", span: 3 },
];

export type Divider = 0 | 1;

export type Side = "left" | "right";

export function widest(bench: Workbench, divider: Divider): number {
  const [before, after] = pairAt(bench, divider);
  return before.span + after.span - NARROWEST;
}

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

export function moved(bench: Workbench, pane: Pane, side: Side): Workbench {
  const at = bench.findIndex((column) => column.pane === pane);

  const left = side === "left" ? at - 1 : at;
  if (left === 0) return [bench[1], bench[0], bench[2]];
  if (left === 1) return [bench[0], bench[2], bench[1]];
  return bench;
}

function pairAt(bench: Workbench, divider: Divider): readonly [Column, Column] {
  return divider === 0 ? [bench[0], bench[1]] : [bench[1], bench[2]];
}

export function spelledWorkbench(bench: Workbench): string {
  return bench.map((column) => `${column.pane}:${String(column.span)}`).join(" ");
}

const PANES: readonly Pane[] = ["sessions", "session", "commits"];

export function readWorkbench(raw: string | null): Workbench {
  const words = (raw ?? "").split(" ");
  const read = words.flatMap((word): Column[] => {
    const [name, width] = word.split(":");
    const pane = PANES.find((each) => each === name);
    const span = Number(width);
    return pane === undefined || !Number.isInteger(span) || span < NARROWEST
      ? []
      : [{ pane, span }];
  });
  const [first, second, third] = read;
  if (words.length !== 3 || first === undefined || second === undefined || third === undefined)
    return WORKBENCH;
  const whole = new Set(read.map((column) => column.pane)).size === 3;
  return whole && first.span + second.span + third.span === COLUMNS
    ? [first, second, third]
    : WORKBENCH;
}

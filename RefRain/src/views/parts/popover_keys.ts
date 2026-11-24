// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import type { Side } from "./layer";
import type { PopoverColumn, PopoverRow } from "./popover";

export type Layout = "content" | "equal";

export interface Place {
  readonly column: number;
  readonly cursor: number;
}

export type Pressed =
  | { readonly kind: "move"; readonly to: Place }
  | { readonly kind: "apply" }
  | { readonly kind: "close" }
  | { readonly kind: "pass" };

export interface Keyed {
  readonly key: string;
  readonly shiftKey: boolean;
  preventDefault(): void;
}

const PASS: Pressed = { kind: "pass" };

export function pressed(place: Place, key: Keyed, rows: readonly number[]): Pressed {
  const here = rows.at(place.column) ?? 0;
  const walk = (by: number): Pressed =>
    here === 0
      ? { kind: "move", to: place }
      : {
          kind: "move",
          to: { column: place.column, cursor: Math.min(here - 1, Math.max(0, place.cursor + by)) },
        };

  const turn = (by: number): Pressed =>
    rows.length === 0
      ? { kind: "move", to: place }
      : {
          kind: "move",
          to: { column: (place.column + by + rows.length) % rows.length, cursor: 0 },
        };
  switch (key.key) {
    case "ArrowDown":
      return walk(1);
    case "ArrowUp":
      return walk(-1);
    case "Home":
      return { kind: "move", to: { column: place.column, cursor: 0 } };
    case "End":
      return { kind: "move", to: { column: place.column, cursor: Math.max(0, here - 1) } };
    case "Tab":
      return turn(key.shiftKey ? -1 : 1);
    case "Enter":
      return { kind: "apply" };
    case "Escape":
      return { kind: "close" };
    default:
      return PASS;
  }
}

export const listId = (seat: string, column: number): string => `${seat}-c${String(column)}`;
export const rowId = (seat: string, column: number, index: number): string =>
  `${seat}-r${String(column)}-${String(index)}`;

export interface PopoverView {
  readonly seat: string;
  readonly layout: Layout;
  readonly columns: readonly PopoverColumn[];
  readonly place: Place;

  readonly holder: "list" | "caller";
  readonly side: Side;

  readonly room: number | null;

  readonly title: Key;

  readonly say: (key: Key) => string;
}

export interface PopoverActions {
  readonly place: (to: Place) => void;
  readonly apply: (column: PopoverColumn, row: PopoverRow) => void;
  readonly close: () => void;
}

export function keysOf(view: PopoverView, hands: PopoverActions): (key: Keyed) => boolean {
  return (key) => {
    const answer = pressed(
      view.place,
      key,
      view.columns.map((pane) => pane.rows.length),
    );
    switch (answer.kind) {
      case "move":
        hands.place(answer.to);
        return true;
      case "apply": {
        const pane = view.columns.at(view.place.column);
        const row = pane?.rows.at(view.place.cursor);
        if (pane !== undefined && row !== undefined) hands.apply(pane, row);
        return true;
      }
      case "close":
        hands.close();
        return true;
      case "pass":
        return false;
    }
  };
}

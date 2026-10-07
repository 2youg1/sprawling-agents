// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a popover decides before anything is drawn - where each key
// moves the cursor, which ids name its lists and rows - and the whole
// value its look is given (`PopoverLook`). The key table is
// `client/spec/Views/Parts.lean` §7-5, modelled as `press` and
// `pressRow` in `client/spec/Views/Parts/Popover.lean`.
//
// It sits apart from `./popover`, which keeps the shapes callers hand a
// popover and re-exports the component beside them: this file is what
// the component itself reads, and a module the component read could
// not re-export the component without a cycle.

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Key } from "../../core/lang";
import type { Side } from "./layer";
import type { PopoverColumn, PopoverRow } from "./popover";

// `content` and `equal` lay the lists side by side, so the arrows walk
// one list and Tab changes it. `rows` stacks them as a table - each list
// one row, its label the leading cell - so the left and right arrows
// walk one row and the up and down arrows change it, as Tab does.
export type Layout = "content" | "equal" | "rows";

// The list the cursor is in and the row it is on.
export interface Place {
  readonly column: number;
  readonly cursor: number;
}

// What one key does.
export type Pressed =
  | { readonly kind: "move"; readonly to: Place }
  | { readonly kind: "apply" }
  | { readonly kind: "close" }
  | { readonly kind: "pass" };

// A key as the wiring reads it; a `KeyboardEvent` is one.
export interface Keyed {
  readonly key: string;
  readonly shiftKey: boolean;
  preventDefault(): void;
}

const PASS: Pressed = { kind: "pass" };

// The key table. `place` is already clamped to the lists, `rows` is
// how many rows each list holds. Left and right are claimed only where
// there is a row to walk; elsewhere they are left to a caller's caret.
export function pressed(layout: Layout, place: Place, key: Keyed, rows: readonly number[]): Pressed {
  const asRow = layout === "rows";
  const here = rows.at(place.column) ?? 0;
  const walk = (by: number): Pressed =>
    here === 0
      ? { kind: "move", to: place }
      : { kind: "move", to: { column: place.column, cursor: Math.min(here - 1, Math.max(0, place.cursor + by)) } };
  // A turn starts the next list at its top even when there is only one
  // list to turn to, as the model's `wrap` does.
  const turn = (by: number): Pressed =>
    rows.length === 0
      ? { kind: "move", to: place }
      : { kind: "move", to: { column: (place.column + by + rows.length) % rows.length, cursor: 0 } };
  switch (key.key) {
    case "ArrowRight":
      return asRow ? walk(1) : PASS;
    case "ArrowLeft":
      return asRow ? walk(-1) : PASS;
    case "ArrowDown":
      return asRow ? turn(1) : walk(1);
    case "ArrowUp":
      return asRow ? turn(-1) : walk(-1);
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

// The ids of one popover's lists and rows, unique within the document
// so `aria-activedescendant` names one row and not one in every popover
// that ever opened.
export const listId = (seat: string, column: number): string => `${seat}-c${String(column)}`;
export const rowId = (seat: string, column: number, index: number): string =>
  `${seat}-r${String(column)}-${String(index)}`;

// The state the seat holds, as the wiring reads it.
export interface PopoverView {
  readonly seat: string;
  readonly layout: Layout;
  readonly columns: readonly PopoverColumn[];
  readonly place: Place;
  // A caller's text box holds the focus and forwards its keys (bind
  // mode), so no list takes the focus or names the cursor.
  readonly holder: "list" | "caller";
  readonly side: Side;
  // The accessible name of the dialog, a lang.json key, as each
  // column's label is.
  readonly title: Key;
  // Puts a lang.json key in the person's language.
  readonly say: (key: Key) => string;
}

// What the seat lends the wiring.
export interface Hands {
  // Every key the table claimed moves the cursor, even to where it was:
  // a clamped key still reveals a cursor scrolled out by the wheel.
  readonly place: (to: Place) => void;
  readonly apply: (column: PopoverColumn, row: PopoverRow) => void;
  readonly close: () => void;
  readonly holdDialog: Attachment<HTMLDivElement>;
  readonly holdList: (column: number) => Attachment<HTMLUListElement>;
}

// The handler a list and a forwarding text box share: it answers whether
// the popover used the key, so a text box knows whether to let the
// character through.
export function keysOf(view: PopoverView, hands: Hands): (key: Keyed) => boolean {
  return (key) => {
    const answer = pressed(
      view.layout,
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

const HOLD = createAttachmentKey();

export interface DialogWire {
  readonly role: "dialog";
  readonly "aria-label": string;
  readonly [hold: symbol]: Attachment<HTMLDivElement>;
}

export interface ListWire {
  readonly id: string;
  readonly role: "listbox";
  readonly "aria-label": string;
  readonly tabindex: 0 | -1;
  readonly "aria-activedescendant": string | null;
  readonly onkeydown: (key: Keyed) => void;
  readonly [hold: symbol]: Attachment<HTMLUListElement>;
}

export interface OptionWire {
  readonly id: string;
  readonly role: "option";
  readonly "aria-selected": boolean;
  readonly onmouseenter: () => void;
  readonly onclick: () => void;
}

export interface RowLook {
  readonly key: string;
  // The row as the caller gave it, for a caller's own `row` snippet.
  readonly row: PopoverRow;
  readonly chosen: boolean;
  readonly cursor: boolean;
  readonly wire: OptionWire;
}

export interface ListLook {
  readonly key: string;
  readonly label: string;
  // A rule stands above this list in the `rows` layout.
  readonly apart: boolean;
  readonly wire: ListWire;
  readonly rows: readonly RowLook[];
}

export interface PopoverLook {
  readonly dialog: DialogWire;
  readonly layout: Layout;
  readonly side: Side;
  readonly lists: readonly ListLook[];
  readonly empty: string;
  readonly header: Snippet | undefined;
  readonly row: Snippet<[PopoverRow]> | undefined;
}

// What the caller draws inside the popover's own frame.
export interface Slots {
  readonly header: Snippet | undefined;
  readonly row: Snippet<[PopoverRow]> | undefined;
}

// The whole value a look draws, wired to `hands`.
export function lookOf(view: PopoverView, hands: Hands, slots: Slots): PopoverLook {
  const keys = keysOf(view, hands);
  const { seat, place } = view;
  const active = (view.columns.at(place.column)?.rows.length ?? 0) > 0;
  return {
    dialog: { role: "dialog", "aria-label": view.say(view.title), [HOLD]: hands.holdDialog },
    layout: view.layout,
    side: view.side,
    empty: view.say("part_no_match"),
    header: slots.header,
    row: slots.row,
    lists: view.columns.map((pane, column) => {
      const focused = view.holder === "list" && column === place.column;
      return {
        key: pane.id,
        label: view.say(pane.label),
        apart: pane.apart === true,
        wire: {
          id: listId(seat, column),
          role: "listbox",
          "aria-label": view.say(pane.label),
          tabindex: focused ? 0 : -1,
          "aria-activedescendant": focused && active ? rowId(seat, column, place.cursor) : null,
          onkeydown: (key) => {
            if (keys(key)) key.preventDefault();
          },
          [HOLD]: hands.holdList(column),
        },
        rows: pane.rows.map((row, index) => ({
          key: row.id,
          row,
          chosen: row.chosen === true,
          cursor: column === place.column && index === place.cursor,
          wire: {
            id: rowId(seat, column, index),
            role: "option",
            "aria-selected": row.chosen === true,
            onmouseenter: () => {
              hands.place({ column, cursor: index });
            },
            onclick: () => {
              hands.apply(pane, row);
            },
          },
        })),
      };
    }),
  };
}

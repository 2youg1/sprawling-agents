// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

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

export interface Hands {
  readonly place: (to: Place) => void;
  readonly apply: (column: PopoverColumn, row: PopoverRow) => void;
  readonly close: () => void;
  readonly holdDialog: Attachment<HTMLDivElement>;
  readonly holdList: (column: number) => Attachment<HTMLUListElement>;
}

export function keysOf(view: PopoverView, hands: Hands): (key: Keyed) => boolean {
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

  readonly row: PopoverRow;
  readonly chosen: boolean;
  readonly cursor: boolean;
  readonly wire: OptionWire;
}

export interface ListLook {
  readonly key: string;
  readonly label: string;
  readonly wire: ListWire;
  readonly rows: readonly RowLook[];
}

export interface PopoverLook {
  readonly dialog: DialogWire;
  readonly layout: Layout;
  readonly side: Side;
  readonly room: number | null;
  readonly lists: readonly ListLook[];
  readonly empty: string;
  readonly header: Snippet | undefined;
  readonly row: Snippet<[PopoverRow]> | undefined;
}

export interface Slots {
  readonly header: Snippet | undefined;
  readonly row: Snippet<[PopoverRow]> | undefined;
}

export function lookOf(view: PopoverView, hands: Hands, slots: Slots): PopoverLook {
  const keys = keysOf(view, hands);
  const { seat, place } = view;
  const active = (view.columns.at(place.column)?.rows.length ?? 0) > 0;
  return {
    dialog: { role: "dialog", "aria-label": view.say(view.title), [HOLD]: hands.holdDialog },
    layout: view.layout,
    side: view.side,
    room: view.room,
    empty: view.say("part_no_match"),
    header: slots.header,
    row: slots.row,
    lists: view.columns.map((pane, column) => {
      const focused = view.holder === "list" && column === place.column;
      return {
        key: pane.id,
        label: view.say(pane.label),
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

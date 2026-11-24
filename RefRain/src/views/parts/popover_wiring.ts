// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../../core/snippet";
import type { ElementRef } from "../../core/element_ref";

import type { Side } from "./layer";
import type { PopoverColumn, PopoverRow } from "./popover";

import {
  keysOf,
  listId,
  rowId,
  type Keyed,
  type Layout,
  type Place,
  type PopoverView,
} from "./popover_keys";
export { keysOf, listId, pressed, rowId } from "./popover_keys";
export type { Keyed, Layout, Place, PopoverView, Pressed } from "./popover_keys";

export interface Hands {
  readonly place: (to: Place) => void;
  readonly apply: (column: PopoverColumn, row: PopoverRow) => void;
  readonly close: () => void;
  readonly holdDialog: ElementRef<HTMLDivElement>;
  readonly holdList: (column: number) => ElementRef<HTMLUListElement>;
}

const HOLD = Symbol();

export interface DialogWire {
  readonly role: "dialog";
  readonly "aria-label": string;
  readonly [hold: symbol]: ElementRef<HTMLDivElement>;
}

export interface ListWire {
  readonly id: string;
  readonly role: "listbox";
  readonly "aria-label": string;
  readonly tabindex: 0 | -1;
  readonly "aria-activedescendant": string | null;
  readonly onkeydown: (key: Keyed) => void;
  readonly [hold: symbol]: ElementRef<HTMLUListElement>;
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

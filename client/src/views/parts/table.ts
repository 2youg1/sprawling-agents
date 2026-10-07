// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one column of a data table states about itself, so the table
// carries no parallel lists of drawing, ordering and correcting.
//
// A cell is content the caller passes as a snippet taking the row: the
// table draws the box, the column draws what goes in it. A column that
// is corrected in place replaces that drawing with an input and hands
// back what was typed rather than deciding what it means.
//
// Below the declarations is what the table decides before anything is
// drawn - the order of the rows, what each header tells a screen
// reader, the third state of the header box - and `lookOf`, which
// builds everything `./table.look.svelte` is handed.

import type { Snippet } from "svelte";

// The module `parts/table` keeps the surface `table.tsx` had - the
// table and its declarations reach a caller through one specifier - so
// the component is re-exported here beside the types it is called with.
export { default as Table } from "./table.svelte";

// What a column is worth before the table has any room to give: text is
// read twelve characters at a time, an identifier spans two dozen, and
// a figure is a control at its own fixed width. A minimum rather than a
// width, so a table with room to spare still hands the surplus to the
// columns that can use it.
export type Min = "12ch" | "24ch" | "figure";

export interface Column<T> {
  // The column's identity: which column the rows are ordered by is
  // remembered under this key.
  readonly key: string;
  // Already in the person's language.
  readonly header: string;
  // What a cell holds when the column is not corrected in place.
  readonly render: Snippet<[T]>;
  // Present makes the column sortable.
  readonly compare?: (a: T, b: T) => number;
  // A column a compact page does not draw. The header and every cell
  // carry the class `theme/preference.css` hides under `[data-density=compact]`,
  // so one rule covers a table and a list alike.
  readonly summary?: true;
  // What the column may not be drawn narrower than. Nothing a column
  // holds is broken across lines to make it fit: the container scrolls
  // sideways instead, because a value read one character per line is a
  // value nobody can read.
  readonly min?: Min;
  // Present makes the cell an input, and the table hands back what was
  // typed rather than deciding what it means.
  readonly editable?: {
    readonly text: (row: T) => string;
    readonly onEdit: (row: T, text: string) => void;
  };
}

export interface Selection<T> {
  readonly picked: (row: T) => boolean;
  readonly onPick: (row: T, on: boolean) => void;
  // The accessible name of the header checkbox.
  readonly allLabel: string;
  readonly onPickAll: (on: boolean) => void;
  readonly allPicked: () => boolean;
}

export interface TableProps<T> {
  // The accessible name of the table.
  readonly caption: string;
  readonly columns: readonly Column<T>[];
  readonly rows: readonly T[];
  // Both the identity of a row and the accessible name of its checkbox.
  readonly keyOf: (row: T) => string;
  readonly selection?: Selection<T>;
  // The caller's content for an empty table, because what to do about
  // no rows is knowledge of the page, not of the table.
  readonly empty?: Snippet;
}

// Which column the rows are ordered by, and which way. The key rather
// than the column, so a column that leaves the table leaves its order
// behind instead of the state holding a column nobody drew.
export type Order = "up" | "down";

export interface Sorted {
  readonly by: string | null;
  readonly order: Order;
}

export const UNSORTED: Sorted = { by: null, order: "up" };

// A press on a column's header: the same column turns round, another
// column starts ascending.
export function turned(sorted: Sorted, key: string): Sorted {
  if (sorted.by === key) {
    return { by: key, order: sorted.order === "up" ? "down" : "up" };
  }
  return { by: key, order: "up" };
}

// The rows in the order the table draws them.
export function ordered<T>(rows: readonly T[], columns: readonly Column<T>[], sorted: Sorted): readonly T[] {
  const compare = columns.find((each) => each.key === sorted.by)?.compare;
  if (compare === undefined) return rows;
  const sorting = [...rows].sort(compare);
  return sorted.order === "up" ? sorting : sorting.reverse();
}

// Some rows picked and some not is neither "all" nor "none", and a box
// that says "none" while rows are ticked is a lie.
export function mixed<T>(selection: Selection<T>, rows: readonly T[]): boolean {
  return !selection.allPicked() && rows.some((row) => selection.picked(row));
}

// Spread on a header cell. A column that cannot be ordered by states
// nothing: `aria-sort` on it would be a sorting statement about a
// column nobody can sort.
export interface HeadWire {
  readonly "aria-sort"?: "ascending" | "descending" | "none";
}

// Spread on the button a sortable column's header is.
export interface SortWire {
  readonly type: "button";
  readonly onclick: () => void;
}

// Spread on a checkbox. `indeterminate` is a DOM property with no
// attribute spelling, and a spread hands it to the element as one.
export interface TickWire {
  readonly type: "checkbox";
  readonly "aria-label": string;
  readonly checked: boolean;
  readonly indeterminate: boolean;
  readonly onchange: (event: { readonly currentTarget: { readonly checked: boolean } }) => void;
}

// Spread on the input a corrected cell is.
export interface FieldWire {
  readonly "aria-label": string;
  readonly value: string;
  readonly onchange: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface HeadLook {
  readonly key: string;
  readonly header: string;
  readonly min: Min | undefined;
  readonly summary: boolean;
  readonly cell: HeadWire;
  readonly sort: SortWire | undefined;
}

// What one cell holds: the column's own drawing of the row, or the
// input a corrected column hands back.
export type HeldLook<T> =
  | { readonly kind: "drawn"; readonly render: Snippet<[T]>; readonly row: T }
  | { readonly kind: "field"; readonly field: FieldWire };

export interface CellLook<T> {
  readonly key: string;
  readonly min: Min | undefined;
  readonly summary: boolean;
  readonly held: HeldLook<T>;
}

export interface LineLook<T> {
  readonly key: string;
  readonly tick: TickWire | undefined;
  readonly cells: readonly CellLook<T>[];
}

// Everything a table's look draws, and nothing else.
export interface TableLook<T> {
  readonly caption: string;
  readonly heads: readonly HeadLook[];
  readonly all: TickWire | undefined;
  readonly lines: readonly LineLook<T>[];
  readonly empty: Snippet | undefined;
}

export function lookOf<T>(props: TableProps<T>, sorted: Sorted, turn: (key: string) => void): TableLook<T> {
  const { caption, columns, keyOf, selection, empty } = props;
  const rows = ordered(props.rows, columns, sorted);
  return {
    caption,
    heads: columns.map((column) => headOf(column, sorted, turn)),
    all:
      selection === undefined
        ? undefined
        : {
            type: "checkbox",
            "aria-label": selection.allLabel,
            checked: selection.allPicked(),
            indeterminate: mixed(selection, rows),
            onchange: (event) => {
              selection.onPickAll(event.currentTarget.checked);
            },
          },
    lines: rows.map((row) => ({
      key: keyOf(row),
      tick:
        selection === undefined
          ? undefined
          : {
              type: "checkbox",
              "aria-label": keyOf(row),
              checked: selection.picked(row),
              indeterminate: false,
              onchange: (event) => {
                selection.onPick(row, event.currentTarget.checked);
              },
            },
      cells: columns.map((column) => ({
        key: column.key,
        min: column.min,
        summary: column.summary === true,
        held: heldOf(column, row, keyOf),
      })),
    })),
    empty,
  };
}

function headOf<T>(column: Column<T>, sorted: Sorted, turn: (key: string) => void): HeadLook {
  const base = { key: column.key, header: column.header, min: column.min, summary: column.summary === true };
  if (column.compare === undefined) {
    return { ...base, cell: {}, sort: undefined };
  }
  return {
    ...base,
    cell: { "aria-sort": sortSaid(sorted, column.key) },
    sort: {
      type: "button",
      onclick: () => {
        turn(column.key);
      },
    },
  };
}

function sortSaid(sorted: Sorted, key: string): "ascending" | "descending" | "none" {
  if (sorted.by !== key) return "none";
  return sorted.order === "up" ? "ascending" : "descending";
}

function heldOf<T>(column: Column<T>, row: T, keyOf: (row: T) => string): HeldLook<T> {
  const editable = column.editable;
  if (editable === undefined) {
    return { kind: "drawn", render: column.render, row };
  }
  return {
    kind: "field",
    field: {
      "aria-label": `${column.header} ${keyOf(row)}`,
      value: editable.text(row),
      onchange: (event) => {
        editable.onEdit(row, event.currentTarget.value);
      },
    },
  };
}

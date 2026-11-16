// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

export { default as Table } from "./table.svelte";

export type Min = "12ch" | "24ch" | "figure";

export interface Column<T> {
  readonly key: string;
  readonly header: string;
  readonly render: Snippet<[T]>;
  readonly compare?: (a: T, b: T) => number;
  readonly summary?: true;
  readonly min?: Min;
  readonly editable?: {
    readonly text: (row: T) => string;
    readonly onEdit: (row: T, text: string) => void;
  };
}

export interface Selection<T> {
  readonly picked: (row: T) => boolean;
  readonly onPick: (row: T, on: boolean) => void;
  readonly allLabel: string;
  readonly onPickAll: (on: boolean) => void;
  readonly allPicked: () => boolean;
}

export interface TableProps<T> {
  readonly caption: string;
  readonly columns: readonly Column<T>[];
  readonly rows: readonly T[];
  readonly keyOf: (row: T) => string;
  readonly selection?: Selection<T>;
  readonly empty?: Snippet;
}

export type Order = "up" | "down";

export interface Sorted {
  readonly by: string | null;
  readonly order: Order;
}

export const UNSORTED: Sorted = { by: null, order: "up" };

export function turned(sorted: Sorted, key: string): Sorted {
  if (sorted.by === key) {
    return { by: key, order: sorted.order === "up" ? "down" : "up" };
  }
  return { by: key, order: "up" };
}

export function ordered<T>(
  rows: readonly T[],
  columns: readonly Column<T>[],
  sorted: Sorted,
): readonly T[] {
  const compare = columns.find((each) => each.key === sorted.by)?.compare;
  if (compare === undefined) return rows;
  const sorting = [...rows].sort(compare);
  return sorted.order === "up" ? sorting : sorting.reverse();
}

export function mixed<T>(selection: Selection<T>, rows: readonly T[]): boolean {
  return !selection.allPicked() && rows.some((row) => selection.picked(row));
}

export interface HeadWire {
  readonly "aria-sort"?: "ascending" | "descending" | "none";
}

export interface SortWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface TickWire {
  readonly type: "checkbox";
  readonly "aria-label": string;
  readonly checked: boolean;
  readonly indeterminate: boolean;
  readonly onchange: (event: { readonly currentTarget: { readonly checked: boolean } }) => void;
}

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

export interface TableLook<T> {
  readonly caption: string;
  readonly heads: readonly HeadLook[];
  readonly all: TickWire | undefined;
  readonly lines: readonly LineLook<T>[];
  readonly empty: Snippet | undefined;
}

export function lookOf<T>(
  props: TableProps<T>,
  sorted: Sorted,
  turn: (key: string) => void,
): TableLook<T> {
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
  const base = {
    key: column.key,
    header: column.header,
    min: column.min,
    summary: column.summary === true,
  };
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

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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one popover lists: one column or three, each row one pick a
// person can make. The module `parts/popover` keeps the shape
// `popover.svelte` is called with in this file for the same reason
// `parts/table` keeps its own: the lint lane resolves no named export
// of a `.svelte` module for a `.svelte` reader, and the shapes must
// not be written down twice.

import type { Key } from "../../core/lang";

// The component beside the types it is called with.
export { default as Popover } from "./popover.svelte";

// One row of one column. `label` and `secondary` arrive already in
// the person's language, or as an identifier somebody typed.
export interface PopoverRow {
  // Unique within its column; the list is keyed on it.
  readonly id: string;
  readonly label: string;
  // The right-hand cell of a row: a grammar, an endpoint, an address.
  readonly secondary?: string | undefined;
  // What is applied now, marked rather than merely pointed at: a
  // cursor says where a person is, not what the city is doing. The
  // mark is `aria-selected` here exactly as in the combobox - the
  // cursor is carried by `aria-activedescendant` and never by this -
  // and it is drawn as the deeper wash, so a person sees the value in
  // force without a second control confirming it.
  readonly chosen?: boolean | undefined;
}

export interface PopoverBinding {
  readonly keys: (event: KeyboardEvent) => boolean;
  readonly controls: readonly string[];
  readonly pointColumn: (columnId: string) => void;
}

export interface PopoverColumn {
  // Unique within one popover.
  readonly id: string;
  // The one word this column is known by, a lang.json key: it names
  // both the heading and the list for a screen reader.
  readonly label: Key;
  readonly rows: readonly PopoverRow[];
}

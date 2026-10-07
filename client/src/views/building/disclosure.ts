// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a row that opens in place is given (client D95: a seat, a look
// and this file): a plan node opening into its cost, a commit opening
// into its facts, a changed file opening into its patch. The three are
// one control - a row whose press shows or hides what sits under it -
// so the building page and the run page draw them one way.

import type { Snippet } from "svelte";

// The column lines the cells of a row stand on, one per kind of row. A
// look draws the chevron in the first column and the seat's cells in the
// rest; the seat never writes a class, so a row's columns are named here
// rather than spelled by its caller.
export type DisclosureLayout = "plan" | "commit" | "change";

export interface DisclosureWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

export interface DisclosureLook {
  readonly layout: DisclosureLayout;
  readonly open: boolean;
  readonly wire: DisclosureWire;
  // The row's cells, already in the person's language.
  readonly cells: Snippet;
}

// The bag for a row standing `open`, which `toggle` turns over.
export function disclosureWire(open: boolean, toggle: () => void): DisclosureWire {
  return { type: "button", "aria-expanded": open, onclick: toggle };
}

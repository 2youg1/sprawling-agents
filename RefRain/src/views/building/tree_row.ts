// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one row of the building's tree is given (client D95: `tree.svelte`
// is the seat, `tree_row.look.svelte` draws the row). A folder opens and
// closes, so it alone states whether it stands open; a file or a
// transcript opens somewhere else and carries no `aria-expanded`, which
// would announce a disclosure that never discloses anything.

// What stands before the name: a folder's chevron, a transcript's arrow
// with the hint that names it, or nothing for a file.
export type TreeMark =
  | { readonly kind: "directory"; readonly open: boolean }
  | { readonly kind: "transcript"; readonly hint: string }
  | { readonly kind: "file" };

// How loudly the name is drawn: the row in the middle column, a hidden
// entry, or any other.
export type TreeTone = "picked" | "hidden" | "plain";

export interface TreeRowWire {
  readonly type: "button";
  readonly "aria-expanded": boolean | undefined;
  readonly onclick: () => void;
}

export interface TreeRowLook {
  // Already in the person's language: the entry's name, or the task of
  // the run that wrote a transcript.
  readonly name: string;
  readonly mark: TreeMark;
  readonly tone: TreeTone;
  // Whether the name is a run's id rather than words, set in the code face.
  readonly coded: boolean;
  // The hint on the lit dot while something works under this row.
  readonly live: string | undefined;
  // A file's size, shown while a hand is on the row.
  readonly size: string | undefined;
  readonly wire: TreeRowWire;
}

export function treeRowWire(mark: TreeMark, press: () => void): TreeRowWire {
  return {
    type: "button",
    "aria-expanded": mark.kind === "directory" ? mark.open : undefined,
    onclick: press,
  };
}

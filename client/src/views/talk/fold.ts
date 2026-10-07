// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the fold over the earlier stretch of a room is given: the one
// press that opens or closes the runs before the session now open, in
// the whole thread and in the results room alike.

export interface FoldWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

export interface FoldLook {
  // Already in the person's language: what the fold holds and what a
  // press does, as one line.
  readonly label: string;
  readonly wire: FoldWire;
}

// The bag for a fold standing `open`, which `toggle` turns over.
export function foldWire(open: boolean, toggle: () => void): FoldWire {
  return { type: "button", "aria-expanded": open, onclick: toggle };
}

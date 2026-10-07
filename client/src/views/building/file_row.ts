// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one changed file of the working tree is given (client D95:
// `status.svelte` is the seat, `file_row.look.svelte` draws the row). A
// press opens the file on the right side; the row whose file is in front
// there is the current one, which the bag says as well as the paint, so
// a screen reader hears which row the right side follows.

export interface FileRowWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly "aria-current": "true" | undefined;
  readonly onclick: () => void;
}

export interface FileRowLook {
  // All three already in the person's language: how the file changed,
  // its path, and how many lines moved.
  readonly how: string;
  readonly path: string;
  readonly lines: string;
  readonly current: boolean;
  readonly wire: FileRowWire;
}

export function fileRowWire(label: string, current: boolean, open: () => void): FileRowWire {
  return { type: "button", "aria-label": label, "aria-current": current ? "true" : undefined, onclick: open };
}

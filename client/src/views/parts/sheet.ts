// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one box a modal is drawn in. Two parts open a native `<dialog>` -
// the irreversible question (`dialog`) and the sheet of key chords
// (`kbd`) - and both draw it as the same raised face, so the face has
// one look, `sheet.look.svelte`, and this file is what that look is
// handed.

import type { Snippet } from "svelte";
import type { HTMLDialogAttributes } from "svelte/elements";

// Where the box stands. A question is centred on the page; the chord
// sheet stands a section below the top edge, because reading twelve
// chords is not navigating and the page underneath should not have to
// move to be read over; a specimen on `#/gallery` stands in its fold's
// flow, out of the top layer, where it covers nothing.
export type Stands = "centre" | "below-top" | "in-flow";

// The part of a cancel request a seat answers: it can be refused, so
// Escape never closes the element behind the caller's back.
export interface Cancelling {
  readonly preventDefault: () => void;
}

// Spread on the `<dialog>` element: its relations, its cancel request,
// `open` for a specimen, and the attachment the seat holds it by.
export type SheetWire = Omit<HTMLDialogAttributes, "oncancel"> & {
  readonly oncancel?: (event: Cancelling) => void;
};

export interface SheetLook {
  readonly wire: SheetWire;
  readonly stands: Stands;
  readonly children: Snippet;
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whether the command palette stands open (client/Spec.lean §4-64b,
// 4-10a). Module state, so the shell's palette key and the conversation's
// address line open the one palette; where the focus returns when it
// closes is the modal's (`modal.ts`), which takes it back to whatever
// held it before the palette opened.
//
// **How it was opened decides where the focus lands** (client/Spec.lean
// §4-64b). Opened from keys or a mouse, the focus goes into the box, so
// the next key types. Opened by a finger or a pen, the focus goes to
// the sheet: a focused text box raises a phone's soft keyboard, which
// covers half the list the press was made to see, and the person taps
// the box when they want to type.
export type Arrival = "keys" | "touch";

let shown = $state(false);
let arrived = $state<Arrival>("keys");

export function paletteShown(): boolean {
  return shown;
}

export function paletteArrival(): Arrival {
  return arrived;
}

export function openPalette(arrival: Arrival): void {
  arrived = arrival;
  shown = true;
}

export function closePalette(): void {
  shown = false;
}

// The shell's palette key: a key, so an opening lands in the box.
export function togglePalette(): void {
  arrived = "keys";
  shown = !shown;
}

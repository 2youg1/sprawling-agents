// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whether the command palette stands open (client/Spec.lean §4-64b,
// 4-10a). Module state, so the shell's palette key and the conversation's
// address line open the one palette; where the focus returns when it
// closes is the modal's (`modal.ts`), which takes it back to whatever
// held it before the palette opened.

let shown = $state(false);

export function paletteShown(): boolean {
  return shown;
}

export function openPalette(): void {
  shown = true;
}

export function closePalette(): void {
  shown = false;
}

export function togglePalette(): void {
  shown = !shown;
}

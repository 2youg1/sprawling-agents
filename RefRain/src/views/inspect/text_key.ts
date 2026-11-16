// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The inspector's small worded key (client D95): the way to the
// person's editor at the right of a crumb or a code view, the
// terminal's "original" toggle, and a letter's link to the run that sent
// it. All of them stand on a head row of a region and take the same
// quiet shape, so one look draws them; what each one does is its seat's.

// The bag spread on the key: a link where the key goes somewhere, a
// button where it acts, and `aria-pressed` on a button that stays down.
export type TextKeyWire =
  | { readonly href: string }
  | { readonly type: "button"; readonly "aria-pressed"?: boolean; readonly onclick: () => void };

// Everything a look is given.
export interface TextKeyLook {
  // Already in the person's language.
  readonly label: string;
  readonly wire: TextKeyWire;
}

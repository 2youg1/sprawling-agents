// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The cursor a typed box walks over the list under it - the palette's
// rows and the file finder's paths, both an APG Combobox whose box keeps
// the focus: ↓ and ↑ move the active option and stop at the ends, Enter
// takes it, and every other key is the box's own. A key an input method
// is composing is the input method's, whatever it is (client D99).

import type { Pressed } from "./parts/combobox";

export type BoxKey = { readonly kind: "move"; readonly to: number } | { readonly kind: "pick" } | { readonly kind: "pass" };

// What `key` does with the cursor at `cursor` over `count` options. An
// empty list keeps the cursor at 0, so the first option to arrive is
// the active one.
export function boxKey(pressed: Pressed, cursor: number, count: number): BoxKey {
  if (pressed.isComposing) return { kind: "pass" };
  switch (pressed.key) {
    case "ArrowDown":
      return { kind: "move", to: Math.max(0, Math.min(cursor + 1, count - 1)) };
    case "ArrowUp":
      return { kind: "move", to: Math.max(cursor - 1, 0) };
    case "Enter":
      return { kind: "pick" };
    default:
      return { kind: "pass" };
  }
}

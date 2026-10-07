// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the one icon-only key decides before anything is drawn. A key
// that shows only a glyph has no word of its own on the screen, so its
// name is given twice from one string: as the accessible name on the
// element, and as the hint the look draws in a `Tip` on hover and focus,
// rather than in a `title` a keyboard never reaches and a touch screen
// never shows (docs/frontend-method.md §4-34, design 4-18). A key a
// person may not use keeps its seat in the Tab order under
// `aria-disabled`, and the hint then says why instead, pointed at by
// `aria-describedby` so a screen reader reads the reason after the name
// (client/spec/Views/Parts.lean §7-2).

import type { GlyphName } from "./glyph";

export interface IconButtonProps {
  readonly glyph: GlyphName;
  // What the key does, already in the person's language.
  readonly label: string;
  // Present means the key cannot be used, and says why.
  readonly why?: string;
  readonly onPress?: () => void;
}

// Spread on the `<button>` the look draws. Enter and Space are both
// delivered as a click, so the guard inside `onclick` is the one place a
// press is stopped.
export interface IconButtonWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly "aria-disabled": boolean;
  readonly "aria-describedby": string | undefined;
  readonly onclick: () => void;
}

export interface IconButtonLook {
  readonly glyph: GlyphName;
  // The words the look's `Tip` draws: the name, or the reason the key
  // cannot be used.
  readonly hint: string;
  // The bag for the `<button>`, given the id of the `Tip` the look draws
  // round it (the `Tip` names its own hint).
  readonly wire: (hint: string) => IconButtonWire;
}

export function lookOf(props: IconButtonProps): IconButtonLook {
  const refused = props.why !== undefined;
  return {
    glyph: props.glyph,
    hint: props.why ?? props.label,
    wire: (hint) => ({
      type: "button",
      "aria-label": props.label,
      "aria-disabled": refused,
      "aria-describedby": refused ? hint : undefined,
      onclick: () => {
        if (refused) return;
        props.onPress?.();
      },
    }),
  };
}

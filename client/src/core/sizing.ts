// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The sizes a person may ask for are the city's range
// (`channels::BODY_PX_MIN` and `BODY_PX_MAX`), which the generated wire
// file states, so the box and the `[ui]` writer refuse the same sizes.
import { BODY_PX } from "../wire";

// What a box of digits says about the body size. Three outcomes rather
// than a number and a flag: an empty box and a refused box lead to
// different acts, and only one of them changes the page.
export type Sizing =
  // Nothing in the box: the page goes back to the size `theme.css` draws.
  | { readonly kind: "cleared" }
  | { readonly kind: "sized"; readonly px: number }
  // Not a whole number in range. The field says so and the page holds
  // the size it already has.
  | { readonly kind: "refused" };

export function sizingOf(text: string): Sizing {
  const trimmed = text.trim();
  if (trimmed === "") return { kind: "cleared" };
  if (!/^[0-9]+$/.test(trimmed)) return { kind: "refused" };
  const px = Number.parseInt(trimmed, 10);
  return px >= BODY_PX.min && px <= BODY_PX.max ? { kind: "sized", px } : { kind: "refused" };
}

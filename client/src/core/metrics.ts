// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two ratios the appearance group's boxes hold: a line box and the
// space between letters, both stored as thousandths
// (`wire::LineHeight`, `wire::Tracking`, wire D126) and both written as
// the CSS ratio a person reads, `1.625` and `-0.01`.
//
// **The floors are the city's**, read out of the generated wire file,
// so a box, a `[ui]` file and a frame refuse the same values. There is
// no ceiling, for the reason the body size has none: above the floor a
// larger number only sets the text looser.
import { LINE_HEIGHT, TRACKING } from "../wire";

// What a box of digits says about one ratio. Three outcomes rather than
// a number and a flag: an empty box and a refused box lead to different
// acts, and only one of them changes the page.
export type Measured =
  // Nothing in the box: the page goes back to the ratio the theme draws.
  | { readonly kind: "cleared" }
  | { readonly kind: "set"; readonly perMille: number }
  // Not a ratio at whole thousandths, or one under the floor. The field
  // says so and the page holds the ratio it already has.
  | { readonly kind: "refused" };

// The widest thousandths of an em the wire holds: `wire::Tracking` is an
// i32 and `wire::LineHeight` a u32, so a box that accepted more would send
// the city a number it refuses to decode, and the whole appearance would
// come back refused instead of this one figure.
const WIDEST = 2_147_483_647;

// Thousandths of the em a ratio is worth, or nothing when the text is
// not a ratio with at most three decimals, or when it is one no integer
// the wire carries could state: `1.625`, `1.6`, `2`, `-0.01`.
function perMilleOf(text: string): number | null {
  const trimmed = text.trim();
  if (!/^-?[0-9]+(\.[0-9]{1,3})?$/.test(trimmed)) return null;
  const perMille = Math.round(Number(trimmed) * 1000);
  return Number.isSafeInteger(perMille) && Math.abs(perMille) <= WIDEST ? perMille : null;
}

// What a box says about a ratio whose floor is `floor`. One function
// for the two boxes, so the two cannot read a typed value differently.
export function ratioOf(text: string, floor: number): Measured {
  if (text.trim() === "") return { kind: "cleared" };
  const perMille = perMilleOf(text);
  if (perMille === null || perMille < floor) return { kind: "refused" };
  return { kind: "set", perMille };
}

// A ratio as the box spells it: thousandths become at most three
// decimals, and nothing else is written (`1.625`, `1.5`, `2`, `0`).
export function spelledRatio(perMille: number): string {
  return String(perMille / 1000);
}

// The line box's floor, as the box and the stored row both read it.
export const LINE_HEIGHT_FLOOR = LINE_HEIGHT.min;
// The tracking floor, as the box and the stored row both read it.
export const TRACKING_FLOOR = TRACKING.min;

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the page is drawn, as the person chose it: the words each
// appearance setting is spelled with, the order a selector offers them
// in, and the record they make together. `prefs.ts` keeps the record
// between visits and `prefs_city.ts` carries it to the city; this file
// only says what a valid one is.

// `system` is the absence of an opinion, and it is resolved where the
// page is drawn rather than in the stylesheet: the light palette is
// declared once, and a second declaration of it inside a
// `prefers-color-scheme` block would be a second authority for the
// same eleven rungs.
export type Lighting = "system" | "dark" | "light";
export type Face = "geist" | "system" | "custom";
// How much air the six spacing steps carry, as `theme.css` spells it
// in `:root[data-density="compact"]`. Two named postures rather than a
// coefficient, because the coefficient is the stylesheet's to choose.
export type Density = "comfortable" | "compact";
export type Chroma = "full" | "off";
// `system` is the absence of an opinion, which is what the stylesheet's
// `prefers-reduced-motion` block reads.
export type Motion = "system" | "on" | "off";

// Every value a selector offers, in the order it is drawn, and the
// same list each stored string is read back through: an option a
// person can pick is therefore an option this build can load.
export const LIGHTINGS: readonly Lighting[] = ["system", "dark", "light"];
export const FACES: readonly Face[] = ["geist", "system", "custom"];
export const DENSITIES: readonly Density[] = ["comfortable", "compact"];
export const CHROMAS: readonly Chroma[] = ["full", "off"];
export const MOTIONS: readonly Motion[] = ["system", "on", "off"];
// What a person may write into a font stack: the characters a family
// name and its punctuation are made of, and nothing that could close
// the declaration it lands in. A stack with anything else in it is not
// repaired, it is refused, and the field says so.
export const STACK_SHAPE = /^[\p{L}\p{N} ,'"_-]{1,120}$/u;

export interface Appearance {
  readonly lighting: Lighting;
  readonly sans: Face;
  readonly mono: Face;
  readonly sansStack: string;
  readonly monoStack: string;
  // The size of a line of body text in pixels, or nothing when the
  // person has stated no size and the stylesheet's own is drawn.
  readonly body: number | null;
  readonly density: Density;
  readonly chroma: Chroma;
  readonly motion: Motion;
}

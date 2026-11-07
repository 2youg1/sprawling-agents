// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the page is drawn, as the person chose it: the words each
// appearance setting is spelled with, the order a selector offers them
// in, and the record they make together. `prefs.ts` keeps the record
// between visits and `prefs_city.ts` carries it to the city; this file
// only says what a valid one is.

import {
  Face as FaceSchema,
  Glass as GlassSchema,
  ReadingFace as ReadingFaceSchema,
  TypeFeature as TypeFeatureSchema,
  Weight as WeightSchema,
  type Face as WireFace,
  type Glass as WireGlass,
  type ReadingFace as WireReadingFace,
  type TypeFeature as WireTypeFeature,
  type Weight as WireWeight,
} from "../wire";

// `system` is the absence of an opinion, and it is resolved where the
// page is drawn rather than in the stylesheet: the light palette is
// declared once, and a second declaration of it inside a
// `prefers-color-scheme` block would be a second authority for the
// same eleven rungs.
export type Lighting = "system" | "dark" | "light";
// Where a face comes from: one of the two bundled families, whatever this
// machine calls its interface font, or a stack the person wrote (wire
// D123). `paper` is the family this build opens in, and `maple` is the
// one the doctor's NF CN build extends with Han glyphs.
export type Face = WireFace;
// Which face reading text is set in: the interface's own, or the serif
// face Libron, spelled by the wire (`wire::ReadingFace`, wire D52).
export type ReadingFace = WireReadingFace;
// One of a bundled face's own OpenType features, spelled as that face's
// name table spells it (wire D125).
export type TypeFeature = WireTypeFeature;
// How heavy running text stands (wire D126).
export type Weight = WireWeight;
// How much air the six spacing steps carry, as `theme/preference.css` spells it
// in `:root[data-density="compact"]`. Two named postures rather than a
// coefficient, because the coefficient is the stylesheet's to choose.
export type Density = "comfortable" | "compact";
export type Chroma = "full" | "off";
// `system` is the absence of an opinion, which is what the stylesheet's
// `prefers-reduced-motion` block reads.
export type Motion = "system" | "on" | "off";
// Whether the edge layer's small surfaces are drawn as glass, spelled by
// the wire (`wire::Glass`, docs/frontend-method.md §4-43).
export type Glass = WireGlass;

// Every value a selector offers, in the order it is drawn, and the
// same list each stored string is read back through: an option a
// person can pick is therefore an option this build can load.
export const LIGHTINGS: readonly Lighting[] = ["system", "dark", "light"];
// The bundled faces first, in the order a person meets them, then the
// two ways of naming a face this build does not carry. Read out of the
// wire's union rather than spelled again here, so a value added to
// `wire::Face` is offered by the picker without a second list to forget.
export const FACES: readonly Face[] = FaceSchema.members.map((face) => face.literal);
export const READING_FACES: readonly ReadingFace[] = ReadingFaceSchema.members.map(
  (face) => face.literal,
);
// The six features in the order `wire::TypeFeature` declares them, which
// is the order the name table lists them in and the order the card draws.
export const FEATURES: readonly TypeFeature[] = TypeFeatureSchema.members.map(
  (feature) => feature.literal,
);
export const WEIGHTS: readonly Weight[] = WeightSchema.literals;
export const DENSITIES: readonly Density[] = ["comfortable", "compact"];
export const CHROMAS: readonly Chroma[] = ["full", "off"];
export const MOTIONS: readonly Motion[] = ["system", "on", "off"];
export const GLASSES: readonly Glass[] = GlassSchema.literals;
// The slider that sets the world layer's opacity in the blend tier, in
// percent. The opacity drawn while the person has said nothing is
// `theme/colour.css`'s `--blend-opacity`, and only there.
export const BLEND_PERCENT = { min: 30, max: 90, step: 5 } as const;
// What a person may write into a font stack: the characters a family
// name and its punctuation are made of, and nothing that could close
// the declaration it lands in. A stack with anything else in it is not
// repaired, it is refused, and the field says so.
export const STACK_SHAPE = /^[\p{L}\p{N} ,'"_-]{1,120}$/u;

// What one family name may hold: the letters, digits, spaces, `-`, `_` and
// `.` a name is made of, and nothing that could close the declaration it
// lands in. No quote and no comma: the page adds the quotes itself, and a
// comma would splice a second family into a slot that holds one
// (`crates/wire/spec/Preference.lean` D124 states the same grammar).
export const FAMILY_SHAPE = /^[\p{L}\p{N} _.-]{1,64}$/u;

// The features a bundled face declares for itself
// (`crates/wire/spec/Preference.lean` D125). Paper Mono's name table
// carries all six; Maple Mono carries three of the same intents, under
// its own tags (`calt`, `cv02`, `zero`), and declares nothing for the
// other three. A face this build does not bundle declares nothing here:
// what `ss01` means in a face nobody looked at is a guess.
const DECLARED: Readonly<Record<Face, readonly TypeFeature[]>> = {
  paper: FEATURES,
  maple: ["coding_ligatures", "single_story_a", "slashed_zero"],
  system: [],
  custom: [],
};

// Which of the six switches say something about the faces in use: one
// that no face in use declares is drawn as unavailable, and one that any
// of them declares is drawn live, because it still moves that face's
// text. A new bundled face joins this by adding its row to `DECLARED`,
// which the compiler refuses to let it forget.
export function featuresOffered(faces: readonly Face[]): ReadonlySet<TypeFeature> {
  return new Set(faces.flatMap((face) => DECLARED[face]));
}

export interface Appearance {
  readonly lighting: Lighting;
  readonly sans: Face;
  readonly mono: Face;
  readonly sansStack: string;
  readonly monoStack: string;
  readonly reading: ReadingFace;
  // The families the chosen face is followed by for the glyphs it lacks,
  // in the order they are tried: at most two names, usually one Chinese
  // face (wire D124). Empty means the machine's own choice stands.
  readonly fallbacks: readonly string[];
  // The bundled face's own features the person turned on (wire D125).
  readonly features: readonly TypeFeature[];
  // The size of a line of body text in pixels, or nothing when the
  // person has stated no size and the stylesheet's own is drawn.
  readonly body: number | null;
  // The height of one line of running text in thousandths of the font
  // size, or nothing when the person has stated none and the
  // stylesheet's own is drawn (wire D126).
  readonly lineHeight: number | null;
  // The space between letters in thousandths of an em, or nothing when
  // the person has stated none and the page's own posture is drawn.
  readonly tracking: number | null;
  // How heavy running text stands.
  readonly weight: Weight;
  readonly density: Density;
  readonly chroma: Chroma;
  readonly motion: Motion;
  readonly glass: Glass;
  // The world layer's opacity in the blend tier in percent, or nothing
  // when the person has stated none and the stylesheet's own is drawn.
  readonly blend: number | null;
}

// A stored or typed opacity as a percent the slider offers, or `null`
// for anything else: a whole number inside the slider's domain.
export function blendOf(raw: string): number | null {
  const said = /^[0-9]{1,3}$/.test(raw) ? Number(raw) : Number.NaN;
  return said >= BLEND_PERCENT.min && said <= BLEND_PERCENT.max ? said : null;
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

export type Lighting = "system" | "dark" | "light";

export type Face = WireFace;

export type ReadingFace = WireReadingFace;

export type TypeFeature = WireTypeFeature;

export type Weight = WireWeight;

export type Density = "comfortable" | "compact";
export type Chroma = "full" | "off";

export type Motion = "system" | "on" | "off";

export type Glass = WireGlass;

export const LIGHTINGS: readonly Lighting[] = ["system", "dark", "light"];

export const FACES: readonly Face[] = FaceSchema.members.map((face) => face.literal);
export const READING_FACES: readonly ReadingFace[] = ReadingFaceSchema.members.map(
  (face) => face.literal,
);

export const FEATURES: readonly TypeFeature[] = TypeFeatureSchema.members.map(
  (feature) => feature.literal,
);
export const WEIGHTS: readonly Weight[] = WeightSchema.literals;
export const DENSITIES: readonly Density[] = ["comfortable", "compact"];
export const CHROMAS: readonly Chroma[] = ["full", "off"];
export const MOTIONS: readonly Motion[] = ["system", "on", "off"];
export const GLASSES: readonly Glass[] = GlassSchema.literals;

export const BLEND_PERCENT = { min: 30, max: 90, step: 5 } as const;

export const STACK_SHAPE = /^[\p{L}\p{N} ,'"_-]{1,120}$/u;

export const FAMILY_SHAPE = /^[\p{L}\p{N} _.-]{1,64}$/u;

const DECLARED: Readonly<Record<Face, readonly TypeFeature[]>> = {
  paper: FEATURES,
  maple: ["coding_ligatures", "single_story_a", "slashed_zero"],
  system: [],
  custom: [],
};

export function featuresOffered(faces: readonly Face[]): ReadonlySet<TypeFeature> {
  return new Set(faces.flatMap((face) => DECLARED[face]));
}

export interface Appearance {
  readonly lighting: Lighting;
  readonly neutralPalette: boolean;
  readonly sans: Face;
  readonly mono: Face;
  readonly sansStack: string;
  readonly monoStack: string;
  readonly reading: ReadingFace;
  readonly fallbacks: readonly string[];
  readonly features: readonly TypeFeature[];
  readonly body: number | null;
  readonly lineHeight: number | null;
  readonly tracking: number | null;
  readonly weight: Weight;
  readonly density: Density;
  readonly chroma: Chroma;
  readonly motion: Motion;
  readonly glass: Glass;
  readonly blend: number | null;
}

export function blendOf(raw: string): number | null {
  const said = /^[0-9]{1,3}$/.test(raw) ? Number(raw) : Number.NaN;
  return said >= BLEND_PERCENT.min && said <= BLEND_PERCENT.max ? said : null;
}

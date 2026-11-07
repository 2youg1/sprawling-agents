// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the appearance settings decide before anything is drawn: which
// word each stored value is offered under, how a choice reaches the
// root element, and how a change says it was kept. The drawing is
// `appearance.svelte`; this module touches no component, so these
// decisions stand without one (`bun test` cannot compile Svelte).
//
// `main.ts` reads two of these before the first paint - a face chosen
// once is the face the next window opens with, and applying it after
// mount is a visible change of shape a person did not ask for. A plain
// `.ts` cannot hold runes, which is exactly why the reactive half of
// the save receipt below is a store: this is the one spelling both the
// page and its appearance group read.

import { get, writable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Key } from "../../core/lang";
import { STACK_SHAPE } from "../../core/appearance";
import type {
  Appearance,
  Chroma,
  Density,
  Face,
  Glass,
  Lighting,
  Motion,
  ReadingFace,
  TypeFeature,
  Weight,
} from "../../core/appearance";
import { spelledRatio } from "../../core/metrics";
import type { PreferenceDoor } from "../../core/prefs";
import { motionOff } from "../shared/motion";

// The Local Font Access API, which Chromium offers and other engines do
// not. Declared optional so the capability check is the type check.
declare global {
  interface Window {
    queryLocalFonts?: () => Promise<readonly { readonly family: string }[]>;
  }
}

// Which of the two axes a face applies to. Named because the list of
// installed faces writes to the axis it is drawn under, and an axis
// inferred from which face happens to be custom made one of the two
// unreachable whenever both were.
export type Axis = "sans" | "mono";

// Which card a receipt belongs to: a setting, since one setting can
// grow a second control (a face card carries its stack box). The type
// cards (`appearance_type.svelte`) and the rest of the group
// (`appearance.svelte`) write through one receipt, so one list names
// them all.
export type Setting =
  | "lighting"
  | "face"
  | "mono"
  | "reading"
  | "fallbacks"
  | "features"
  | "body"
  | "metrics"
  | "density"
  | "chroma"
  | "motion"
  | "glass"
  | "blend";

// The custom property the body size is written to, which is the same
// property `theme/tokens-type.css` declares the default in and derives the note and
// label steps from.
const BODY_PROPERTY = "--text-body";
// The custom property the fallback chain is written to: the families
// after the chosen face, each with the comma that joins it to what is in
// front of it, so an unnamed chain leaves the family list valid.
const FALLBACK_PROPERTY = "--face-fallback";
// The custom property the line box is written to, which the elements
// that run text read through `leading-read` (`theme/tokens-type.css`).
const LINE_HEIGHT_PROPERTY = "--leading-read";
// The custom property the tracking is written to, which
// `theme/base.css` reads in both its postures.
const TRACKING_PROPERTY = "--tracking";
// The custom property the world layer's opacity in the blend tier is
// written to, which `theme/colour.css` declares the default in.
const BLEND_PROPERTY = "--blend-opacity";

// The word each stored value is offered under. A record keyed by the
// value rather than a list of pairs: the set of values belongs to
// `core/prefs.ts`, which reads every stored row back through it, and a
// value added there leaves this table refusing to compile until
// somebody gives it a word.
export const LIGHTING_WORDS: Record<Lighting, Key> = {
  system: "appearance_lighting_system",
  dark: "appearance_lighting_dark",
  light: "appearance_lighting_light",
};
export const FACE_WORDS: Record<Face, Key> = {
  paper: "appearance_face_paper",
  maple: "appearance_face_maple",
  system: "appearance_face_system",
  custom: "appearance_face_custom",
};
export const WEIGHT_WORDS: Record<Weight, Key> = {
  light: "appearance_weight_light",
  regular: "appearance_weight_regular",
  medium: "appearance_weight_medium",
};
// What each of the bundled face's own features is called, in the words a
// person reads. A record keyed by the value rather than a list of pairs:
// the set of features belongs to the wire, and a feature added there
// leaves this table refusing to compile until somebody gives it a word.
export const FEATURE_WORDS: Record<TypeFeature, Key> = {
  coding_ligatures: "appearance_feature_coding_ligatures",
  duospace: "appearance_feature_duospace",
  narrow_space: "appearance_feature_narrow_space",
  small_arrows: "appearance_feature_small_arrows",
  single_story_a: "appearance_feature_single_story_a",
  slashed_zero: "appearance_feature_slashed_zero",
};
export const READING_WORDS: Record<ReadingFace, Key> = {
  interface: "appearance_reading_interface",
  libron: "appearance_reading_libron",
};
export const DENSITY_WORDS: Record<Density, Key> = {
  comfortable: "appearance_density_comfortable",
  compact: "appearance_density_compact",
};
export const CHROMA_WORDS: Record<Chroma, Key> = {
  full: "appearance_chroma_full",
  off: "appearance_chroma_none",
};
export const MOTION_WORDS: Record<Motion, Key> = {
  system: "appearance_motion_system",
  on: "appearance_motion_full",
  off: "appearance_motion_off",
};
export const GLASS_WORDS: Record<Glass, Key> = {
  on: "appearance_glass_on",
  off: "appearance_glass_off",
};

// One control's cells: every value this build can load, in the order
// `core/prefs.ts` offers them, each under the word this file gives it.
export function cellsOf<T extends string>(
  offered: readonly T[],
  words: Record<T, Key>,
  said: (key: Key) => string,
): readonly { readonly value: T; readonly label: string }[] {
  return offered.map((each) => ({ value: each, label: said(words[each]) }));
}

// One face, as the root element spells it: an attribute naming the
// choice, and - for a stack somebody typed - the stack itself, which is
// the one value the stylesheet cannot hold in advance.
function wearFace(root: HTMLElement, axis: Axis, face: Face, stack: string): void {
  const property = axis === "sans" ? "--face-sans" : "--face-mono";
  root.dataset[axis] = face;
  if (face === "custom" && STACK_SHAPE.test(stack)) {
    root.style.setProperty(property, stack);
  } else {
    root.style.removeProperty(property);
  }
}

// Which way the machine says it is lit. The query is the only one a
// browser offers, so "not light" is what dark means here.
const MACHINE_LIGHT = "(prefers-color-scheme: light)";

function machineLighting(): "dark" | "light" {
  return window.matchMedia(MACHINE_LIGHT).matches ? "light" : "dark";
}

// Follow the machine while nobody has said otherwise. Called once, from
// the one place that starts the client: a listener added wherever the
// settings screen mounts would be a second listener doing the same
// work, and neither would know about the other.
export function watchMachineLighting(root: HTMLElement, kept: PreferenceDoor): void {
  window.matchMedia(MACHINE_LIGHT).addEventListener("change", () => {
    const held = get(kept.held).appearance;
    if (held.lighting === "system") applyAppearance(root, held);
  });
}

export function applyAppearance(root: HTMLElement, held: Appearance): void {
  wearFace(root, "sans", held.sans, held.sansStack);
  wearFace(root, "mono", held.mono, held.monoStack);
  root.dataset.read = held.reading;
  // The chain the person named, each name carrying the comma after it,
  // which is what `theme/tokens-type.css` expects of the token: a chain
  // nobody named is a removed property, and the stack reads on with its
  // own comma (`var(--face-fallback,)`).
  if (held.fallbacks.length === 0) {
    root.style.removeProperty(FALLBACK_PROPERTY);
  } else {
    root.style.setProperty(FALLBACK_PROPERTY, fallbackToken(held.fallbacks));
  }
  // The words are the wire's own spelling, which is what
  // `theme/preference.css` selects on: one spelling from the file to the
  // stylesheet.
  if (held.features.length === 0) delete root.dataset.features;
  else root.dataset.features = held.features.join(" ");
  root.dataset.weight = held.weight;
  if (held.body === null) {
    root.style.removeProperty(BODY_PROPERTY);
  } else {
    root.style.setProperty(BODY_PROPERTY, `${String(held.body)}px`);
  }
  if (held.lineHeight === null) {
    root.style.removeProperty(LINE_HEIGHT_PROPERTY);
  } else {
    root.style.setProperty(LINE_HEIGHT_PROPERTY, spelledRatio(held.lineHeight));
  }
  if (held.tracking === null) {
    root.style.removeProperty(TRACKING_PROPERTY);
  } else {
    root.style.setProperty(TRACKING_PROPERTY, `${spelledRatio(held.tracking)}em`);
  }
  root.dataset.density = held.density;
  root.dataset.chroma = held.chroma;
  root.dataset.motion = held.motion;
  root.dataset.glass = held.glass;
  if (held.blend === null) {
    root.style.removeProperty(BLEND_PROPERTY);
  } else {
    root.style.setProperty(BLEND_PROPERTY, `${String(held.blend)}%`);
  }
  // Last, so the motion it reads is the one this change asked for.
  wearLighting(root, held.lighting === "system" ? machineLighting() : held.lighting);
}

// Light and dark trade places as one cross-fade of the whole page, a
// `panel` long (docs/frontend-method.md §4-43), rather than every
// surface snapping at once: a view transition fades the old picture
// into the new one. The first paint has nothing to fade from, motion
// off asks for no travel, and an engine without the API swaps in one
// frame. The page takes no pointer for that quarter second, which a
// press on the switch that started it does not miss.
function wearLighting(root: HTMLElement, lighting: "dark" | "light"): void {
  const was = root.dataset.theme;
  if (
    was === undefined ||
    was === lighting ||
    !("startViewTransition" in document) ||
    motionOff(root)
  ) {
    root.dataset.theme = lighting;
    return;
  }
  void document.startViewTransition(() => {
    root.dataset.theme = lighting;
  });
}

// What a person may write into a font stack: the characters a family
// name is made of. A stack with anything else in it is not repaired, it
// is refused, and the field says so.
export function stackRefused(stack: string): boolean {
  return stack !== "" && !STACK_SHAPE.test(stack);
}

// The fallback chain as the one custom property the family stacks read.
// Each name is quoted and followed by the comma that joins it to the next
// family, so one token holds the whole chain: the stacks in
// `theme/tokens-type.css` put it where a family would stand and supply the
// comma that ends it when nobody named one (`var(--face-fallback,)`).
export function fallbackToken(fallbacks: readonly string[]): string {
  return fallbacks.map((name) => `"${name}",`).join(" ");
}

// The size the page is drawn at right now, in whole pixels, read back
// off the root element. A box nobody has filled in shows what the
// person is looking at, and this file never repeats the stylesheet's
// own figure.
export function drawnSize(root: HTMLElement): string {
  const drawn = window.getComputedStyle(root).getPropertyValue(BODY_PROPERTY);
  return /^([0-9]+)/.exec(drawn.trim())?.[1] ?? "";
}

// The line box the page is drawn at right now, read back off the root
// element for the reason `drawnSize` reads the body size: the box shows
// what a person is looking at, and this file never repeats the
// stylesheet's own figure.
export function drawnLeading(root: HTMLElement): string {
  const drawn = window.getComputedStyle(root).getPropertyValue(LINE_HEIGHT_PROPERTY);
  return /^([0-9]*\.?[0-9]+)/.exec(drawn.trim())?.[1] ?? "";
}

// The world layer's opacity in the blend tier as the page draws it now,
// in whole percent, read back off the root element for the reason
// `drawnSize` reads the body size: the slider shows what a person is
// looking at, and this file never repeats the stylesheet's figure.
export function drawnBlend(root: HTMLElement): number | null {
  const drawn = window.getComputedStyle(root).getPropertyValue(BLEND_PROPERTY).trim();
  const percent = /^([0-9]+)%$/.exec(drawn)?.[1];
  return percent === undefined ? null : Number(percent);
}

// ------------------------------------------------------- the receipt

// How long an instant control keeps saying the change was kept, and the
// receipt itself (ux-upgrades A2). Long enough for the one word to be
// read, short enough not to stand there looking like a state. One home
// for both figures because the settings page shows this receipt in two
// places - its own cards and the appearance group's - and two clocks
// would be two answers to "how long is a moment".
export const SAVED_MS = 1200;

export interface Receipt {
  // The name of the card whose change last landed, or `null`.
  readonly saved: Readable<string | null>;
  readonly landed: (name: string) => void;
}

// The receipt one screen keeps. A store rather than runes because this
// module is plain TypeScript, which is the same boundary `core/prefs.ts`
// crosses for the record it holds.
export function saveReceipt(): Receipt {
  const saved = writable<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  return {
    saved,
    landed(name) {
      saved.set(name);
      if (timer !== undefined) clearTimeout(timer);
      timer = setTimeout(() => {
        saved.set(null);
      }, SAVED_MS);
    },
  };
}

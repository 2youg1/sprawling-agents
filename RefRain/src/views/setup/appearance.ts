// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, writable } from "../../core/store";
import type { Readable } from "../../core/store";

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

declare global {
  interface Window {
    queryLocalFonts?: () => Promise<readonly { readonly family: string }[]>;
  }
}

export type Axis = "sans" | "mono";

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

const BODY_PROPERTY = "--text-body";
const FALLBACK_PROPERTY = "--face-fallback";
const LINE_HEIGHT_PROPERTY = "--leading-read";
const TRACKING_PROPERTY = "--tracking";
const BLEND_PROPERTY = "--blend-opacity";

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

export function cellsOf<T extends string>(
  offered: readonly T[],
  words: Record<T, Key>,
  said: (key: Key) => string,
): readonly { readonly value: T; readonly label: string }[] {
  return offered.map((each) => ({ value: each, label: said(words[each]) }));
}

function wearFace(root: HTMLElement, axis: Axis, face: Face, stack: string): void {
  const property = axis === "sans" ? "--face-sans" : "--face-mono";
  root.dataset[axis] = face;
  if (face === "custom" && STACK_SHAPE.test(stack)) {
    root.style.setProperty(property, stack);
  } else {
    root.style.removeProperty(property);
  }
}

const MACHINE_LIGHT = "(prefers-color-scheme: light)";

function machineLighting(): "dark" | "light" {
  return window.matchMedia(MACHINE_LIGHT).matches ? "light" : "dark";
}

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
  if (held.fallbacks.length === 0) {
    root.style.removeProperty(FALLBACK_PROPERTY);
  } else {
    root.style.setProperty(FALLBACK_PROPERTY, fallbackToken(held.fallbacks));
  }
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
  wearLighting(root, held.lighting === "system" ? machineLighting() : held.lighting);
}

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

export function stackRefused(stack: string): boolean {
  return stack !== "" && !STACK_SHAPE.test(stack);
}

export function faceStyle(family: string): string {
  return `"${family.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}", ui-sans-serif, system-ui, sans-serif`;
}

export function fallbackToken(fallbacks: readonly string[]): string {
  return fallbacks.map((name) => `"${name}",`).join(" ");
}

export function drawnSize(root: HTMLElement): string {
  const drawn = window.getComputedStyle(root).getPropertyValue(BODY_PROPERTY);
  return /^([0-9]+)/.exec(drawn.trim())?.[1] ?? "";
}

export function drawnLeading(root: HTMLElement): string {
  const drawn = window.getComputedStyle(root).getPropertyValue(LINE_HEIGHT_PROPERTY);
  return /^([0-9]*\.?[0-9]+)/.exec(drawn.trim())?.[1] ?? "";
}

export function drawnBlend(root: HTMLElement): number | null {
  const drawn = window.getComputedStyle(root).getPropertyValue(BLEND_PROPERTY).trim();
  const percent = /^([0-9]+)%$/.exec(drawn)?.[1];
  return percent === undefined ? null : Number(percent);
}

export const SAVED_MS = 1200;

export interface Receipt {
  readonly saved: Readable<string | null>;
  readonly landed: (name: string) => void;
}

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

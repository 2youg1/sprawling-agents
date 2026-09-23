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
import { STACK_SHAPE } from "../../core/prefs";
import type {
  Appearance,
  Chroma,
  Density,
  Face,
  Lighting,
  Motion,
  PreferenceDoor,
} from "../../core/prefs";

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

// The custom property the body size is written to, which is the same
// property `theme.css` declares the default in and derives the note and
// label steps from.
const BODY_PROPERTY = "--text-body";

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
  geist: "appearance_face_geist",
  system: "appearance_face_system",
  custom: "appearance_face_custom",
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
  root.dataset.theme = held.lighting === "system" ? machineLighting() : held.lighting;
  wearFace(root, "sans", held.sans, held.sansStack);
  wearFace(root, "mono", held.mono, held.monoStack);
  if (held.body === null) {
    root.style.removeProperty(BODY_PROPERTY);
  } else {
    root.style.setProperty(BODY_PROPERTY, `${String(held.body)}px`);
  }
  root.dataset.density = held.density;
  root.dataset.chroma = held.chroma;
  root.dataset.motion = held.motion;
}

// Whether a stack box holds characters this page may not write into a
// declaration. An empty box is not a refusal: absence is the person
// saying nothing, and the page keeps the face it already chose.
export function stackRefused(stack: string): boolean {
  return stack !== "" && !STACK_SHAPE.test(stack);
}

// The size the page is drawn at right now, in whole pixels, read back
// off the root element. A box nobody has filled in shows what the
// person is looking at, and this file never repeats the stylesheet's
// own figure.
export function drawnSize(root: HTMLElement): string {
  const drawn = window.getComputedStyle(root).getPropertyValue(BODY_PROPERTY);
  return /^([0-9]+)/.exec(drawn.trim())?.[1] ?? "";
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

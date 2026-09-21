// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this browser remembers about the person, as one value with one
// door in front of it.
//
// **Every row the client keeps is named here and nowhere else.** The
// settings screens, the shell and the keymap each used to spell their
// own row names against the same store, which is how a key written
// one way and read another goes unnoticed; now they read
// `PreferenceDoor` and this file is the only reader of `ROWS`.
//
// **The door is where the city's answer will arrive.** The person's
// own layer of `config.toml` is the authority these values are headed
// for (roadmap 3.14), so every reader above already takes them as one
// record handed over by a door rather than as rows it fetches itself:
// the day `PreferencesAnswer` lands, this file changes and no view
// does.
//
// **A guess this build cannot read is dropped rather than repaired.**
// An unreadable row falls back to the posture the client ships with,
// which `theme.css` already draws, so a cache can never become a
// second authority for a value somebody else owns.
//
// How hard the model thinks is deliberately absent. It is the city's
// `[model] effort`, and a copy kept here would ride on every dispatch
// from this browser and quietly overrule the city's own file.

import { createSignal } from "solid-js";
import type { Accessor } from "solid-js";

import type { Lang } from "./lang";
import { langOf } from "./lang";
import { browserRows } from "./rows";
import type { Rows } from "./rows";
import type { Proxying } from "../wire";

// ------------------------------------------------------------- the rows

// Every row this client keeps, by the name it is kept under. Two of
// them are families rather than single rows: a draft is kept per place
// a person writes, and a chord per action the person rebound, so those
// two are prefixes and the rest are whole names.
const ROWS = {
  lang: "sprawling.lang",
  welcomed: "sprawling.welcomed",
  // Whether the artefact panel beside a conversation is open.
  panel: "sprawling.talk.panel",
  lighting: "sprawling.appearance.lighting",
  sans: "sprawling.appearance.sans",
  mono: "sprawling.appearance.mono",
  sansStack: "sprawling.appearance.sans_stack",
  monoStack: "sprawling.appearance.mono_stack",
  body: "sprawling.appearance.body",
  density: "sprawling.appearance.density",
  chroma: "sprawling.appearance.chroma",
  motion: "sprawling.appearance.motion",
  // Which rule a provider attached from now on starts with. A person
  // behind a relay settles it once, on the network screen, instead of
  // on every form they open; an endpoint already attached keeps the
  // rule the city recorded for it.
  proxying: "sprawling.network.proxying",
  // One unsent message per place a person writes, kept across a reload
  // or a page change; the rest of the name is the room or the run.
  draft: "sprawling.draft.",
  // One chord per action the person rebound; the rest of the name is
  // the action. An action left at its shipped chord has no row.
  chord: "sprawling.key.",
} as const;

// ------------------------------------------------------------ appearance

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
const PROXYINGS: readonly Proxying[] = ["except_local", "always", "never"];

// The sizes a person may ask for. The floor is the smallest size the
// colour gate has to hold its contrast tiers at, and the ceiling is
// where a line of body text stops being body text.
export const BODY_PX = { min: 12, max: 20 } as const;

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

// ---------------------------------------------------------- the reading

// The person's preferences, whole. One value rather than a dozen
// accessors because that is the shape the city will answer with, and
// because a screen that changes two of them at once must not be able
// to write one and drop the other.
export interface Preferences {
  readonly lang: Lang;
  // Whether this browser has walked through the welcome once. The city
  // decides whether setup is *needed*; this only decides whether a
  // person who skipped it is nagged again.
  readonly welcomed: boolean;
  readonly panel: boolean;
  readonly appearance: Appearance;
  readonly proxying: Proxying;
}

// The one way to the person's preferences: the record as it stands,
// five named changes to it, and the two families that are read by name
// because they have one row each per place and per action.
//
// Named changes rather than one `write`, because each of them becomes
// its own command the day the city keeps these: a caller that handed
// over a whole record would have to be rewritten then, and a caller
// that says which fact it is changing would not.
export interface PreferenceDoor {
  readonly held: Accessor<Preferences>;
  readonly setLang: (lang: Lang) => void;
  readonly setWelcomed: (done: boolean) => void;
  readonly setPanel: (open: boolean) => void;
  readonly setAppearance: (next: Appearance) => void;
  readonly setProxying: (rule: Proxying) => void;
  // The chord the person set for one action, or `""` for an action
  // they left alone. The spelling is the keymap's grammar, not this
  // file's: what is kept here is a name and a string.
  readonly chord: (action: string) => string;
  readonly setChord: (action: string, spelled: string) => void;
  // What was typed and not sent, by where it was typed. Not a signal:
  // the box that owns it reads it once when it mounts.
  readonly draft: (at: string) => string;
  readonly setDraft: (at: string, text: string) => void;
}

// A stored word, or the posture this client ships with when the row is
// empty or holds a word this build no longer offers.
function readOne<T extends string>(offered: readonly T[], raw: string | null, fallback: T): T {
  return offered.find((each) => each === raw) ?? fallback;
}

function readBody(raw: string | null): number | null {
  const said = sizingOf(raw ?? "");
  return said.kind === "sized" ? said.px : null;
}

function readStack(raw: string | null): string {
  return raw !== null && STACK_SHAPE.test(raw) ? raw : "";
}

function readLang(raw: string | null, fallback: string): Lang {
  return raw === "en" || raw === "zh" ? raw : langOf(fallback);
}

function readAppearance(rows: Rows): Appearance {
  return {
    lighting: readOne(LIGHTINGS, rows.getItem(ROWS.lighting), "system"),
    sans: readOne(FACES, rows.getItem(ROWS.sans), "geist"),
    mono: readOne(FACES, rows.getItem(ROWS.mono), "geist"),
    sansStack: readStack(rows.getItem(ROWS.sansStack)),
    monoStack: readStack(rows.getItem(ROWS.monoStack)),
    body: readBody(rows.getItem(ROWS.body)),
    density: readOne(DENSITIES, rows.getItem(ROWS.density), "comfortable"),
    chroma: readOne(CHROMAS, rows.getItem(ROWS.chroma), "full"),
    motion: readOne(MOTIONS, rows.getItem(ROWS.motion), "system"),
  };
}

// A size the person has not stated is absent from storage too, so the
// stylesheet's own figure keeps its one home in `theme.css`.
function writeAppearance(rows: Rows, next: Appearance): void {
  rows.setItem(ROWS.lighting, next.lighting);
  rows.setItem(ROWS.sans, next.sans);
  rows.setItem(ROWS.mono, next.mono);
  rows.setItem(ROWS.sansStack, next.sansStack);
  rows.setItem(ROWS.monoStack, next.monoStack);
  if (next.body === null) {
    rows.removeItem(ROWS.body);
  } else {
    rows.setItem(ROWS.body, String(next.body));
  }
  rows.setItem(ROWS.density, next.density);
  rows.setItem(ROWS.chroma, next.chroma);
  rows.setItem(ROWS.motion, next.motion);
}

function readPreferences(rows: Rows, browserLang: string): Preferences {
  return {
    lang: readLang(rows.getItem(ROWS.lang), browserLang),
    welcomed: rows.getItem(ROWS.welcomed) === "yes",
    panel: rows.getItem(ROWS.panel) !== "no",
    appearance: readAppearance(rows),
    proxying: readOne(PROXYINGS, rows.getItem(ROWS.proxying), "except_local"),
  };
}

// The door onto one store. A test hands it a map and its own language
// tag; the page reaches the browser's through `preferences()` below.
export function loadPreferences(rows: Rows, browserLang: string): PreferenceDoor {
  const [held, setHeld] = createSignal<Preferences>(readPreferences(rows, browserLang));
  // Each change writes its own rows and then the record, so a reader
  // that redraws on the signal and a reader that reloads the page see
  // the same thing.
  return {
    held,
    setLang(lang) {
      rows.setItem(ROWS.lang, lang);
      setHeld((before) => ({ ...before, lang }));
    },
    setWelcomed(done) {
      rows.setItem(ROWS.welcomed, done ? "yes" : "no");
      setHeld((before) => ({ ...before, welcomed: done }));
    },
    setPanel(open) {
      rows.setItem(ROWS.panel, open ? "yes" : "no");
      setHeld((before) => ({ ...before, panel: open }));
    },
    setAppearance(next) {
      writeAppearance(rows, next);
      setHeld((before) => ({ ...before, appearance: next }));
    },
    setProxying(rule) {
      rows.setItem(ROWS.proxying, rule);
      setHeld((before) => ({ ...before, proxying: rule }));
    },
    chord: (action) => rows.getItem(ROWS.chord + action) ?? "",
    setChord(action, spelled) {
      if (spelled === "") {
        rows.removeItem(ROWS.chord + action);
      } else {
        rows.setItem(ROWS.chord + action, spelled);
      }
    },
    draft: (at) => rows.getItem(ROWS.draft + at) ?? "",
    setDraft(at, text) {
      if (text === "") {
        rows.removeItem(ROWS.draft + at);
      } else {
        rows.setItem(ROWS.draft + at, text);
      }
    },
  };
}

// The preferences this page runs on. One per document, for the reason
// the keymap is one per document: the shell, the settings screens and
// the face the page is drawn in all have to be looking at the same
// record, and a second copy would let a change go unseen.
let shared: PreferenceDoor | undefined;

export function preferences(): PreferenceDoor {
  shared ??= loadPreferences(
    browserRows(),
    typeof navigator === "undefined" ? "" : navigator.language,
  );
  return shared;
}

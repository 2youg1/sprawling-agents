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
// **Two layers keep these, and the city wins.** The person's own
// `~/.sprawling/config.toml` is the authority (roadmap 3.1) and this
// browser's store is the cache in front of it: the cache draws the
// first paint so no screen flashes the posture it ships with, and
// `adopt` replaces the whole record the moment the city answers. A
// change made before the city has answered is kept in this browser
// alone, and `keeper()` says so on the settings page rather than
// leaving a person to find out when they clear the browser's data.
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

// Who keeps these preferences between one visit and the next.
//
// Two answers, and a person is entitled to both of them: somebody who
// picks a face for the page needs to know whether the choice follows
// them to their next browser or dies with this profile's data.
export type Keeper =
  // This browser and nothing else. Clearing its data loses them, and
  // another browser reaching the same city starts from the postures
  // this client ships with.
  | "browser"
  // The city, in the person's own `~/.sprawling/config.toml`. This
  // browser still caches the record, and the cache never outranks the
  // answer: every answer that arrives replaces it whole.
  | "city";

// The one way to the person's preferences: the record as it stands,
// who is keeping it, the city's answer coming the other way, five
// named changes to it, and the two families that are read by name
// because they have one row each per place and per action.
//
// Named changes rather than one `write`, because each of them becomes
// its own command the day the city keeps these: a caller that handed
// over a whole record would have to be rewritten then, and a caller
// that says which fact it is changing would not. `adopt` is the other
// direction and is therefore whole - an answer states every value at
// once, and a record applied field by field could be half of one
// answer and half of the last.
export interface PreferenceDoor {
  readonly held: Accessor<Preferences>;
  readonly keeper: Accessor<Keeper>;
  // The city's whole record, taken as the one that counts: it becomes
  // what `held` answers, it is mirrored into the cache so the next
  // first paint draws it rather than the shipped postures, and it is
  // the only thing that makes `keeper` say `city`.
  readonly adopt: (stated: Preferences) => void;
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

// The two words a yes-or-no row is written with, spelled here so the
// write and the read cannot spell them differently.
//
// Each of the two rows is compared against one of them, because an
// absent row means different things: `welcomed` is false until
// somebody has walked the welcome, and `panel` is open until somebody
// has closed it.
const YES = "yes";
const NO = "no";

function readPreferences(rows: Rows, browserLang: string): Preferences {
  return {
    lang: readLang(rows.getItem(ROWS.lang), browserLang),
    welcomed: rows.getItem(ROWS.welcomed) === YES,
    panel: rows.getItem(ROWS.panel) !== NO,
    appearance: readAppearance(rows),
    proxying: readOne(PROXYINGS, rows.getItem(ROWS.proxying), "except_local"),
  };
}

// The whole record into the cache, which `readPreferences` reads back.
// The two are inverse, and that is what makes the cache a cache: what
// the city last answered is what the next first paint draws.
function writePreferences(rows: Rows, next: Preferences): void {
  rows.setItem(ROWS.lang, next.lang);
  rows.setItem(ROWS.welcomed, next.welcomed ? YES : NO);
  rows.setItem(ROWS.panel, next.panel ? YES : NO);
  writeAppearance(rows, next.appearance);
  rows.setItem(ROWS.proxying, next.proxying);
}

// The door onto one store. A test hands it a map and its own language
// tag; the page reaches the browser's through `preferences()` below.
export function loadPreferences(rows: Rows, browserLang: string): PreferenceDoor {
  const [held, setHeld] = createSignal<Preferences>(readPreferences(rows, browserLang));
  const [keeper, setKeeper] = createSignal<Keeper>("browser");
  // One write path for every change, the city's answer included: the
  // cache and the signal move together, so a reader that redraws and a
  // reader that reloads the page never see two different records.
  const settle = (next: Preferences): void => {
    writePreferences(rows, next);
    setHeld(next);
  };
  return {
    held,
    keeper,
    adopt(stated) {
      settle(stated);
      setKeeper("city");
    },
    setLang(lang) {
      settle({ ...held(), lang });
    },
    setWelcomed(welcomed) {
      settle({ ...held(), welcomed });
    },
    setPanel(panel) {
      settle({ ...held(), panel });
    },
    setAppearance(appearance) {
      settle({ ...held(), appearance });
    },
    setProxying(proxying) {
      settle({ ...held(), proxying });
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

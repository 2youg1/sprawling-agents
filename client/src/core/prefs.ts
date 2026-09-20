// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The startup cache: what this browser saw last time, so the first
// frame is drawn in the language, at the size and in the light the
// person left, rather than flashing one face and then another.
//
// Every value here is a guess about a fact the city owns. A guess this
// build cannot read is dropped rather than replaced by an invented
// one, because a cache that invents is a second authority: `effort`
// left unset is the person saying nothing, which reaches the provider
// as nothing and lets the provider choose.
//
// **This file is the client's one door to browser storage.** Every row
// the page keeps is named here and read back here, so a key cannot be
// spelled one way where it is written and another way where it is
// read; and a browser that offers no storage is answered once, so
// every reader above degrades the same way.
//
// **Browser storage refuses by throwing, in three places a person can
// reach**: a profile that denies storage throws on the property access
// itself, a private window can grant a store whose quota is zero, and
// a quota can fill while the tab is open. Each throw is turned into a
// value here, so no reader above has to know that keeping a row is an
// operation that can fail, and the first paint cannot die on one.

import { Effect, Either } from "effect";
import { createSignal } from "solid-js";
import type { Accessor } from "solid-js";

import type { Lang } from "./lang";
import { langOf } from "./lang";
import { Effort as EffortSchema } from "../wire";
import type { Effort, Proxying } from "../wire";

// ------------------------------------------------------------- the store

// The little of `Storage` this client needs: a test hands it a map and
// a browser hands it `localStorage`. Narrow on purpose - nothing above
// may enumerate or clear rows it did not write.
export interface Rows {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
  readonly removeItem: (key: string) => void;
}

// What a browser without storage remembers: this session, and no
// longer. A choice still takes effect; it just does not outlive the
// tab.
function memory(): Rows {
  const held = new Map<string, string>();
  return {
    getItem: (key) => held.get(key) ?? null,
    setItem: (key, value) => {
      held.set(key, value);
    },
    removeItem: (key) => {
      held.delete(key);
    },
  };
}

// What a browser did when asked, as a value: the result, or nothing
// when the browser refused. This is the client's second use of Effect
// at run time and it is the same use as the first - a failure that
// would otherwise be thrown is read as data (client-SPEC 4-6).
function attempted<T>(act: () => T): T | null {
  const ran = Effect.runSync(Effect.either(Effect.try(act)));
  return Either.isRight(ran) ? ran.right : null;
}

// The row written and dropped to find out whether this browser keeps
// anything at all. Named like every other row so a store shared with
// another application cannot mistake it for theirs.
const PROBE_KEY = "sprawling.probe";

// The browser's own store with its refusals answered: a row the quota
// will not take is kept for this session instead, so a person typing
// into a box never loses the sentence that filled the quota and no
// page dies on a write.
function guarded(store: Rows, spare: Rows): Rows {
  return {
    getItem: (key) => attempted(() => store.getItem(key)) ?? spare.getItem(key),
    setItem: (key, value) => {
      const took = attempted(() => {
        store.setItem(key, value);
        return true;
      });
      if (took === null) {
        spare.setItem(key, value);
      }
    },
    removeItem: (key) => {
      // Both copies are asked to drop the row and neither answer is
      // needed: a store that refuses a removal is one the probe would
      // not have admitted, and the session copy is dropped regardless.
      attempted(() => {
        store.removeItem(key);
        return true;
      });
      spare.removeItem(key);
    },
  };
}

// Whether this browser gives the client a place to keep rows, decided
// by writing one and dropping it again. A store that answers the probe
// is used; a store that throws on the reach, or takes nothing, is
// stood in for by a map that lasts as long as the tab.
function decided(): Rows {
  const spare = memory();
  const store = attempted<Rows>(() => localStorage);
  if (store === null) {
    return spare;
  }
  const kept = attempted(() => {
    store.setItem(PROBE_KEY, PROBE_KEY);
    store.removeItem(PROBE_KEY);
    return true;
  });
  return kept === null ? spare : guarded(store, spare);
}

// Where this browser's preferences live, and the only reach for that
// global in the whole client. A hardened profile, a document rendered
// before storage is granted, and a test runner all arrive here, and
// what happens to the three of them is decided once.
let reached: Rows | undefined;

export function browserRows(): Rows {
  // One table for every caller, and one probe for the whole session. A
  // fresh one per call would let the shell and a settings panel each
  // write their own preferences into a map the other never reads.
  reached ??= decided();
  return reached;
}

// ------------------------------------------------------------- the rows

const LANG_KEY = "sprawling.lang";
const EFFORT_KEY = "sprawling.effort";
const WELCOMED_KEY = "sprawling.welcomed";
// One unsent message per place a person writes, kept across a reload
// or a page change; the key is the room or the run.
const DRAFT_PREFIX = "sprawling.draft.";
const LIGHTING_KEY = "sprawling.appearance.lighting";
const SANS_KEY = "sprawling.appearance.sans";
const MONO_KEY = "sprawling.appearance.mono";
const SANS_STACK_KEY = "sprawling.appearance.sans_stack";
const MONO_STACK_KEY = "sprawling.appearance.mono_stack";
const BODY_KEY = "sprawling.appearance.body";
const DENSITY_KEY = "sprawling.appearance.density";
const CHROMA_KEY = "sprawling.appearance.chroma";
const MOTION_KEY = "sprawling.appearance.motion";

// Which rule a provider attached from now on starts with. A person
// behind a relay settles it once, on the network screen, instead of on
// every form they open; an endpoint already attached keeps the rule the
// city recorded for it, so this is a starting point and not a setting
// that reaches back.
const PROXYING_KEY = "sprawling.network.proxying";

// Whether the artefact panel beside a conversation is open. A debt with
// a name: it belongs in the person's own TOML layer (roadmap 3.1), and
// this row moves there whole when that layer lands rather than growing
// a second reader here.
const PANEL_KEY = "sprawling.talk.panel";

const PROXYINGS: readonly Proxying[] = ["except_local", "always", "never"];

export function defaultProxying(rows: Rows): Proxying {
  const held = rows.getItem(PROXYING_KEY);
  return PROXYINGS.find((rule) => rule === held) ?? "except_local";
}

export function setDefaultProxying(rows: Rows, rule: Proxying): void {
  rows.setItem(PROXYING_KEY, rule);
}

// ---------------------------------------------------------------- effort

// The ladder the wire accepts, in the wire's own order. The generated
// schema is the one place it is written; a level added there appears
// in every selector without anybody editing a list.
export const EFFORTS: readonly Effort[] = EffortSchema.literals;

// `null` for a value this build cannot read, which is the same as
// never having chosen: the request carries no effort and the provider
// decides. `Effort` has a `"none"` level, and it means the opposite -
// think as little as possible - so absence may not be spelled with it.
function readEffort(raw: string | null): Effort | null {
  return EFFORTS.find((effort) => effort === raw) ?? null;
}

function readLang(raw: string | null, fallback: string): Lang {
  return raw === "en" || raw === "zh" ? raw : langOf(fallback);
}

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

function readBody(raw: string | null): number | null {
  const said = sizingOf(raw ?? "");
  return said.kind === "sized" ? said.px : null;
}

function readStack(raw: string | null): string {
  return raw !== null && STACK_SHAPE.test(raw) ? raw : "";
}

// A stored word, or the posture this client ships with when the row is
// empty or holds a word this build no longer offers.
function readOne<T extends string>(offered: readonly T[], raw: string | null, fallback: T): T {
  return offered.find((each) => each === raw) ?? fallback;
}

// Every appearance choice this browser holds. A row this build cannot
// read is not repaired and not invented: it falls back to the posture
// the client ships with, which `theme.css` already draws.
export function readAppearance(rows: Rows): Appearance {
  return {
    lighting: readOne(LIGHTINGS, rows.getItem(LIGHTING_KEY), "system"),
    sans: readOne(FACES, rows.getItem(SANS_KEY), "geist"),
    mono: readOne(FACES, rows.getItem(MONO_KEY), "geist"),
    sansStack: readStack(rows.getItem(SANS_STACK_KEY)),
    monoStack: readStack(rows.getItem(MONO_STACK_KEY)),
    body: readBody(rows.getItem(BODY_KEY)),
    density: readOne(DENSITIES, rows.getItem(DENSITY_KEY), "comfortable"),
    chroma: readOne(CHROMAS, rows.getItem(CHROMA_KEY), "full"),
    motion: readOne(MOTIONS, rows.getItem(MOTION_KEY), "system"),
  };
}

// A size the person has not stated is absent from storage too, so the
// stylesheet's own figure keeps its one home in `theme.css`.
export function writeAppearance(rows: Rows, next: Appearance): void {
  rows.setItem(LIGHTING_KEY, next.lighting);
  rows.setItem(SANS_KEY, next.sans);
  rows.setItem(MONO_KEY, next.mono);
  rows.setItem(SANS_STACK_KEY, next.sansStack);
  rows.setItem(MONO_STACK_KEY, next.monoStack);
  if (next.body === null) {
    rows.removeItem(BODY_KEY);
  } else {
    rows.setItem(BODY_KEY, String(next.body));
  }
  rows.setItem(DENSITY_KEY, next.density);
  rows.setItem(CHROMA_KEY, next.chroma);
  rows.setItem(MOTION_KEY, next.motion);
}

// ----------------------------------------------------------- the reading

export interface Prefs {
  readonly lang: Accessor<Lang>;
  readonly setLang: (lang: Lang) => void;
  // How hard the model should think, or `null` for a person who has
  // not said: the choice then belongs to the provider.
  readonly effort: Accessor<Effort | null>;
  readonly setEffort: (effort: Effort | null) => void;
  // Whether this browser has walked through the welcome once. The city
  // decides whether setup is *needed*; this only decides whether a
  // person who skipped it is nagged again.
  readonly welcomed: Accessor<boolean>;
  // Whether the artefact panel beside a conversation is open.
  readonly panel: Accessor<boolean>;
  readonly setPanel: (open: boolean) => void;
  readonly setWelcomed: (done: boolean) => void;
  // What was typed and not sent, by where it was typed. Not a signal:
  // the box that owns it reads it once when it mounts.
  readonly draft: (at: string) => string;
  readonly setDraft: (at: string, text: string) => void;
}

export function loadPrefs(rows: Rows, browserLang: string): Prefs {
  const [lang, setLangSignal] = createSignal<Lang>(
    readLang(rows.getItem(LANG_KEY), browserLang),
  );
  const [effort, setEffortSignal] = createSignal<Effort | null>(
    readEffort(rows.getItem(EFFORT_KEY)),
  );
  const [welcomed, setWelcomedSignal] = createSignal<boolean>(
    rows.getItem(WELCOMED_KEY) === "yes",
  );
  const [panel, setPanelSignal] = createSignal<boolean>(
    rows.getItem(PANEL_KEY) !== "no",
  );
  return {
    lang,
    setLang(next) {
      rows.setItem(LANG_KEY, next);
      setLangSignal(next);
    },
    effort,
    setEffort(next) {
      if (next === null) {
        rows.removeItem(EFFORT_KEY);
      } else {
        rows.setItem(EFFORT_KEY, next);
      }
      setEffortSignal(next);
    },
    panel,
    setPanel(open) {
      rows.setItem(PANEL_KEY, open ? "yes" : "no");
      setPanelSignal(open);
    },
    welcomed,
    setWelcomed(done) {
      rows.setItem(WELCOMED_KEY, done ? "yes" : "no");
      setWelcomedSignal(done);
    },
    draft(at) {
      return rows.getItem(DRAFT_PREFIX + at) ?? "";
    },
    setDraft(at, text) {
      if (text === "") {
        rows.removeItem(DRAFT_PREFIX + at);
      } else {
        rows.setItem(DRAFT_PREFIX + at, text);
      }
    },
  };
}

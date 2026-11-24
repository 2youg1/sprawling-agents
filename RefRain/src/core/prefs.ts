// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { derived, get, writable } from "./store";
import type { Readable } from "./store";

import { EDITORS, type Opening } from "./editor";
import type { Lang } from "./lang";
import { browserRows, type Rows } from "./rows";
import {
  Proxying,
  Tier as TierSchema,
  type Chord,
  type PreferencePatch,
  type Tier as WireTier,
} from "../wire";
import { appearanceOnWire, type Keeper } from "./prefs_city";
import { readTheme, type Theme } from "./theme_override";
import {
  CHROMAS,
  DENSITIES,
  FACES,
  FEATURES,
  GLASSES,
  LIGHTINGS,
  MOTIONS,
  READING_FACES,
  WEIGHTS,
  blendOf,
  type Appearance,
} from "./appearance";
import { readBody, readLang, readOne, readStack, writeFigure } from "./prefs_values";
import { readAtLeast, readFamily, readMany, writeNames } from "./prefs_values";
import { LINE_HEIGHT_FLOOR, TRACKING_FLOOR } from "./metrics";
import type { Notifying } from "./notify";
import { SHOWINGS, type Showing } from "./results";
import { readWorkbench, spelledWorkbench, type Workbench } from "./workbench";

const ROWS = {
  lang: "sprawling.lang",
  welcomed: "sprawling.welcomed",
  tier: "sprawling.tier",

  panel: "sprawling.talk.panel",
  lighting: "sprawling.appearance.lighting",
  sans: "sprawling.appearance.sans",
  mono: "sprawling.appearance.mono",
  sansStack: "sprawling.appearance.sans_stack",
  monoStack: "sprawling.appearance.mono_stack",
  reading: "sprawling.appearance.reading",

  fallback1: "sprawling.appearance.fallback1",
  fallback2: "sprawling.appearance.fallback2",

  features: "sprawling.appearance.features",
  body: "sprawling.appearance.body",

  lineHeight: "sprawling.appearance.line_height",
  tracking: "sprawling.appearance.tracking",
  weight: "sprawling.appearance.weight",
  density: "sprawling.appearance.density",
  chroma: "sprawling.appearance.chroma",
  motion: "sprawling.appearance.motion",
  glass: "sprawling.appearance.glass",
  blend: "sprawling.appearance.blend",

  theme: "sprawling.appearance.theme",

  proxying: "sprawling.network.proxying",

  notifying: "sprawling.notify",

  showing: "sprawling.showing",

  draft: "sprawling.draft.",

  chord: "sprawling.key.",
  editor: "sprawling.editor",
  cityFolder: "sprawling.editor.folder",

  workbench: "sprawling.workbench",

  paletteFound: "sprawling.palette.found",
} as const;

export type Tier = WireTier;
export const TIERS: readonly Tier[] = TierSchema.literals;

export const SESSION_TIERS: readonly ("zen" | "panorama")[] = ["zen", "panorama"];

export const PROXYING_RULES: readonly Proxying[] = Proxying.members.map((rule) => rule.literal);

export const NOTIFYINGS: readonly Notifying[] = ["off", "on"];

export interface Preferences {
  readonly lang: Lang;

  readonly welcomed: boolean;
  readonly panel: boolean;
  readonly tier: Tier;
  readonly appearance: Appearance;
  readonly theme: Theme;
  readonly proxying: Proxying;
  readonly notifying: Notifying;
  readonly showing: Showing;
}

export interface PreferenceDoor {
  readonly held: Readable<Preferences>;
  readonly keeper: Readable<Keeper>;
  readonly themeUnkept: Readable<boolean>;

  readonly adopt: (stated: Preferences, chords: readonly Chord[]) => void;

  readonly tell: (send: (patch: PreferencePatch) => void) => void;
  readonly setLang: (lang: Lang) => void;
  readonly setWelcomed: (done: boolean) => void;
  readonly setPanel: (open: boolean) => void;

  readonly setTier: (tier: Tier) => void;
  readonly setAppearance: (next: Appearance) => void;
  readonly setTheme: (next: Theme) => void;
  readonly setProxying: (rule: Proxying) => void;
  readonly setNotifying: (switched: Notifying) => void;
  readonly setShowing: (showing: Showing) => void;

  readonly chord: (action: string) => string;
  readonly setChord: (action: string, spelled: string) => void;

  readonly draft: (at: string) => string;
  readonly setDraft: (at: string, text: string) => void;

  readonly draftUnkept: (at: string) => Readable<boolean>;

  readonly editor: () => Pick<Opening, "editor" | "folder">;

  readonly editorHeld: Readable<Pick<Opening, "editor" | "folder">>;
  readonly setEditor: (next: Pick<Opening, "editor" | "folder">) => void;
  readonly workbench: Readable<Workbench>;
  readonly setWorkbench: (next: Workbench) => void;

  readonly paletteFound: Readable<boolean>;
  readonly findPalette: () => void;
}

function readAppearance(rows: Rows): Appearance {
  return {
    lighting: readOne(LIGHTINGS, rows.getItem(ROWS.lighting), "system"),
    sans: readOne(FACES, rows.getItem(ROWS.sans), "paper"),
    mono: readOne(FACES, rows.getItem(ROWS.mono), "paper"),
    sansStack: readStack(rows.getItem(ROWS.sansStack)),
    monoStack: readStack(rows.getItem(ROWS.monoStack)),
    reading: readOne(READING_FACES, rows.getItem(ROWS.reading), "interface"),

    fallbacks: [
      readFamily(rows.getItem(ROWS.fallback1)),
      readFamily(rows.getItem(ROWS.fallback2)),
    ].filter((name) => name !== ""),
    features: readMany(FEATURES, rows.getItem(ROWS.features)),
    body: readBody(rows.getItem(ROWS.body)),
    lineHeight: readAtLeast(rows.getItem(ROWS.lineHeight), LINE_HEIGHT_FLOOR),
    tracking: readAtLeast(rows.getItem(ROWS.tracking), TRACKING_FLOOR),
    weight: readOne(WEIGHTS, rows.getItem(ROWS.weight), "regular"),
    density: readOne(DENSITIES, rows.getItem(ROWS.density), "comfortable"),
    chroma: readOne(CHROMAS, rows.getItem(ROWS.chroma), "full"),
    motion: readOne(MOTIONS, rows.getItem(ROWS.motion), "system"),
    glass: readOne(GLASSES, rows.getItem(ROWS.glass), "on"),
    blend: blendOf(rows.getItem(ROWS.blend) ?? ""),
  };
}

function writeAppearance(rows: Rows, next: Appearance): void {
  rows.setItem(ROWS.lighting, next.lighting);
  rows.setItem(ROWS.sans, next.sans);
  rows.setItem(ROWS.mono, next.mono);
  rows.setItem(ROWS.sansStack, next.sansStack);
  rows.setItem(ROWS.monoStack, next.monoStack);
  rows.setItem(ROWS.reading, next.reading);

  const [first, second] = next.fallbacks;
  if (first === undefined) rows.removeItem(ROWS.fallback1);
  else rows.setItem(ROWS.fallback1, first);
  if (second === undefined) rows.removeItem(ROWS.fallback2);
  else rows.setItem(ROWS.fallback2, second);
  writeNames(rows, ROWS.features, next.features);
  writeFigure(rows, ROWS.body, next.body);
  writeFigure(rows, ROWS.lineHeight, next.lineHeight);
  writeFigure(rows, ROWS.tracking, next.tracking);
  rows.setItem(ROWS.weight, next.weight);
  rows.setItem(ROWS.density, next.density);
  rows.setItem(ROWS.chroma, next.chroma);
  rows.setItem(ROWS.motion, next.motion);
  rows.setItem(ROWS.glass, next.glass);
  writeFigure(rows, ROWS.blend, next.blend);
}

const YES = "yes";
const NO = "no";

function readPreferences(rows: Rows, browserLang: string): Preferences {
  return {
    lang: readLang(rows.getItem(ROWS.lang), browserLang),
    welcomed: rows.getItem(ROWS.welcomed) === YES,
    panel: rows.getItem(ROWS.panel) !== NO,
    tier: readOne(SESSION_TIERS, rows.getItem(ROWS.tier), "zen"),
    appearance: readAppearance(rows),
    theme: readTheme(rows.getItem(ROWS.theme)),
    proxying: readOne(PROXYING_RULES, rows.getItem(ROWS.proxying), "except_local"),
    notifying: readOne(NOTIFYINGS, rows.getItem(ROWS.notifying), "off"),
    showing: readOne(SHOWINGS, rows.getItem(ROWS.showing), "whole"),
  };
}

function writePreferences(rows: Rows, next: Preferences): void {
  rows.setItem(ROWS.lang, next.lang);
  rows.setItem(ROWS.welcomed, next.welcomed ? YES : NO);
  rows.setItem(ROWS.panel, next.panel ? YES : NO);
  writeAppearance(rows, next.appearance);
  rows.setItem(ROWS.theme, JSON.stringify(next.theme));
  rows.setItem(ROWS.proxying, next.proxying);
  rows.setItem(ROWS.notifying, next.notifying);
  rows.setItem(ROWS.showing, next.showing);
}

export function loadPreferences(rows: Rows, browserLang: string): PreferenceDoor {
  const held = writable<Preferences>(readPreferences(rows, browserLang));
  const keeper = writable<Keeper>("browser");
  const workbench = writable<Workbench>(readWorkbench(rows.getItem(ROWS.workbench)));
  const paletteFound = writable<boolean>(rows.getItem(ROWS.paletteFound) === YES);
  const editorHeld = writable<Pick<Opening, "editor" | "folder">>({
    editor: readOne(EDITORS, rows.getItem(ROWS.editor), "none"),
    folder: rows.getItem(ROWS.cityFolder) ?? "",
  });

  const settle = (next: Preferences): void => {
    writePreferences(rows, next);
    held.set(next);
  };
  let told: (patch: PreferencePatch) => void = () => undefined;
  const writeChord = (action: string, spelled: string): void => {
    if (spelled === "") {
      rows.removeItem(ROWS.chord + action);
    } else {
      rows.setItem(ROWS.chord + action, spelled);
    }
  };
  return {
    held,
    keeper,
    themeUnkept: derived(rows.unkept, (unkept) => unkept.has(ROWS.theme)),
    adopt(stated, chords) {
      settle(stated);
      for (const each of chords) writeChord(each.action, each.spelled);
      keeper.set("city");
    },
    tell(send) {
      told = send;
    },
    setLang(lang) {
      settle({ ...get(held), lang });
      told({ lang });
    },
    setWelcomed(welcomed) {
      settle({ ...get(held), welcomed });
      told({ welcomed });
    },
    setPanel(panel) {
      settle({ ...get(held), panel });
      told({ panel });
    },
    setTier(tier) {
      const density = tier === "blend" ? "zen" : tier;
      rows.setItem(ROWS.tier, density);
      held.set({ ...get(held), tier: density });
    },
    setAppearance(appearance) {
      settle({ ...get(held), appearance });
      told({ appearance: appearanceOnWire(appearance) });
    },
    setTheme(theme) {
      settle({ ...get(held), theme });
      told({ theme: { tokens: { ...theme.tokens }, css: theme.css } });
    },
    setProxying(proxying) {
      settle({ ...get(held), proxying });
      told({ proxying });
    },
    setNotifying(notifying) {
      settle({ ...get(held), notifying });
    },
    setShowing(showing) {
      settle({ ...get(held), showing });
    },
    chord: (action) => rows.getItem(ROWS.chord + action) ?? "",
    setChord(action, spelled) {
      writeChord(action, spelled);
      told({ chord: { action, spelled } });
    },
    draft: (at) => rows.getItem(ROWS.draft + at) ?? "",
    setDraft(at, text) {
      if (text === "") rows.removeItem(ROWS.draft + at);
      else rows.setItem(ROWS.draft + at, text);
    },
    draftUnkept: (at) => derived(rows.unkept, (names) => names.has(ROWS.draft + at)),
    editor: () => get(editorHeld),
    editorHeld,
    setEditor(next) {
      rows.setItem(ROWS.editor, next.editor);
      rows.setItem(ROWS.cityFolder, next.folder);
      editorHeld.set(next);
    },
    workbench,
    setWorkbench(next) {
      rows.setItem(ROWS.workbench, spelledWorkbench(next));
      workbench.set(next);
    },
    paletteFound,
    findPalette() {
      if (get(paletteFound)) return;
      rows.setItem(ROWS.paletteFound, YES);
      paletteFound.set(true);
    },
  };
}

let shared: PreferenceDoor | undefined;

export function preferences(): PreferenceDoor {
  shared ??= loadPreferences(
    browserRows(),
    typeof navigator === "undefined" ? "" : navigator.language,
  );
  return shared;
}
